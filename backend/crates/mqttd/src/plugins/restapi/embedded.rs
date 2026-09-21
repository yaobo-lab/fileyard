use super::{api, config::PluginConfig};
use axum::{
    body::{to_bytes, Body},
    extract::Request,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use rmqtt::context::ServerContext;
use salvo::{conn::SocketAddr, http::uri::Scheme, Service};
use std::sync::{Arc, RwLock};

/// In-process REST API. The hosting application must authenticate and authorize callers.
#[derive(Clone, Default)]
pub struct EmbeddedApi(Arc<RwLock<Option<Arc<Service>>>>);

impl EmbeddedApi {
    pub(crate) fn prepare(scx: ServerContext) -> anyhow::Result<Service> {
        let cfg = scx.plugins.read_config_default::<PluginConfig>("restapi")?;
        Ok(Service::new(api::route_at(
            scx,
            Arc::new(tokio::sync::RwLock::new(cfg)),
            None,
            "api/mqtt",
        )))
    }

    pub(crate) fn activate(&self, service: Service) -> ActiveApi {
        *self.0.write().expect("MQTT API state poisoned") = Some(Arc::new(service));
        ActiveApi(self.clone())
    }

    pub async fn handle(&self, request: Request) -> Response {
        let service = self.0.read().expect("MQTT API state poisoned").clone();
        let Some(service) = service else {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(serde_json::json!({"error": "MQTT server is not running"})),
            )
                .into_response();
        };
        let (mut parts, body) = request.into_parts();
        // Authentication belongs to the host; do not pass its credentials to plugins.
        parts.headers.remove(axum::http::header::AUTHORIZATION);
        parts.headers.remove(axum::http::header::COOKIE);
        let body = match to_bytes(body, 2 * 1024 * 1024).await {
            Ok(body) => body,
            Err(_) => return StatusCode::PAYLOAD_TOO_LARGE.into_response(),
        };
        let request =
            salvo::Request::from_hyper(axum::http::Request::from_parts(parts, body), Scheme::HTTP);
        service
            .hyper_handler(
                SocketAddr::Unknown,
                SocketAddr::Unknown,
                Scheme::HTTP,
                None,
                None,
            )
            .handle(request)
            .await
            .into_hyper()
            .map(Body::new)
    }
}

pub(crate) struct ActiveApi(EmbeddedApi);

impl Drop for ActiveApi {
    fn drop(&mut self) {
        *self.0 .0.write().expect("MQTT API state poisoned") = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn call(api: &EmbeddedApi, method: &str, path: &str, body: &str) -> Response {
        api.handle(
            Request::builder()
                .method(method)
                .uri(path)
                .header("content-type", "application/json")
                .body(Body::from(body.to_owned()))
                .unwrap(),
        )
        .await
    }

    #[tokio::test]
    async fn embedded_routes_preserve_requests_and_track_lifecycle() -> anyhow::Result<()> {
        let api = EmbeddedApi::default();
        assert_eq!(
            call(&api, "GET", "/api/mqtt", "").await.status(),
            StatusCode::SERVICE_UNAVAILABLE
        );
        let scx = ServerContext::new()
            .busy_check_enable(false)
            .plugins_config_map_add("restapi", "max_row_limit = 20")
            .plugins_config_map_add("auth", "priority = 10")
            .build()
            .await;
        crate::plugin::registers_with_mode(&scx, vec!["restapi".into()], true).await?;
        assert!(scx.plugins.get("restapi").is_none());
        let active = api.activate(EmbeddedApi::prepare(scx.clone())?);
        let response = call(&api, "GET", "/api/mqtt", "").await;
        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), 65536).await?;
        let endpoints: serde_json::Value = serde_json::from_slice(&body)?;
        assert!(endpoints
            .as_array()
            .unwrap()
            .iter()
            .all(|entry| entry["path"].as_str().unwrap().starts_with("/api/mqtt/")));
        assert_eq!(
            call(&api, "GET", "/api/mqtt/clients?_limit=2", "")
                .await
                .status(),
            StatusCode::OK
        );
        assert_eq!(
            call(&api, "GET", "/api/mqtt/clients?_limit=invalid", "")
                .await
                .status(),
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            call(&api, "POST", "/api/mqtt/publish", "{").await.status(),
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            call(
                &api,
                "POST",
                "/api/mqtt/publish",
                r#"{"topic":"test","payload":"hello","encoding":"plain"}"#
            )
            .await
            .status(),
            StatusCode::OK
        );
        let path = format!("/api/mqtt/plugins/{}/auth/load", scx.node.id());
        assert_eq!(call(&api, "PUT", &path, "").await.status(), StatusCode::OK);
        assert!(scx.plugins.is_active("auth"));
        assert_eq!(
            call(&api, "DELETE", "/api/mqtt/clients/missing", "")
                .await
                .status(),
            StatusCode::NOT_FOUND
        );
        drop(active);
        assert_eq!(
            call(&api, "GET", "/api/mqtt/clients", "").await.status(),
            StatusCode::SERVICE_UNAVAILABLE
        );
        Ok(())
    }
}
