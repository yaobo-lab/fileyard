//! OIDC SSO Handlers
//!
//! Provides endpoints for:
//! - Tenant OIDC provider management (CRUD)
//! - SSO login flow (discover, authorize, callback)
//! - Account linking/unlinking

use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{Json, Redirect},
    Extension,
};
use chrono::Utc;
use openidconnect::{
    core::{CoreClient, CoreIdTokenClaims, CoreProviderMetadata, CoreResponseType},
    AuthenticationFlow, AuthorizationCode, ClientId, ClientSecret, CsrfToken, IssuerUrl, Nonce,
    RedirectUrl, Scope, TokenResponse,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
use uuid::Uuid;

use crate::sso_common::{
    self, SsoIdentityParams, SsoProvisionConfig, SsoSessionResult, SsoUserResolution,
};
use crate::AppState;
use clovalink_auth::{require_super_admin, AuthUser};
use clovalink_entity::repositories::{NewOidcProvider, OidcProviderPatch};

// ==================== Models ====================

#[derive(Debug, Serialize)]
pub struct OidcProvider {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub name: String,
    pub slug: String,
    pub provider_type: String,
    pub issuer_url: String,
    pub client_id: String,
    #[serde(skip_serializing)]
    pub client_secret_encrypted: String,
    pub scopes: String,
    pub authorization_endpoint: Option<String>,
    pub token_endpoint: Option<String>,
    pub userinfo_endpoint: Option<String>,
    pub jwks_uri: Option<String>,
    pub auto_provision: bool,
    pub default_role: String,
    pub default_department_id: Option<Uuid>,
    pub email_domains: Vec<String>,
    pub trust_idp_mfa: bool,
    pub enabled: bool,
    pub created_at: chrono::DateTime<Utc>,
    pub updated_at: chrono::DateTime<Utc>,
}
impl From<clovalink_entity::entities::tenant_oidc_providers::Model> for OidcProvider {
    fn from(v: clovalink_entity::entities::tenant_oidc_providers::Model) -> Self {
        Self {
            id: v.id,
            tenant_id: v.tenant_id,
            name: v.name,
            slug: v.slug,
            provider_type: v.provider_type,
            issuer_url: v.issuer_url,
            client_id: v.client_id,
            client_secret_encrypted: v.client_secret_encrypted,
            scopes: v.scopes,
            authorization_endpoint: v.authorization_endpoint,
            token_endpoint: v.token_endpoint,
            userinfo_endpoint: v.userinfo_endpoint,
            jwks_uri: v.jwks_uri,
            auto_provision: v.auto_provision,
            default_role: v.default_role,
            default_department_id: v.default_department_id,
            email_domains: v.email_domains,
            trust_idp_mfa: v.trust_idp_mfa,
            enabled: v.enabled,
            created_at: v.created_at.with_timezone(&Utc),
            updated_at: v.updated_at.with_timezone(&Utc),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateProviderInput {
    pub name: String,
    pub slug: String,
    pub provider_type: Option<String>,
    pub issuer_url: String,
    pub client_id: String,
    pub client_secret: String,
    pub scopes: Option<String>,
    pub auto_provision: Option<bool>,
    pub default_role: Option<String>,
    pub default_department_id: Option<Uuid>,
    pub email_domains: Option<Vec<String>>,
    pub trust_idp_mfa: Option<bool>,
    pub enabled: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateProviderInput {
    pub name: Option<String>,
    pub slug: Option<String>,
    pub provider_type: Option<String>,
    pub issuer_url: Option<String>,
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
    pub scopes: Option<String>,
    pub auto_provision: Option<bool>,
    pub default_role: Option<String>,
    pub default_department_id: Option<Uuid>,
    pub email_domains: Option<Vec<String>>,
    pub trust_idp_mfa: Option<bool>,
    pub enabled: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct DiscoverParams {
    pub email: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct OidcCallbackParams {
    pub code: Option<String>,
    pub state: Option<String>,
    pub error: Option<String>,
    pub error_description: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct OidcIdentity {
    pub id: Uuid,
    pub user_id: Uuid,
    pub provider_id: Uuid,
    pub oidc_subject: String,
    pub oidc_issuer: String,
    pub oidc_email: Option<String>,
    pub oidc_name: Option<String>,
    pub last_login_at: Option<chrono::DateTime<Utc>>,
    pub login_count: i32,
    pub created_at: chrono::DateTime<Utc>,
    pub updated_at: chrono::DateTime<Utc>,
}
impl From<clovalink_entity::entities::user_oidc_identities::Model> for OidcIdentity {
    fn from(v: clovalink_entity::entities::user_oidc_identities::Model) -> Self {
        Self {
            id: v.id,
            user_id: v.user_id,
            provider_id: v.provider_id,
            oidc_subject: v.oidc_subject,
            oidc_issuer: v.oidc_issuer,
            oidc_email: v.oidc_email,
            oidc_name: v.oidc_name,
            last_login_at: v.last_login_at.map(|x| x.with_timezone(&Utc)),
            login_count: v.login_count,
            created_at: v.created_at.with_timezone(&Utc),
            updated_at: v.updated_at.with_timezone(&Utc),
        }
    }
}

// ==================== Provider Discovery (Public) ====================

/// Discover SSO providers (OIDC + SAML) by email domain
/// GET /api/auth/oidc/providers?email=user@acme.com
pub async fn discover_providers(
    State(state): State<Arc<AppState>>,
    Query(params): Query<DiscoverParams>,
) -> Result<Json<Value>, StatusCode> {
    let email = params.email.unwrap_or_default();
    let domain = email.split('@').nth(1).unwrap_or("");

    if domain.is_empty() {
        return Ok(Json(json!({ "providers": [], "sso_only": false })));
    }

    let providers = state.store.oidc().discover(domain).await.map_err(|e| {
        tracing::error!("Failed to discover providers: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    let oidc_providers: Vec<_> = providers
        .iter()
        .filter(|p| p.4 == "oidc")
        .map(|p| (p.0, p.1.clone(), p.2.clone(), p.3.clone()))
        .collect();
    let saml_providers: Vec<_> = providers
        .iter()
        .filter(|p| p.4 == "saml")
        .map(|p| (p.0, p.1.clone(), p.2.clone(), p.3.clone()))
        .collect();

    // Check if the tenant is SSO-only (no 'local' in auth_methods)
    let first_provider_id = oidc_providers
        .first()
        .map(|p| p.0)
        .or_else(|| saml_providers.first().map(|p| p.0));
    let sso_only = if let Some(pid) = first_provider_id {
        // Try OIDC provider's tenant first, then SAML
        state
            .store
            .oidc()
            .tenant_sso_only_for_provider(pid)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    } else {
        false
    };

    // Build unified provider list with protocol field
    let mut provider_list: Vec<Value> = Vec::new();
    for (id, name, slug, provider_type) in &oidc_providers {
        provider_list.push(json!({
            "id": id, "name": name, "slug": slug,
            "provider_type": provider_type, "protocol": "oidc",
        }));
    }
    for (id, name, slug, provider_type) in &saml_providers {
        provider_list.push(json!({
            "id": id, "name": name, "slug": slug,
            "provider_type": provider_type, "protocol": "saml",
        }));
    }

    Ok(Json(json!({
        "providers": provider_list,
        "sso_only": sso_only,
    })))
}

// ==================== OIDC Auth Flow (Public) ====================

/// Start OIDC authorization flow
/// GET /api/auth/oidc/authorize/{provider_id}
pub async fn start_oidc_auth(
    State(state): State<Arc<AppState>>,
    Path(provider_id): Path<Uuid>,
) -> Result<Redirect, (StatusCode, Json<Value>)> {
    let provider: OidcProvider = state
        .store
        .oidc()
        .enabled_provider(provider_id)
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Database error"})),
            )
        })?
        .map(Into::into)
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(json!({"error": "Provider not found or disabled"})),
            )
        })?;

    let base_url = types::config::get_config().web.base_url.clone();
    let callback_url = format!("{}/api/auth/oidc/callback", base_url);

    // Discover OIDC endpoints
    let issuer = IssuerUrl::new(provider.issuer_url.clone()).map_err(|e| {
        tracing::error!("Invalid issuer URL: {:?}", e);
        (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Invalid provider issuer URL"})),
        )
    })?;

    let metadata = CoreProviderMetadata::discover_async(issuer, &reqwest::Client::new())
        .await
        .map_err(|e| {
            tracing::error!("OIDC discovery failed for {}: {:?}", provider.name, e);
            (
                StatusCode::BAD_GATEWAY,
                Json(json!({"error": "Failed to discover OIDC provider endpoints"})),
            )
        })?;

    let client = CoreClient::from_provider_metadata(
        metadata,
        ClientId::new(provider.client_id.clone()),
        Some(ClientSecret::new(provider.client_secret_encrypted.clone())),
    )
    .set_redirect_uri(RedirectUrl::new(callback_url).map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Invalid callback URL"})),
        )
    })?);

    // Generate authorization URL
    let mut auth_request = client.authorize_url(
        AuthenticationFlow::<CoreResponseType>::AuthorizationCode,
        CsrfToken::new_random,
        Nonce::new_random,
    );

    // Add scopes
    for scope in provider.scopes.split_whitespace() {
        if scope != "openid" {
            auth_request = auth_request.add_scope(Scope::new(scope.to_string()));
        }
    }

    let (auth_url, csrf_state, nonce) = auth_request.url();

    // Store state for callback validation
    state
        .store
        .oidc()
        .create_state(
            csrf_state.secret().clone(),
            nonce.secret().clone(),
            provider.id,
            provider.tenant_id,
            None,
        )
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to create OAuth state"})),
            )
        })?;

    Ok(Redirect::temporary(auth_url.as_str()))
}

/// Handle OIDC callback from the identity provider
/// GET /api/auth/oidc/callback
pub async fn oidc_callback(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(params): Query<OidcCallbackParams>,
) -> Result<Redirect, (StatusCode, String)> {
    let frontend_url = types::config::get_config().web.base_url.clone();

    // Handle IdP errors
    if let Some(error) = params.error {
        let desc = params.error_description.unwrap_or_default();
        tracing::warn!("OIDC auth error: {} - {}", error, desc);
        return Ok(Redirect::temporary(&format!(
            "{}/login?error=oidc_error&message={}",
            frontend_url,
            urlencoding::encode(&desc)
        )));
    }

    let code = params
        .code
        .ok_or((StatusCode::BAD_REQUEST, "Missing code".to_string()))?;
    let state_token = params
        .state
        .ok_or((StatusCode::BAD_REQUEST, "Missing state".to_string()))?;

    // Validate state and get associated data
    let state_record = state
        .store
        .oidc()
        .consume_state(&state_token)
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Database error".to_string(),
            )
        })?;

    let state_record = state_record.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            "Invalid or expired state".to_string(),
        )
    })?;
    let (nonce_str, provider_id, tenant_id, linking_user_id) = (
        state_record.nonce,
        state_record.provider_id,
        state_record.tenant_id,
        state_record.user_id,
    );

    // Load provider config
    let provider: OidcProvider = state
        .store
        .oidc()
        .provider(provider_id)
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Provider not found".to_string(),
            )
        })?
        .ok_or((
            StatusCode::INTERNAL_SERVER_ERROR,
            "Provider not found".to_string(),
        ))?
        .into();

    let base_url = types::config::get_config().web.base_url.clone();
    let callback_url = format!("{}/api/auth/oidc/callback", base_url);

    // Discover OIDC endpoints and exchange code for tokens
    let issuer = IssuerUrl::new(provider.issuer_url.clone()).map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Invalid issuer".to_string(),
        )
    })?;

    let metadata = CoreProviderMetadata::discover_async(issuer, &reqwest::Client::new())
        .await
        .map_err(|e| {
            tracing::error!("OIDC discovery failed: {:?}", e);
            (StatusCode::BAD_GATEWAY, "OIDC discovery failed".to_string())
        })?;

    let client = CoreClient::from_provider_metadata(
        metadata,
        ClientId::new(provider.client_id.clone()),
        Some(ClientSecret::new(provider.client_secret_encrypted.clone())),
    )
    .set_redirect_uri(RedirectUrl::new(callback_url).map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Invalid callback URL".to_string(),
        )
    })?);

    // Exchange authorization code for tokens
    let http_client = reqwest::Client::new();
    let token_response = client
        .exchange_code(AuthorizationCode::new(code))
        .map_err(|e| {
            tracing::error!("Token exchange config error: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Token exchange config error".to_string(),
            )
        })?
        .request_async(&http_client)
        .await
        .map_err(|e| {
            tracing::error!("Token exchange failed: {:?}", e);
            (StatusCode::BAD_GATEWAY, "Token exchange failed".to_string())
        })?;

    // Verify and extract ID token claims
    let id_token = token_response.id_token().ok_or_else(|| {
        (
            StatusCode::BAD_GATEWAY,
            "No ID token in response".to_string(),
        )
    })?;

    let nonce = Nonce::new(nonce_str);
    let id_token_verifier = client.id_token_verifier();
    let claims: &CoreIdTokenClaims = id_token.claims(&id_token_verifier, &nonce).map_err(|e| {
        tracing::error!("ID token verification failed: {:?}", e);
        (
            StatusCode::BAD_GATEWAY,
            "ID token verification failed".to_string(),
        )
    })?;

    // Extract user info from claims
    let oidc_subject = claims.subject().to_string();
    let oidc_email = claims.email().map(|e| e.to_string());
    let oidc_name = claims
        .name()
        .and_then(|n| n.get(None))
        .map(|n| n.to_string());

    // === Account Linking Flow ===
    if let Some(linking_user_id) = linking_user_id {
        // Verify user still exists
        let user = state
            .store
            .sso()
            .active_user(linking_user_id)
            .await
            .map_err(|_| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Database error".to_string(),
                )
            })?;

        let user = user.ok_or_else(|| (StatusCode::NOT_FOUND, "User not found".to_string()))?;

        // Create identity link
        state
            .store
            .sso()
            .link_identity(
                "oidc",
                user.id,
                provider_id,
                &oidc_subject,
                &provider.issuer_url,
                oidc_email.as_deref(),
                oidc_name.as_deref(),
            )
            .await
            .map_err(|e| {
                tracing::error!("Failed to link OIDC identity: {:?}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Failed to link identity".to_string(),
                )
            })?;

        // Update identity_provider to hybrid if currently local
        if user.identity_provider == "local" {
            let _ = state.store.sso().set_hybrid(user.id).await;
        }

        tracing::info!(user_id = %user.id, provider = %provider.name, "OIDC identity linked");
        return Ok(Redirect::temporary(&format!(
            "{}/profile?oidc=linked",
            frontend_url
        )));
    }

    // === Login Flow (via shared SSO logic) ===

    let identity = SsoIdentityParams {
        protocol: "oidc".to_string(),
        provider_id,
        tenant_id,
        subject: oidc_subject.clone(),
        issuer: provider.issuer_url.clone(),
        email: oidc_email.clone(),
        name: oidc_name.clone(),
    };

    let provision_config = SsoProvisionConfig {
        auto_provision: provider.auto_provision,
        default_role: provider.default_role.clone(),
        default_custom_role_id: None, // OIDC providers don't have this yet
        default_department_id: provider.default_department_id,
        trust_idp_mfa: provider.trust_idp_mfa,
        provider_name: provider.name.clone(),
        provider_slug: provider.slug.clone(),
    };

    // TODO: Extract custom claims (groups, roles) from ID token for attribute mapping
    let role_override = None;

    let user = match sso_common::resolve_sso_user(
        &state.store,
        &identity,
        &provision_config,
        role_override.as_ref(),
    )
    .await?
    {
        SsoUserResolution::ExistingUser(u) | SsoUserResolution::NewUser(u) => u,
        SsoUserResolution::NoAccount => {
            return Ok(Redirect::temporary(&format!(
                "{}/login?error=no_account",
                frontend_url
            )));
        }
        SsoUserResolution::NoEmail => {
            return Ok(Redirect::temporary(&format!(
                "{}/login?error=no_email",
                frontend_url
            )));
        }
    };

    // Update OIDC identity login tracking
    let _ = state
        .store
        .oidc()
        .touch_identity(provider_id, &oidc_subject, oidc_email, oidc_name)
        .await;

    // Load tenant
    let tenant = state
        .store
        .sso()
        .tenant(tenant_id)
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Tenant not found".to_string(),
            )
        })?
        .ok_or((
            StatusCode::INTERNAL_SERVER_ERROR,
            "Tenant not found".to_string(),
        ))?;

    // Create session via shared logic
    match sso_common::create_sso_session(
        &state.store,
        &user,
        &tenant,
        &headers,
        &provision_config,
        &frontend_url,
    )
    .await?
    {
        SsoSessionResult::Token(token) => Ok(Redirect::temporary(&format!(
            "{}/auth/oidc/complete?token={}",
            frontend_url, token
        ))),
        SsoSessionResult::Pending2fa {
            user_id,
            provider_slug,
        } => Ok(Redirect::temporary(&format!(
            "{}/login?pending_2fa=true&user_id={}&provider={}",
            frontend_url, user_id, provider_slug
        ))),
        SsoSessionResult::Suspended => Ok(Redirect::temporary(&format!(
            "{}/login?error=suspended",
            frontend_url
        ))),
    }
}

// ==================== Provider Management (Admin) ====================

/// List OIDC providers for the current tenant
/// GET /api/oidc/providers
pub async fn list_providers(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
) -> Result<Json<Value>, StatusCode> {
    require_super_admin(&auth)?;

    let providers: Vec<OidcProvider> = state
        .store
        .oidc()
        .list(auth.tenant_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .into_iter()
        .map(Into::into)
        .collect();

    Ok(Json(json!({ "providers": providers })))
}

/// Create a new OIDC provider
/// POST /api/oidc/providers
pub async fn create_provider(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    Json(input): Json<CreateProviderInput>,
) -> Result<Json<Value>, StatusCode> {
    require_super_admin(&auth)?;

    let provider: OidcProvider = state
        .store
        .oidc()
        .create(NewOidcProvider {
            tenant_id: auth.tenant_id,
            name: input.name,
            slug: input.slug,
            provider_type: input.provider_type.unwrap_or_else(|| "generic".into()),
            issuer_url: input.issuer_url,
            client_id: input.client_id,
            client_secret: input.client_secret,
            scopes: input
                .scopes
                .unwrap_or_else(|| "openid email profile".into()),
            auto_provision: input.auto_provision.unwrap_or(false),
            default_role: input.default_role.unwrap_or_else(|| "Employee".into()),
            default_department_id: input.default_department_id,
            email_domains: input.email_domains.unwrap_or_default(),
            trust_idp_mfa: input.trust_idp_mfa.unwrap_or(true),
            enabled: input.enabled.unwrap_or(true),
        })
        .await
        .map_err(|e| {
            tracing::error!("Failed to create OIDC provider: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .into();

    Ok(Json(json!({ "provider": provider })))
}

/// Update an OIDC provider
/// PUT /api/oidc/providers/{id}
pub async fn update_provider(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<Uuid>,
    Json(input): Json<UpdateProviderInput>,
) -> Result<Json<Value>, StatusCode> {
    require_super_admin(&auth)?;

    let provider: OidcProvider = state
        .store
        .oidc()
        .update(
            auth.tenant_id,
            id,
            OidcProviderPatch {
                name: input.name,
                slug: input.slug,
                provider_type: input.provider_type,
                issuer_url: input.issuer_url,
                client_id: input.client_id,
                client_secret: input.client_secret,
                scopes: input.scopes,
                auto_provision: input.auto_provision,
                default_role: input.default_role,
                default_department_id: input.default_department_id,
                email_domains: input.email_domains,
                trust_idp_mfa: input.trust_idp_mfa,
                enabled: input.enabled,
            },
        )
        .await
        .map_err(|e| {
            tracing::error!("Failed to update OIDC provider: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?
        .into();

    Ok(Json(json!({ "provider": provider })))
}

/// Delete an OIDC provider
/// DELETE /api/oidc/providers/{id}
pub async fn delete_provider(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>, StatusCode> {
    require_super_admin(&auth)?;

    // Check for OIDC-only users that would be locked out
    let oidc_only_count = state
        .store
        .oidc()
        .oidc_only_count(auth.tenant_id, id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if oidc_only_count > 0 {
        return Ok(Json(json!({
            "error": "provider_has_sso_only_users",
            "message": format!("{} user(s) use only this provider for login and would be locked out. Set passwords for them first.", oidc_only_count),
            "affected_count": oidc_only_count,
        })));
    }

    let deleted = state
        .store
        .oidc()
        .delete(auth.tenant_id, id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if !deleted {
        return Err(StatusCode::NOT_FOUND);
    }

    Ok(Json(json!({ "success": true })))
}

/// Test an OIDC provider by fetching its discovery document
/// POST /api/oidc/providers/{id}/test
pub async fn test_provider(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>, StatusCode> {
    require_super_admin(&auth)?;

    let provider: OidcProvider = state
        .store
        .oidc()
        .tenant_provider(auth.tenant_id, id, None)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?
        .into();

    let issuer =
        IssuerUrl::new(provider.issuer_url.clone()).map_err(|_| StatusCode::BAD_REQUEST)?;

    match CoreProviderMetadata::discover_async(issuer, &reqwest::Client::new()).await {
        Ok(metadata) => {
            let auth_endpoint = metadata.authorization_endpoint().to_string();
            let token_endpoint = metadata.token_endpoint().map(|e| e.to_string());

            Ok(Json(json!({
                "success": true,
                "authorization_endpoint": auth_endpoint,
                "token_endpoint": token_endpoint,
            })))
        }
        Err(e) => Ok(Json(json!({
            "success": false,
            "error": format!("Discovery failed: {}", e),
        }))),
    }
}

// ==================== Account Linking (Authenticated) ====================

/// Start OIDC flow for account linking
/// GET /api/auth/oidc/link/{provider_id}
pub async fn link_oidc_identity(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    Path(provider_id): Path<Uuid>,
) -> Result<Redirect, (StatusCode, Json<Value>)> {
    let provider: OidcProvider = state
        .store
        .oidc()
        .tenant_provider(auth.tenant_id, provider_id, Some(true))
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Database error"})),
            )
        })?
        .map(Into::into)
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(json!({"error": "Provider not found"})),
            )
        })?;

    let base_url = types::config::get_config().web.base_url.clone();
    let callback_url = format!("{}/api/auth/oidc/callback", base_url);

    let issuer = IssuerUrl::new(provider.issuer_url.clone()).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Invalid issuer URL"})),
        )
    })?;

    let metadata = CoreProviderMetadata::discover_async(issuer, &reqwest::Client::new())
        .await
        .map_err(|_| {
            (
                StatusCode::BAD_GATEWAY,
                Json(json!({"error": "OIDC discovery failed"})),
            )
        })?;

    let client = CoreClient::from_provider_metadata(
        metadata,
        ClientId::new(provider.client_id.clone()),
        Some(ClientSecret::new(provider.client_secret_encrypted.clone())),
    )
    .set_redirect_uri(RedirectUrl::new(callback_url).map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Invalid callback URL"})),
        )
    })?);

    let mut auth_request = client.authorize_url(
        AuthenticationFlow::<CoreResponseType>::AuthorizationCode,
        CsrfToken::new_random,
        Nonce::new_random,
    );

    for scope in provider.scopes.split_whitespace() {
        if scope != "openid" {
            auth_request = auth_request.add_scope(Scope::new(scope.to_string()));
        }
    }

    let (auth_url, csrf_state, nonce) = auth_request.url();

    // Store state WITH user_id for account linking
    state
        .store
        .oidc()
        .create_state(
            csrf_state.secret().clone(),
            nonce.secret().clone(),
            provider.id,
            auth.tenant_id,
            Some(auth.user_id),
        )
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to create state"})),
            )
        })?;

    Ok(Redirect::temporary(auth_url.as_str()))
}

/// Unlink an OIDC identity
/// DELETE /api/auth/oidc/unlink/{identity_id}
pub async fn unlink_oidc_identity(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    Path(identity_id): Path<Uuid>,
) -> Result<Json<Value>, StatusCode> {
    // Verify identity belongs to user
    let identity = state
        .store
        .oidc()
        .identity(auth.user_id, identity_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let _identity = identity.ok_or(StatusCode::NOT_FOUND)?;

    // Check if user has a password — prevent lockout
    let user = state
        .store
        .users()
        .user(auth.user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Count remaining OIDC identities
    let identity_count = state
        .store
        .oidc()
        .identity_count(auth.user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if user.password_hash.is_none() && identity_count <= 1 {
        return Ok(Json(json!({
            "error": "cannot_unlink",
            "message": "Cannot unlink your only login method. Set a password first.",
        })));
    }

    state
        .store
        .oidc()
        .delete_identity(auth.user_id, identity_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Update identity_provider if no more OIDC identities
    if identity_count <= 1 && user.password_hash.is_some() {
        let _ = state.store.oidc().set_local(auth.user_id).await;
    }

    Ok(Json(json!({ "success": true })))
}

/// List current user's linked OIDC identities
/// GET /api/auth/oidc/identities
pub async fn list_my_identities(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
) -> Result<Json<Value>, StatusCode> {
    let identities: Vec<OidcIdentity> = state
        .store
        .oidc()
        .identities(auth.user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .into_iter()
        .map(Into::into)
        .collect();

    // Also fetch provider names for display
    let mut result = Vec::new();
    for identity in &identities {
        let provider_info = state
            .store
            .oidc()
            .provider(identity.provider_id)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        let (provider_name, provider_slug, provider_type) = provider_info
            .map(|p| (p.name, p.slug, p.provider_type))
            .unwrap_or_else(|| ("Unknown".into(), "unknown".into(), "generic".into()));

        result.push(json!({
            "id": identity.id,
            "provider_id": identity.provider_id,
            "provider_name": provider_name,
            "provider_slug": provider_slug,
            "provider_type": provider_type,
            "oidc_email": identity.oidc_email,
            "oidc_name": identity.oidc_name,
            "last_login_at": identity.last_login_at,
            "login_count": identity.login_count,
            "created_at": identity.created_at,
        }));
    }

    Ok(Json(json!({ "identities": result })))
}
