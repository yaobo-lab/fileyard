use super::super::PublishParams;
use super::log_prefix;
use anyhow::anyhow;
use salvo::conn::tcp::TcpAcceptor;
use salvo::http::header::{HeaderValue, CONTENT_TYPE};
use salvo::http::mime;
use salvo::prelude::*;
use serde_json::{self, json};
use std::convert::From as _;
use std::io::ErrorKind;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::sync::oneshot;

use rmqtt::{
    context::ServerContext,
    node::NodeStatus,
    stats::Stats,
    types::{ClientId, HashMap, Id, SubsSearchParams},
    Result,
};

use super::types::{ClientSearchParams, ClientSearchResult, SubscribeParams, UnsubscribeParams};
use super::{clients, plugin, subs, PluginConfigType};

struct BearerValidator {
    token: String,
}
impl BearerValidator {
    pub fn new(token: &str) -> Self {
        Self {
            token: format!("Bearer {token}"),
        }
    }
}

#[async_trait]
impl Handler for BearerValidator {
    async fn handle(
        &self,
        req: &mut Request,
        depot: &mut Depot,
        res: &mut Response,
        ctrl: &mut FlowCtrl,
    ) {
        if req
            .headers()
            .get("authorization")
            .is_some_and(|token| token == &self.token)
        {
            ctrl.call_next(req, depot, res).await;
        } else {
            res.status_code(StatusCode::UNAUTHORIZED);
            ctrl.skip_rest()
        }
    }
}

fn route(scx: ServerContext, cfg: PluginConfigType, token: Option<String>) -> Router {
    route_at(scx, cfg, token, "api/v1")
}

pub(super) fn route_at(
    scx: ServerContext,
    cfg: PluginConfigType,
    token: Option<String>,
    path: &str,
) -> Router {
    let mut router = Router::with_path(path)
        .hoop(affix_state::inject((scx, cfg)))
        .hoop(api_logger);
    if let Some(token) = token {
        router = router.hoop(BearerValidator::new(&token));
    }
    router
        .get(list_apis)
        .push(
            Router::with_path("brokers")
                .get(get_brokers)
                .push(Router::with_path("{id}").get(get_brokers)),
        )
        .push(
            Router::with_path("nodes")
                .get(get_nodes)
                .push(Router::with_path("{id}").get(get_nodes)),
        )
        .push(
            Router::with_path("health/check")
                .get(check_health)
                .push(Router::with_path("{id}").get(check_health)),
        )
        .push(
            Router::with_path("clients")
                .push(
                    Router::with_path("offlines")
                        .get(search_offlines)
                        .delete(kick_offlines),
                )
                .get(search_clients)
                .push(
                    Router::with_path("{clientid}")
                        .get(get_client)
                        .delete(kick_client)
                        .push(Router::with_path("online").get(check_online)),
                ),
        )
        .push(
            Router::with_path("subscriptions")
                .get(query_subscriptions)
                .push(Router::with_path("{clientid}").get(get_client_subscriptions)),
        )
        .push(
            Router::with_path("routes")
                .get(get_routes)
                .push(Router::with_path("{topic}").get(get_route)),
        )
        .push(
            (if path == "api/mqtt" {
                Router::new()
            } else {
                Router::with_path("mqtt")
            })
            .push(Router::with_path("publish").post(publish))
            .push(Router::with_path("subscribe").post(subscribe))
            .push(Router::with_path("unsubscribe").post(unsubscribe)),
        )
        .push(
            Router::with_path("plugins")
                .get(all_plugins)
                .push(Router::with_path("{node}").get(node_plugins))
                .push(Router::with_path("{node}/{plugin}").get(node_plugin_info))
                .push(Router::with_path("{node}/{plugin}/config").get(node_plugin_config))
                .push(
                    Router::with_path("{node}/{plugin}/config/reload")
                        .put(node_plugin_config_reload),
                )
                .push(Router::with_path("{node}/{plugin}/load").put(node_plugin_load))
                .push(Router::with_path("{node}/{plugin}/unload").put(node_plugin_unload)),
        )
        .push(
            Router::with_path("stats")
                .get(get_stats)
                .push(Router::with_path("sum").get(get_stats_sum))
                .push(Router::with_path("{id}").get(get_stats)),
        )
}

pub(super) async fn listen_and_serve(
    scx: ServerContext,
    laddr: SocketAddr,
    cfg: PluginConfigType,
    rx: oneshot::Receiver<()>,
) -> Result<()> {
    let (reuseaddr, reuseport, http_bearer_token) = {
        let cfg = cfg.read().await;
        (
            cfg.http_reuseaddr,
            cfg.http_reuseport,
            cfg.http_bearer_token.clone(),
        )
    };

    let listen = tokio::net::TcpListener::from_std(bind(laddr, 128, reuseaddr, reuseport)?)?;
    let acceptor = TcpAcceptor::try_from(listen)?;
    let server = Server::new(acceptor);
    let handler = server.handle();
    tokio::task::spawn(async move {
        rx.await.ok();
        handler.stop_graceful(None);
    });
    server.try_serve(route(scx, cfg, http_bearer_token)).await?;
    Ok(())
}

#[inline]
fn bind(
    laddr: std::net::SocketAddr,
    backlog: i32,
    _reuseaddr: bool,
    _reuseport: bool,
) -> Result<std::net::TcpListener> {
    use socket2::{Domain, SockAddr, Socket, Type};
    let builder = Socket::new(Domain::for_address(laddr), Type::STREAM, None)?;
    builder.set_nonblocking(true)?;
    #[cfg(unix)]
    builder.set_reuse_address(_reuseaddr)?;
    #[cfg(unix)]
    builder.set_reuse_port(_reuseport)?;
    builder.bind(&SockAddr::from(laddr))?;
    builder.listen(backlog)?;
    Ok(std::net::TcpListener::from(builder))
}

#[handler]
async fn list_apis(req: &mut Request, res: &mut Response) {
    let mut data = serde_json::json!([
        {
            "name": "get_brokers",
            "method": "GET",
            "path": "/api/v1/brokers/{node}",
            "descr": "Return the basic information of all nodes in the cluster"
        },
        {
            "name": "get_nodes",
            "method": "GET",
            "path": "/api/v1/nodes/{node}",
            "descr": "Returns the status of the node"
        },
        {
            "name": "check_health",
            "method": "GET",
            "path": "/api/v1/health/check/{node}",
            "descr": "Node health check"
        },
        {
            "name": "search_clients",
            "method": "GET",
            "path": "/api/v1/clients/",
            "descr": "Search clients information from the cluster"
        },
        {
            "name": "get_client",
            "method": "GET",
            "path": "/api/v1/clients/{clientid}",
            "descr": "Get client information from the cluster"
        },
        {
            "name": "kick_client",
            "method": "DELETE",
            "path": "/api/v1/clients/{clientid}",
            "descr": "Kick client from the cluster"
        },
        {
            "name": "check_online",
            "method": "GET",
            "path": "/api/v1/clients/{clientid}/online",
            "descr": "Check a client whether online from the cluster"
        },
        {
            "name": "search_offlines",
            "method": "GET",
            "path": "/api/v1/clients/offlines",
            "descr": "Search offlines clients information from the cluster"
        },
        {
            "name": "kick_offlines",
            "method": "DELETE",
            "path": "/api/v1/clients/offlines",
            "descr": "Kick offlines clients from the cluster"
        },
        {
            "name": "query_subscriptions",
            "method": "GET",
            "path": "/api/v1/subscriptions",
            "descr": "Query subscriptions information from the cluster"
        },
        {
            "name": "get_client_subscriptions",
            "method": "GET",
            "path": "/api/v1/subscriptions/{clientid}",
            "descr": "Get subscriptions information for the client from the cluster"
        },

        {
            "name": "get_routes",
            "method": "GET",
            "path": "/api/v1/routes",
            "descr": "Return all routing information from the cluster"
        },
        {
            "name": "get_route",
            "method": "GET",
            "path": "/api/v1/routes/{topic}",
            "descr": "Get routing information from the cluster"
        },

        {
            "name": "publish",
            "method": "POST",
            "path": "/api/v1/mqtt/publish",
            "descr": "Publish MQTT message"
        },
        {
            "name": "subscribe",
            "method": "POST",
            "path": "/api/v1/mqtt/subscribe",
            "descr": "Subscribe to MQTT topic"
        },
        {
            "name": "unsubscribe",
            "method": "POST",
            "path": "/api/v1/mqtt/unsubscribe",
            "descr": "Unsubscribe"
        },

        {
            "name": "all_plugins",
            "method": "GET",
            "path": "/api/v1/plugins/",
            "descr": "Returns information of all plugins in the cluster"
        },
        {
            "name": "node_plugins",
            "method": "GET",
            "path": "/api/v1/plugins/{node}",
            "descr": "Similar with GET /api/v1/plugins, return the plugin information under the specified node"
        },
        {
            "name": "node_plugin_info",
            "method": "GET",
            "path": "/api/v1/plugins/{node}/{plugin}",
            "descr": "Get a plugin info"
        },
        {
            "name": "node_plugin_config",
            "method": "GET",
            "path": "/api/v1/plugins/{node}/{plugin}/config",
            "descr": "Get a plugin config"
        },
        {
            "name": "node_plugin_config_reload",
            "method": "PUT",
            "path": "/api/v1/plugins/{node}/{plugin}/config/reload",
            "descr": "Reload a plugin config"
        },
        {
            "name": "node_plugin_load",
            "method": "PUT",
            "path": "/api/v1/plugins/{node}/{plugin}/load",
            "descr": "Load the specified plugin under the specified node."
        },
        {
            "name": "node_plugin_unload",
            "method": "PUT",
            "path": "/api/v1/plugins/{node}/{plugin}/unload",
            "descr": "Unload the specified plugin under the specified node."
        },

        {
            "name": "get_stats",
            "method": "GET",
            "path": "/api/v1/stats/{node}",
            "descr": "Returns all statistics information from the cluster"
        },
        {
            "name": "get_stats_sum",
            "method": "GET",
            "path": "/api/v1/stats/sum",
            "descr": "Summarize all statistics information from the cluster"
        },
    ]);
    let base = req.uri().path().trim_end_matches('/');
    if let Some(apis) = data.as_array_mut() {
        for api in apis {
            if let Some(path) = api["path"].as_str() {
                let path = if base == "/api/mqtt" {
                    path.replacen("/api/v1/mqtt/", "/api/v1/", 1)
                } else {
                    path.to_owned()
                };
                api["path"] = json!(path.replacen("/api/v1", base, 1));
            }
        }
    }
    res.render(Json(data));
}

fn get_scx_cfg(
    depot: &mut Depot,
) -> std::result::Result<&(ServerContext, PluginConfigType), salvo::Error> {
    let scx_cfg = depot
        .obtain::<(ServerContext, PluginConfigType)>()
        .map_err(|e| match e {
            None => salvo::Error::Io(std::io::Error::new(ErrorKind::NotFound, anyhow!("None"))),
            Some(e) => salvo::Error::Io(std::io::Error::new(ErrorKind::NotFound, format!("{e:?}"))),
        })?;
    Ok(scx_cfg)
}

#[handler]
async fn api_logger(req: &mut Request, depot: &mut Depot) -> std::result::Result<(), salvo::Error> {
    let (_, cfg) = get_scx_cfg(depot)?;
    if !cfg.read().await.http_request_log {
        return Ok(());
    }
    let log_data = format!(
        "Request {}, {:?}, {}, {}",
        req.remote_addr(),
        req.version(),
        req.method(),
        req.uri()
    );
    let txt_body = if let Some(m) = req.content_type() {
        if let mime::PLAIN | mime::JSON | mime::TEXT = m.subtype() {
            if let Ok(body) = req.payload().await {
                Some(String::from_utf8_lossy(body))
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    };
    if let Some(txt_body) = txt_body {
        log::info!("{log_prefix} {log_data}, body: {txt_body}");
    } else {
        log::info!("{log_prefix} {log_data}");
    }
    Ok(())
}

#[handler]
async fn get_brokers(
    depot: &mut Depot,
    res: &mut Response,
) -> std::result::Result<(), salvo::Error> {
    let (scx, _) = get_scx_cfg(depot)?;
    let broker_info = scx.node.broker_info(scx).await.to_json();
    res.render(Json(broker_info));
    Ok(())
}

#[handler]
async fn get_nodes(depot: &mut Depot, res: &mut Response) -> std::result::Result<(), salvo::Error> {
    let (scx, _) = get_scx_cfg(depot)?;
    let node_info = scx.node.node_info(scx).await;
    res.render(Json(node_info.to_json()));
    Ok(())
}

#[handler]
async fn check_health(
    depot: &mut Depot,
    res: &mut Response,
) -> std::result::Result<(), salvo::Error> {
    let (scx, _) = get_scx_cfg(depot)?;
    let health_status = scx.extends.shared().await.health_status().await;
    if let Err(e) = health_status {
        res.render(StatusError::service_unavailable().detail(e.to_string()));
        return Ok(());
    }
    let health_status = health_status.unwrap();
    if health_status.is_running() {
        res.render(Json(health_status.to_json()))
    } else {
        res.status_code(StatusCode::SERVICE_UNAVAILABLE);
        res.render(Json(health_status.to_json()))
    }
    Ok(())
}

#[handler]
async fn get_client(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> std::result::Result<(), salvo::Error> {
    let (scx, _) = get_scx_cfg(depot)?;
    let clientid = req.param::<String>("clientid");
    if let Some(clientid) = clientid {
        match _get_client(scx, &clientid).await {
            Ok(Some(reply)) => res.render(Json(reply)),
            Ok(None) => {
                res.status_code(StatusCode::NOT_FOUND);
            }
            Err(e) => res.render(StatusError::service_unavailable().detail(e.to_string())),
        }
    } else {
        res.render(StatusError::bad_request())
    }
    Ok(())
}

async fn _get_client(scx: &ServerContext, clientid: &str) -> Result<Option<serde_json::Value>> {
    let reply = clients::get(scx, clientid).await;
    if let Some(reply) = reply {
        return Ok(Some(reply.to_json()));
    }
    Ok(None)
}

#[handler]
async fn search_clients(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> std::result::Result<(), salvo::Error> {
    let (scx, cfg) = get_scx_cfg(depot)?;
    let max_row_limit = cfg.read().await.max_row_limit;
    let mut q = match req.parse_queries::<ClientSearchParams>() {
        Ok(q) => q,
        Err(e) => {
            res.render(StatusError::bad_request().detail(e.to_string()));
            return Ok(());
        }
    };

    if q._limit == 0 || q._limit > max_row_limit {
        q._limit = max_row_limit;
    }
    match _search_clients(scx, q).await {
        Ok(replys) => {
            let replys = replys.iter().map(|res| res.to_json()).collect::<Vec<_>>();
            res.render(Json(replys))
        }
        Err(e) => res.render(StatusError::service_unavailable().detail(e.to_string())),
    }
    Ok(())
}

#[handler]
async fn search_offlines(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> std::result::Result<(), salvo::Error> {
    let (scx, cfg) = get_scx_cfg(depot)?;
    let max_row_limit = cfg.read().await.max_row_limit;
    let mut q = match req.parse_queries::<ClientSearchParams>() {
        Ok(q) => q,
        Err(e) => {
            res.render(StatusError::bad_request().detail(e.to_string()));
            return Ok(());
        }
    };
    q.connected = Some(false);

    if q._limit == 0 || q._limit > max_row_limit {
        q._limit = max_row_limit;
    }
    match _search_clients(scx, q).await {
        Ok(replys) => {
            let replys = replys.iter().map(|res| res.to_json()).collect::<Vec<_>>();
            res.render(Json(replys))
        }
        Err(e) => res.render(StatusError::service_unavailable().detail(e.to_string())),
    }
    Ok(())
}

async fn _search_clients(
    scx: &ServerContext,
    q: ClientSearchParams,
) -> Result<Vec<ClientSearchResult>> {
    let replys = clients::search(scx, &q).await;
    Ok(replys)
}

#[handler]
async fn kick_client(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> std::result::Result<(), salvo::Error> {
    let (scx, _) = get_scx_cfg(depot)?;
    let clientid = req.param::<String>("clientid");
    if let Some(clientid) = clientid {
        let mut entry = scx
            .extends
            .shared()
            .await
            .entry(Id::from(scx.node.id(), ClientId::from(clientid)));
        let s = entry.session();
        if let Some(s) = s {
            match entry.kick(true, true, true).await {
                Err(e) => res.render(StatusError::service_unavailable().detail(e.to_string())),
                Ok(_) => res.render(Json(s.id.to_json())),
            }
        } else {
            res.status_code(StatusCode::NOT_FOUND);
        }
    } else {
        res.render(StatusError::bad_request())
    }
    Ok(())
}

#[handler]
async fn kick_offlines(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> std::result::Result<(), salvo::Error> {
    let (scx, cfg) = get_scx_cfg(depot)?;
    let max_row_limit = cfg.read().await.max_row_limit;
    let mut q = match req.parse_queries::<ClientSearchParams>() {
        Ok(q) => q,
        Err(e) => {
            res.render(StatusError::bad_request().detail(e.to_string()));
            return Ok(());
        }
    };
    q.connected = Some(false);

    if q._limit == 0 || q._limit > max_row_limit {
        q._limit = max_row_limit;
    }

    let mut count = 0;
    match _search_clients(scx, q).await {
        Ok(replys) => {
            for reply in replys.iter() {
                log::debug!("{log_prefix} clientid: {}", reply.clientid);
                let mut entry = scx.extends.shared().await.entry(Id::from(
                    reply.node_id,
                    ClientId::from(reply.clientid.clone()),
                ));
                let s = entry.session();
                if s.is_some() {
                    match entry.kick(true, true, true).await {
                        Err(e) => {
                            log::warn!("{log_prefix} {e}");
                        }
                        Ok(_) => {
                            count += 1;
                        }
                    }
                } else {
                    log::warn!(
                        "{log_prefix} session is not found, node_id: {}, clientid: {}",
                        reply.node_id,
                        reply.clientid
                    );
                }
            }
        }
        Err(e) => {
            log::warn!("{log_prefix} {e}");
        }
    }
    res.render(Json(json!({"count": count})));
    Ok(())
}

#[handler]
async fn check_online(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> std::result::Result<(), salvo::Error> {
    let (scx, _) = get_scx_cfg(depot)?;
    let clientid = req.param::<String>("clientid");
    if let Some(clientid) = clientid {
        let entry = scx
            .extends
            .shared()
            .await
            .entry(Id::from(scx.node.id(), ClientId::from(clientid)));

        let online = entry.online().await;
        res.render(Json(online));
    } else {
        res.render(StatusError::bad_request())
    }
    Ok(())
}

#[handler]
async fn query_subscriptions(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> std::result::Result<(), salvo::Error> {
    let (scx, cfg) = get_scx_cfg(depot)?;
    let max_row_limit = cfg.read().await.max_row_limit;
    let mut q = match req.parse_queries::<SubsSearchParams>() {
        Ok(q) => q,
        Err(e) => {
            res.render(StatusError::bad_request().detail(e.to_string()));
            return Ok(());
        }
    };
    if q._limit == 0 || q._limit > max_row_limit {
        q._limit = max_row_limit;
    }
    let replys = scx
        .extends
        .shared()
        .await
        .query_subscriptions(&q)
        .await
        .into_iter()
        .map(|res| res.to_json())
        .collect::<Vec<serde_json::Value>>();
    res.render(Json(replys));
    Ok(())
}

#[handler]
async fn get_client_subscriptions(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> std::result::Result<(), salvo::Error> {
    let (scx, _) = get_scx_cfg(depot)?;
    let clientid = req.param::<String>("clientid");
    if let Some(clientid) = clientid {
        let entry = scx
            .extends
            .shared()
            .await
            .entry(Id::from(scx.node.id(), ClientId::from(clientid)));
        if let Some(subs) = entry.subscriptions().await {
            let subs = subs
                .into_iter()
                .map(|res| res.to_json())
                .collect::<Vec<serde_json::Value>>();
            res.render(Json(subs));
        } else {
            res.status_code(StatusCode::NOT_FOUND);
        }
    } else {
        res.render(StatusError::bad_request());
    }
    Ok(())
}

#[handler]
async fn get_routes(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> std::result::Result<(), salvo::Error> {
    let (scx, cfg) = get_scx_cfg(depot)?;
    let max_row_limit = cfg.read().await.max_row_limit;
    let limit = req.query::<usize>("_limit");
    let limit = if let Some(limit) = limit {
        if limit > max_row_limit {
            max_row_limit
        } else {
            limit
        }
    } else {
        max_row_limit
    };
    let replys = scx.extends.router().await.gets(limit).await;
    res.render(Json(replys));
    Ok(())
}

#[handler]
async fn get_route(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> std::result::Result<(), salvo::Error> {
    let (scx, _) = get_scx_cfg(depot)?;
    let topic = req.param::<String>("topic");
    if let Some(topic) = topic {
        match scx.extends.router().await.get(&topic).await {
            Ok(replys) => res.render(Json(replys)),
            Err(e) => res.render(StatusError::service_unavailable().detail(e.to_string())),
        }
    } else {
        res.render(StatusError::bad_request())
    }
    Ok(())
}

#[handler]
async fn publish(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> std::result::Result<(), salvo::Error> {
    let (scx, cfg) = get_scx_cfg(depot)?;
    let expiry_interval = {
        let cfg_rl = cfg.read().await;
        cfg_rl.message_expiry_interval
    };

    let params = match req.parse_json::<PublishParams>().await {
        Ok(p) => p,
        Err(e) => {
            res.render(StatusError::bad_request().detail(e.to_string()));
            return Ok(());
        }
    };
    match _publish(scx, params, expiry_interval).await {
        Ok(()) => res.render(Text::Plain("ok")),
        Err(e) => res.render(StatusError::service_unavailable().detail(e.to_string())),
    }
    Ok(())
}

async fn _publish(
    scx: &ServerContext,
    params: PublishParams,
    expiry_interval: Duration,
) -> Result<()> {
    super::super::re_publish_message(params, scx, Some(expiry_interval)).await?;
    Ok(())
}

#[handler]
async fn subscribe(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> std::result::Result<(), salvo::Error> {
    let (scx, _) = get_scx_cfg(depot)?;
    let params = match req.parse_json::<SubscribeParams>().await {
        Ok(p) => p,
        Err(e) => {
            res.render(StatusError::bad_request().detail(e.to_string()));
            return Ok(());
        }
    };

    #[allow(clippy::mutable_key_type)]
    match subs::subscribe(scx, params).await {
        Ok(replys) => {
            let replys = replys
                .into_iter()
                .map(|(t, r)| {
                    let r = match r {
                        Ok(b) => serde_json::Value::Bool(b),
                        Err(e) => serde_json::Value::String(e.to_string()),
                    };
                    (t, r)
                })
                .collect::<HashMap<_, _>>();
            res.render(Json(replys))
        }
        Err(e) => res.render(StatusError::service_unavailable().detail(e.to_string())),
    }

    Ok(())
}

#[handler]
async fn unsubscribe(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> std::result::Result<(), salvo::Error> {
    let (scx, _) = get_scx_cfg(depot)?;
    let params = match req.parse_json::<UnsubscribeParams>().await {
        Ok(p) => p,
        Err(e) => {
            res.render(StatusError::bad_request().detail(e.to_string()));
            return Ok(());
        }
    };
    match subs::unsubscribe(scx, params).await {
        Ok(()) => res.render(Json(true)),
        Err(e) => res.render(StatusError::service_unavailable().detail(e.to_string())),
    }

    Ok(())
}

#[handler]
async fn all_plugins(
    depot: &mut Depot,
    res: &mut Response,
) -> std::result::Result<(), salvo::Error> {
    let (scx, _) = get_scx_cfg(depot)?;
    match _all_plugins(scx).await {
        Ok(pluginss) => res.render(Json(pluginss)),
        Err(e) => res.render(StatusError::service_unavailable().detail(e.to_string())),
    }
    Ok(())
}

#[inline]
async fn _all_plugins(scx: &ServerContext) -> Result<Vec<serde_json::Value>> {
    let mut pluginss = Vec::new();
    let node_id = scx.node.id();
    let plugins = plugin::get_plugins(scx).await?;
    let plugins = plugins
        .into_iter()
        .map(|p| p.to_json())
        .collect::<Result<Vec<_>>>()?;
    pluginss.push(json!({
        "node": node_id,
        "plugins": plugins,
    }));
    Ok(pluginss)
}

#[handler]
async fn node_plugins(
    depot: &mut Depot,
    res: &mut Response,
) -> std::result::Result<(), salvo::Error> {
    let (scx, _) = get_scx_cfg(depot)?;
    match _node_plugins(scx).await {
        Ok(plugins) => res.render(Json(plugins)),
        Err(e) => res.render(StatusError::service_unavailable().detail(e.to_string())),
    }
    Ok(())
}

async fn _node_plugins(scx: &ServerContext) -> Result<Vec<serde_json::Value>> {
    let plugins = plugin::get_plugins(scx).await?;
    plugins
        .into_iter()
        .map(|p| p.to_json())
        .collect::<Result<Vec<_>>>()
}

#[handler]
async fn node_plugin_info(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> std::result::Result<(), salvo::Error> {
    let (scx, _) = get_scx_cfg(depot)?;
    let name = if let Some(name) = req.param::<String>("plugin") {
        name
    } else {
        res.status_code(StatusCode::NOT_FOUND);
        return Ok(());
    };

    match _node_plugin_info(scx, &name).await {
        Ok(plugin) => res.render(Json(plugin)),
        Err(e) => res.render(StatusError::service_unavailable().detail(e.to_string())),
    }

    Ok(())
}

async fn _node_plugin_info(scx: &ServerContext, name: &str) -> Result<Option<serde_json::Value>> {
    let plugin = plugin::get_plugin(scx, name).await?;
    if let Some(plugin) = plugin {
        Ok(Some(plugin.to_json()?))
    } else {
        Ok(None)
    }
}

#[handler]
async fn node_plugin_config(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> std::result::Result<(), salvo::Error> {
    let (scx, _) = get_scx_cfg(depot)?;
    let name = if let Some(name) = req.param::<String>("plugin") {
        name
    } else {
        res.status_code(StatusCode::NOT_FOUND);
        return Ok(());
    };

    match _node_plugin_config(scx, &name).await {
        Ok(cfg) => {
            res.headers_mut().insert(
                CONTENT_TYPE,
                HeaderValue::from_static("application/json; charset=utf-8"),
            );
            res.write_body(cfg).ok();
        }
        Err(e) => res.render(StatusError::service_unavailable().detail(e.to_string())),
    }
    Ok(())
}

async fn _node_plugin_config(scx: &ServerContext, name: &str) -> Result<Vec<u8>> {
    let plugin_cfg = plugin::get_plugin_config(scx, name).await?;
    Ok(plugin_cfg)
}

#[handler]
async fn node_plugin_config_reload(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> std::result::Result<(), salvo::Error> {
    let (scx, _) = get_scx_cfg(depot)?;
    let name = if let Some(name) = req.param::<String>("plugin") {
        name
    } else {
        res.status_code(StatusCode::NOT_FOUND);
        return Ok(());
    };

    match _node_plugin_config_reload(scx, &name).await {
        Ok(r) => res.render(Json(r)),
        Err(e) => res.render(StatusError::service_unavailable().detail(e.to_string())),
    }
    Ok(())
}

async fn _node_plugin_config_reload(scx: &ServerContext, name: &str) -> Result<bool> {
    scx.plugins.load_config(name).await?;
    Ok(true)
}

#[handler]
async fn node_plugin_load(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> std::result::Result<(), salvo::Error> {
    let (scx, _) = get_scx_cfg(depot)?;
    let name = if let Some(name) = req.param::<String>("plugin") {
        name
    } else {
        res.status_code(StatusCode::NOT_FOUND);
        return Ok(());
    };

    match _node_plugin_load(scx, &name).await {
        Ok(r) => res.render(Json(r)),
        Err(e) => res.render(StatusError::service_unavailable().detail(e.to_string())),
    }
    Ok(())
}

async fn _node_plugin_load(scx: &ServerContext, name: &str) -> Result<bool> {
    scx.plugins.start(name).await?;
    Ok(true)
}

#[handler]
async fn node_plugin_unload(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> std::result::Result<(), salvo::Error> {
    let (scx, _) = get_scx_cfg(depot)?;

    let name = if let Some(name) = req.param::<String>("plugin") {
        name
    } else {
        res.status_code(StatusCode::NOT_FOUND);
        return Ok(());
    };

    match _node_plugin_unload(scx, &name).await {
        Ok(r) => res.render(Json(r)),
        Err(e) => res.render(StatusError::service_unavailable().detail(e.to_string())),
    }
    Ok(())
}

async fn _node_plugin_unload(scx: &ServerContext, name: &str) -> Result<bool> {
    scx.plugins.stop(name).await
}

#[handler]
async fn get_stats_sum(
    depot: &mut Depot,
    res: &mut Response,
) -> std::result::Result<(), salvo::Error> {
    let (scx, _) = get_scx_cfg(depot)?;
    match _get_stats_sum(scx, false).await {
        Ok(stats_sum) => res.render(Json(stats_sum)),
        Err(e) => res.render(StatusError::service_unavailable().detail(e.to_string())),
    }
    Ok(())
}

async fn _get_stats_sum(scx: &ServerContext, is_sys: bool) -> Result<serde_json::Value> {
    let this_id = scx.node.id();
    let mut nodes = HashMap::default();
    nodes.insert(
        this_id,
        json!({
            "name": scx.node.name(scx,this_id).await,
            "running": scx.node.status(scx).await.is_running(),
        }),
    );
    let stats_sum = scx.stats.clone(scx).await;
    let stats_sum = json!({
        "nodes": nodes,
        "stats": if is_sys { stats_sum.to_sys_json(scx).await} else {stats_sum.to_json(scx).await}
    });
    Ok(stats_sum)
}

#[handler]
async fn get_stats(depot: &mut Depot, res: &mut Response) -> std::result::Result<(), salvo::Error> {
    let (scx, _) = get_scx_cfg(depot)?;
    match get_stats_one(scx).await {
        Ok((node_status, state)) => {
            let data = json!({
                "node": node_status.is_running(),
                "stats": state
            });
            res.render(Json(data))
        }
        Err(e) => res.render(StatusError::service_unavailable().detail(e.to_string())),
    }

    Ok(())
}

#[inline]
pub(super) async fn get_stats_one(scx: &ServerContext) -> Result<(NodeStatus, Box<Stats>)> {
    let node_status = scx.node.status(scx).await;
    let stats = scx.stats.clone(scx).await;
    Ok((node_status, Box::new(stats)))
}
