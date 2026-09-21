use super::config::PluginConfig;
use super::rule::Url;
use anyhow::anyhow;
use backoff::{ExponentialBackoff, future::retry};
use bytestring::ByteString;
use rmqtt::{
    Result, hook,
    types::{DashMap, Topic, TopicFilter},
    utils::Counter,
};
use std::path::Path;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use tokio::{
    fs::{File, OpenOptions},
    io::AsyncWriteExt,
    sync::RwLock,
};

#[allow(clippy::too_many_arguments)]
pub(super) async fn handle_msg(
    httpc: &reqwest::Client,
    cfg: Arc<RwLock<PluginConfig>>,
    writers: HookWriters,
    backoff_strategy: Arc<ExponentialBackoff>,
    typ: hook::Type,
    topic: Option<TopicFilter>,
    body: serde_json::Value,
    fails: &Counter,
) -> Result<()> {
    let topic = if let Some(topic) = topic {
        Some(Topic::from_str(&topic)?)
    } else {
        None
    };
    let hook_writes = {
        let cfg = cfg.read().await;
        if let Some(rules) = cfg.rules.get(&typ) {
            //get action and urls
            let action_urls = rules.iter().filter_map(|r| {
                let is_allowed = if let Some(topic) = &topic {
                    if let Some((rule_topics, _)) = &r.topics {
                        rule_topics.is_match(topic)
                    } else {
                        true
                    }
                } else {
                    true
                };

                if is_allowed {
                    let urls = if r.urls.is_empty() {
                        cfg.urls()
                    } else {
                        &r.urls
                    };
                    if urls.is_empty() {
                        None
                    } else {
                        Some((&r.action, urls))
                    }
                } else {
                    None
                }
            });

            //build hook log write futures
            let mut hook_writes = Vec::new();
            for (action, urls) in action_urls {
                let mut new_body = body.clone();
                if let Some(obj) = new_body.as_object_mut() {
                    obj.insert("action".into(), serde_json::Value::String(action.clone()));
                }
                if urls.len() == 1 {
                    log::debug!("action: {}, url: {:?}", action, urls[0]);
                    hook_writes.push(write(
                        httpc,
                        writers.clone(),
                        backoff_strategy.clone(),
                        urls[0].clone(),
                        Arc::new(new_body),
                        cfg.http_timeout,
                        fails,
                    ));
                } else {
                    let new_body = Arc::new(new_body);
                    for url in urls {
                        log::debug!("action: {action}, url: {url:?}");
                        hook_writes.push(write(
                            httpc,
                            writers.clone(),
                            backoff_strategy.clone(),
                            url.clone(),
                            new_body.clone(),
                            cfg.http_timeout,
                            fails,
                        ));
                    }
                }
            }
            Some(hook_writes)
        } else {
            None
        }
    };

    //send hook_writes
    if let Some(mut hook_writes) = hook_writes {
        let c = hook_writes.len();
        match c {
            0 => {}
            1 => {
                hook_writes.remove(0).await;
            }
            _ => {
                let _ = futures::future::join_all(hook_writes).await;
            }
        }
    }

    Ok(())
}

#[inline]
async fn write(
    httpc: &reqwest::Client,
    writers: HookWriters,
    backoff_strategy: Arc<ExponentialBackoff>,
    url: Url,
    body: Arc<serde_json::Value>,
    timeout: Duration,
    fails: &Counter,
) {
    if url.is_file() {
        //is file
        let data = match serde_json::to_vec(body.as_ref()) {
            Ok(data) => data,
            Err(e) => {
                log::warn!("write hook message failure, {e:?}");
                return;
            }
        };
        let writer = writers
            .entry(url.loc.clone())
            .or_insert_with(|| Arc::new(RwLock::new(HookWriter::new(url.loc))))
            .value()
            .clone();
        let mut writer = writer.write().await;
        log::debug!("writer.log start ... ");
        //time::sleep(time::Duration::from_secs(2)).await;
        if let Err(e) = writer.log(data.as_slice()).await {
            fails.current_inc();
            log::warn!(
                "write hook message failure, file: {:?}, {:?}",
                writer.file_name,
                e
            );
        }
        log::debug!("writer.log end ... ");
    } else {
        //is http
        http_request(httpc, backoff_strategy, url, body, timeout, fails).await;
    }
}

//************ http 处理逻辑 ********************* */
pub(super) fn new_http_client() -> Result<reqwest::Client> {
    reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(8))
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|e| anyhow!(e))
}

pub(super) async fn http_request(
    httpc: &reqwest::Client,
    backoff_strategy: Arc<ExponentialBackoff>,
    url: Url,
    body: Arc<serde_json::Value>,
    timeout: Duration,
    fails: &Counter,
) {
    if let Err(e) = retry(backoff_strategy.as_ref().clone(), || async {
        Ok(_http_request(httpc, &url.loc, body.clone(), timeout).await?)
    })
    .await
    {
        fails.current_inc();
        log::warn!("send web hook message failure, {e:?}");
    }
}

async fn _http_request(
    httpc: &reqwest::Client,
    url: &str,
    body: Arc<serde_json::Value>,
    timeout: Duration,
) -> Result<()> {
    log::debug!("http_request, timeout: {timeout:?}, url: {url}, body: {body}");

    let resp = httpc
        .request(reqwest::Method::POST, url)
        .timeout(timeout)
        .json(body.as_ref())
        .send()
        .await
        .map_err(|e| anyhow!(e))?;

    if resp.status().is_success() {
        Ok(())
    } else {
        Err(anyhow!(format!(
            "response status is not OK, url:{:?}, response:{:?}",
            url, resp
        )))
    }
}

//************ log 处理逻辑 ********************* */
pub(super) type HookWriters = Arc<DashMap<ByteString, Arc<RwLock<HookWriter>>>>;

pub(super) struct HookWriter {
    pub(super) file_name: String,
    file: Option<File>,
}

impl HookWriter {
    pub(super) fn new(file: ByteString) -> Self {
        Self {
            file_name: file.to_string(),
            file: None,
        }
    }

    #[inline]
    pub(super) async fn log(
        &mut self,
        msg: &[u8],
    ) -> std::result::Result<(), Box<dyn std::error::Error>> {
        if let Some(file) = self.file.as_mut() {
            file.write_all(msg).await?;
            file.write_all(b"\n").await?;
        } else {
            Self::create_dirs(Path::new(&self.file_name)).await?;
            let mut file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&self.file_name)
                .await?;
            file.write_all(msg).await?;
            file.write_all(b"\n").await?;
            self.file.replace(file);
        }
        Ok(())
    }

    #[inline]
    pub(super) async fn create_dirs(path: &Path) -> std::result::Result<(), std::io::Error> {
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                tokio::fs::create_dir_all(parent).await?;
            }
        }
        Ok(())
    }
}
