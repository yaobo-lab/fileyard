//! MQTT management endpoints, hosted at `/api/mqtt`.
use crate::auth::{require_super_admin, AuthUser};
use axum::{extract::Request, http::StatusCode, response::Response, Extension};
use mqttd::plugins::restapi::EmbeddedApi;

pub async fn dispatch(
    Extension(auth): Extension<AuthUser>,
    Extension(api): Extension<EmbeddedApi>,
    request: Request,
) -> Result<Response, StatusCode> {
    // The broker is shared across tenants, so tenant administrators cannot manage it.
    require_super_admin(&auth)?;
    Ok(api.handle(request).await)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;

    #[tokio::test]
    async fn only_super_admin_can_reach_broker_api() {
        for (role, expected) in [
            ("Admin", StatusCode::FORBIDDEN),
            ("User", StatusCode::FORBIDDEN),
            ("SuperAdmin", StatusCode::SERVICE_UNAVAILABLE),
        ] {
            let auth = AuthUser {
                user_id: uuid::Uuid::nil(),
                tenant_id: uuid::Uuid::nil(),
                role: role.into(),
                email: String::new(),
                ip_address: None,
            };
            let response = dispatch(
                Extension(auth),
                Extension(EmbeddedApi::default()),
                Request::new(Body::empty()),
            )
            .await;
            let status = match response {
                Ok(response) => response.status(),
                Err(status) => status,
            };
            assert_eq!(status, expected);
        }
    }
}
