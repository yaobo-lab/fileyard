//! MQTT management endpoints, hosted at `/api/mqtt`.
use crate::auth::{require_admin, AuthUser};
use axum::{extract::Request, http::StatusCode, response::Response, Extension};
use mqttd::plugins::restapi::EmbeddedApi;

pub async fn dispatch(
    Extension(auth): Extension<AuthUser>,
    Extension(api): Extension<EmbeddedApi>,
    request: Request,
) -> Result<Response, StatusCode> {
    // Read operations (GET) are accessible to authenticated users
    // Mutating operations (POST, PUT, DELETE) require Admin or SuperAdmin
    if request.method() != axum::http::Method::GET {
        require_admin(&auth)?;
    }
    Ok(api.handle(request).await)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;

    #[tokio::test]
    async fn authenticated_users_can_read_broker_api() {
        for role in ["User", "Employee", "Manager", "Admin", "SuperAdmin"] {
            let auth = AuthUser {
                user_id: uuid::Uuid::nil(),
                tenant_id: uuid::Uuid::nil(),
                role: role.into(),
                email: String::new(),
                ip_address: None,
            };
            let mut req = Request::new(Body::empty());
            *req.method_mut() = axum::http::Method::GET;
            let response = dispatch(
                Extension(auth),
                Extension(EmbeddedApi::default()),
                req,
            )
            .await;
            let status = match response {
                Ok(response) => response.status(),
                Err(status) => status,
            };
            assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
        }
    }

    #[tokio::test]
    async fn mutating_broker_api_requires_admin() {
        for (role, expected) in [
            ("User", StatusCode::FORBIDDEN),
            ("Employee", StatusCode::FORBIDDEN),
            ("Admin", StatusCode::SERVICE_UNAVAILABLE),
            ("SuperAdmin", StatusCode::SERVICE_UNAVAILABLE),
        ] {
            let auth = AuthUser {
                user_id: uuid::Uuid::nil(),
                tenant_id: uuid::Uuid::nil(),
                role: role.into(),
                email: String::new(),
                ip_address: None,
            };
            let mut req = Request::new(Body::empty());
            *req.method_mut() = axum::http::Method::DELETE;
            let response = dispatch(
                Extension(auth),
                Extension(EmbeddedApi::default()),
                req,
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
