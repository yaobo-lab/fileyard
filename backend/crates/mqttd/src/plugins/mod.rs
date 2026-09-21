#![allow(non_upper_case_globals)]
//插件目录
pub mod auth;
#[cfg(feature = "message-storage")]
pub mod message;
pub mod restapi;
pub mod retainer;
pub mod session;
pub mod transfer;

#[cfg(feature = "webhook")]
pub mod webhook;
use anyhow::Result;
use anyhow::anyhow;
use base64::prelude::{BASE64_STANDARD, Engine};
use chrono::TimeZone;
use rmqtt::types::Publish;
use rmqtt::{
    codec::v5::PublishProperties,
    context::ServerContext,
    session::SessionState,
    types::{ClientId, CodecPublish, From, Id, QoS, TimestampMillis, TopicName, UserName},
    utils::timestamp_millis,
};
use serde::{Deserialize, Serialize};
use std::time::Duration;

fn to_timestamp_millis(t: TimestampMillis) -> i64 {
    if t <= 0 {
        0
    } else {
        if let chrono::LocalResult::Single(t) = chrono::Local.timestamp_millis_opt(t) {
            t.timestamp_millis()
        } else {
            0
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct PublishParams {
    //For topic and topics, with at least one of them specified
    pub topic: Option<TopicName>,
    //Multiple topics separated by ,. This field is used to publish messages to multiple topics at the same time
    pub topics: Option<TopicName>,
    //Client identifier. Default:　system
    #[serde(default = "PublishParams::clientid_default")]
    pub clientid: ClientId,
    //Message body
    pub payload: String,
    //The encoding used in the message body. Currently only plain and base64 are supported. Default:　plain
    #[serde(default = "PublishParams::encoding_default")]
    pub encoding: String,
    //QoS level, Default: 0
    #[serde(default = "PublishParams::qos_default")]
    pub qos: u8,
    //Whether it is a retained message, Default: false
    #[serde(default = "PublishParams::retain_default")]
    pub retain: bool,
    //Publish Properties
    pub properties: Option<PublishProperties>,
}

impl PublishParams {
    fn clientid_default() -> ClientId {
        "system".into()
    }

    fn encoding_default() -> String {
        "base64".into()
    }

    fn qos_default() -> u8 {
        0
    }

    fn retain_default() -> bool {
        false
    }
}

//发布消息
async fn re_publish_message(
    params: PublishParams,
    scx: &ServerContext,
    expiry_interval: Option<Duration>,
) -> Result<()> {
    let mut topics = if let Some(topics) = params.topics {
        topics
            .split(',')
            .collect::<Vec<_>>()
            .iter()
            .map(|t| TopicName::from(t.trim()))
            .collect()
    } else {
        Vec::new()
    };
    if let Some(topic) = params.topic {
        topics.push(topic);
    }
    if topics.is_empty() {
        return Err(anyhow!("topics or topic is empty"));
    }
    let qos = QoS::try_from(params.qos).map_err(|e| anyhow::Error::msg(e.to_string()))?;
    let encoding = params.encoding.to_ascii_lowercase();
    // let payload = if encoding == "plain" {
    //     bytes::Bytes::from(params.payload)
    // } else if encoding == "base64" {
    //     bytes::Bytes::from(
    //         BASE64_STANDARD
    //             .decode(params.payload)
    //             .map_err(anyhow::Error::new)?,
    //     )
    // } else {
    //     return Err(anyhow!(
    //         "encoding error, currently only plain and base64 are supported"
    //     ));
    // };

    let payload = if encoding == "base64" {
        bytes::Bytes::from(
            BASE64_STANDARD
                .decode(params.payload)
                .map_err(anyhow::Error::new)?,
        )
    } else {
        bytes::Bytes::from(params.payload)
    };

    let from = From::from_system(Id::new(
        scx.node.id(),
        0,
        None,
        None,
        ClientId::from_static("admin"),
        Some(UserName::from("admin")),
    ));

    let p = CodecPublish {
        dup: false,
        retain: params.retain,
        qos,
        packet_id: None,
        topic: "".into(),
        payload,
        properties: Some(PublishProperties::default()),
    };

    let message_expiry_interval = params
        .properties
        .as_ref()
        .and_then(|props| {
            props
                .message_expiry_interval
                .map(|interval| Some(Duration::from_secs(interval.get() as u64)))
        })
        .unwrap_or(expiry_interval);

    let create_time = timestamp_millis();
    let storage_available = scx.extends.message_mgr().await.enable();

    let mut futs = Vec::new();
    for topic in topics {
        let from = from.clone();
        let mut p1 = p.clone();
        p1.topic = topic;
        let p1 = <CodecPublish as Into<Publish>>::into(p1).create_time(create_time);
        let fut = async move {
            if let Err(e) =
                SessionState::forwards(scx, from, p1, storage_available, message_expiry_interval)
                    .await
            {
                log::warn!("{e:?}");
            }
        };
        futs.push(fut);
    }
    let _ = futures::future::join_all(futs).await;
    Ok(())
}
