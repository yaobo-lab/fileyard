//! Discord OAuth and DM Notification Handlers
//!
//! Provides endpoints for:
//! - OAuth flow (connect/disconnect Discord account)
//! - Notification preferences
//! - Sending DM notifications

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{Json, Redirect},
    Extension,
};
use chrono::{Duration, Utc};
use app_entity::repositories::DiscordPreferencePatch;
use app_entity::DataStore;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
use uuid::Uuid;

use crate::AppState;
use crate::auth::middleware::AuthUser;

// ==================== Configuration ====================

/// Discord OAuth configuration from `etc/config.toml`.
#[derive(Clone)]
pub struct DiscordConfig {
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uri: String,
}

impl DiscordConfig {
    pub fn from_config() -> Option<Self> {
        let source = &types::config::get_config().discord;
        if source.client_id.is_empty()
            || source.client_secret.is_empty()
            || source.redirect_uri.is_empty()
        {
            return None;
        }

        Some(Self {
            client_id: source.client_id.clone(),
            client_secret: source.client_secret.clone(),
            redirect_uri: source.redirect_uri.clone(),
        })
    }
}

// ==================== Models ====================

#[derive(Debug, Serialize)]
pub struct DiscordConnectionStatus {
    pub connected: bool,
    pub discord_username: Option<String>,
    pub discord_avatar_url: Option<String>,
    pub dm_notifications_enabled: bool,
    pub notify_file_shared: bool,
    pub notify_file_uploaded: bool,
    pub notify_comments: bool,
    pub notify_file_requests: bool,
}

#[derive(Debug, Deserialize)]
pub struct OAuthCallbackParams {
    pub code: Option<String>,
    pub state: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct UpdatePreferencesInput {
    pub dm_notifications_enabled: Option<bool>,
    pub notify_file_shared: Option<bool>,
    pub notify_file_uploaded: Option<bool>,
    pub notify_comments: Option<bool>,
    pub notify_file_requests: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct TenantDiscordSettings {
    pub enabled: bool,
}

// Discord API response types
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct DiscordTokenResponse {
    access_token: String,
    token_type: String,
    expires_in: i64,
    refresh_token: String,
    scope: String,
}

#[derive(Debug, Deserialize)]
struct DiscordUser {
    id: String,
    username: String,
    #[allow(dead_code)]
    discriminator: String,
    avatar: Option<String>,
}

// ==================== Tenant Settings ====================

/// Check if Discord is enabled for the tenant
/// GET /api/discord/settings
pub async fn get_discord_settings(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
) -> Result<Json<TenantDiscordSettings>, StatusCode> {
    let enabled = state
        .store
        .discord()
        .is_enabled(auth.tenant_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(TenantDiscordSettings { enabled }))
}

/// Update Discord settings (Admin only)
/// POST /api/discord/settings/update
pub async fn update_discord_settings(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    Json(input): Json<Value>,
) -> Result<Json<Value>, StatusCode> {
    if auth.role != "Admin" && auth.role != "SuperAdmin" {
        return Err(StatusCode::FORBIDDEN);
    }

    let enabled = input["enabled"].as_bool().unwrap_or(false);

    state
        .store
        .discord()
        .set_enabled(auth.tenant_id, enabled)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(json!({ "enabled": enabled })))
}

// ==================== User Connection Status ====================

/// Get user's Discord connection status
/// GET /api/discord/status
pub async fn get_connection_status(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
) -> Result<Json<DiscordConnectionStatus>, StatusCode> {
    let connection = state
        .store
        .discord()
        .get_connection(auth.user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    match connection {
        Some(conn) => {
            let avatar_url = conn
                .discord_avatar
                .as_ref()
                .map(|a| format!("https://cdn.discordapp.com/avatars/{}/{}.png", conn.discord_user_id, a));
            Ok(Json(DiscordConnectionStatus {
                connected: true,
                discord_username: conn.discord_username,
                discord_avatar_url: avatar_url,
                dm_notifications_enabled: conn.dm_notifications_enabled,
                notify_file_shared: conn.notify_file_shared,
                notify_file_uploaded: conn.notify_file_uploaded,
                notify_comments: conn.notify_comments,
                notify_file_requests: conn.notify_file_requests,
            }))
        }
        None => Ok(Json(DiscordConnectionStatus {
            connected: false,
            discord_username: None,
            discord_avatar_url: None,
            dm_notifications_enabled: true,
            notify_file_shared: true,
            notify_file_uploaded: true,
            notify_comments: true,
            notify_file_requests: true,
        })),
    }
}

// ==================== OAuth Flow ====================

/// Start Discord OAuth flow
/// GET /api/discord/connect
pub async fn start_oauth(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
) -> Result<Redirect, (StatusCode, Json<Value>)> {
    let config = DiscordConfig::from_config().ok_or_else(|| {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({
                "error": "Discord integration is not configured"
            })),
        )
    })?;

    let enabled = state
        .store
        .discord()
        .is_enabled(auth.tenant_id)
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Database error"})),
            )
        })?;

    if !enabled {
        return Err((
            StatusCode::FORBIDDEN,
            Json(json!({
                "error": "Discord is not enabled for your organization"
            })),
        ));
    }

    let state_token = format!("{}", Uuid::new_v4());
    let expires_at = Utc::now() + Duration::minutes(10);

    state
        .store
        .discord()
        .create_oauth_state(&state_token, auth.user_id, auth.tenant_id, expires_at)
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to create OAuth state"})),
            )
        })?;

    let oauth_url = format!(
        "https://discord.com/api/oauth2/authorize?client_id={}&redirect_uri={}&response_type=code&scope=identify&state={}",
        config.client_id,
        urlencoding::encode(&config.redirect_uri),
        state_token
    );

    Ok(Redirect::temporary(&oauth_url))
}

/// Handle Discord OAuth callback
/// GET /api/discord/callback
pub async fn oauth_callback(
    State(state): State<Arc<AppState>>,
    Query(params): Query<OAuthCallbackParams>,
) -> Result<Redirect, (StatusCode, String)> {
    if let Some(error) = params.error {
        log::warn!("Discord OAuth error: {}", error);
        return Ok(Redirect::temporary("/settings?discord=error"));
    }

    let code = params
        .code
        .ok_or((StatusCode::BAD_REQUEST, "Missing code".to_string()))?;
    let state_token = params
        .state
        .ok_or((StatusCode::BAD_REQUEST, "Missing state".to_string()))?;

    let state_record = state
        .store
        .discord()
        .consume_oauth_state(&state_token)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error".to_string()))?;

    let (user_id, tenant_id) = state_record.ok_or((
        StatusCode::BAD_REQUEST,
        "Invalid or expired state".to_string(),
    ))?;

    let config = DiscordConfig::from_config().ok_or((
        StatusCode::SERVICE_UNAVAILABLE,
        "Discord not configured".to_string(),
    ))?;

    let client = Client::new();
    let token_response = client
        .post("https://discord.com/api/oauth2/token")
        .form(&[
            ("client_id", config.client_id.as_str()),
            ("client_secret", config.client_secret.as_str()),
            ("grant_type", "authorization_code"),
            ("code", &code),
            ("redirect_uri", &config.redirect_uri),
        ])
        .send()
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, format!("Discord API error: {}", e)))?;

    if !token_response.status().is_success() {
        let error_text = token_response.text().await.unwrap_or_default();
        log::error!("Discord token exchange failed: {}", error_text);
        return Ok(Redirect::temporary("/settings?discord=error"));
    }

    let tokens: DiscordTokenResponse = token_response.json().await.map_err(|_| {
        (
            StatusCode::BAD_GATEWAY,
            "Invalid Discord response".to_string(),
        )
    })?;

    let user_response = client
        .get("https://discord.com/api/v10/users/@me")
        .header("Authorization", format!("Bearer {}", tokens.access_token))
        .send()
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, format!("Discord API error: {}", e)))?;

    if !user_response.status().is_success() {
        return Ok(Redirect::temporary("/settings?discord=error"));
    }

    let discord_user: DiscordUser = user_response.json().await.map_err(|_| {
        (
            StatusCode::BAD_GATEWAY,
            "Invalid Discord user response".to_string(),
        )
    })?;

    let expires_at = Utc::now() + Duration::seconds(tokens.expires_in);

    state
        .store
        .discord()
        .upsert_connection(
            user_id,
            tenant_id,
            &discord_user.id,
            &discord_user.username,
            discord_user.avatar.as_deref(),
            &tokens.access_token,
            &tokens.refresh_token,
            expires_at,
        )
        .await
        .map_err(|e| {
            log::error!("Failed to store Discord connection: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to save connection".to_string(),
            )
        })?;

    log::info!(
        "Discord account connected (user_id: {}, discord_user: {})",
        user_id,
        discord_user.username
    );

    Ok(Redirect::temporary("/settings?discord=connected"))
}

/// Disconnect Discord account
/// POST /api/discord/disconnect
pub async fn disconnect(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
) -> Result<Json<Value>, StatusCode> {
    state
        .store
        .discord()
        .disconnect(auth.user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    log::info!("Discord account disconnected (user_id: {})", auth.user_id);

    Ok(Json(json!({ "success": true })))
}

// ==================== Notification Preferences ====================

/// Update notification preferences
/// POST /api/discord/preferences
pub async fn update_preferences(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    Json(input): Json<Value>,
) -> Result<Json<Value>, StatusCode> {
    let patch = DiscordPreferencePatch {
        dm_notifications_enabled: input.get("dm_notifications_enabled").and_then(|v| v.as_bool()),
        notify_file_shared: input.get("notify_file_shared").and_then(|v| v.as_bool()),
        notify_file_uploaded: input.get("notify_file_uploaded").and_then(|v| v.as_bool()),
        notify_comments: input.get("notify_comments").and_then(|v| v.as_bool()),
        notify_file_requests: input.get("notify_file_requests").and_then(|v| v.as_bool()),
    };

    state
        .store
        .discord()
        .update_preferences(auth.user_id, patch)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(json!({ "success": true })))
}

// ==================== Send DM (Internal Service) ====================

/// Check if Discord is enabled for a tenant
pub async fn is_discord_enabled(store: &DataStore, tenant_id: Uuid) -> bool {
    store.discord().is_enabled(tenant_id).await.unwrap_or(false)
}

/// Send a Discord DM to a user (fire-and-forget, logs errors but doesn't fail)
pub async fn send_dm(
    store: &DataStore,
    user_id: Uuid,
    event_type: &str,
    message: &str,
) -> Result<(), String> {
    let connection = store
        .discord()
        .get_connection(user_id)
        .await
        .map_err(|e| format!("Database error: {}", e))?
        .ok_or("User not connected to Discord")?;

    if !connection.dm_notifications_enabled {
        return Ok(());
    }

    let should_send = match event_type {
        "file_shared" => connection.notify_file_shared,
        "file_uploaded" => connection.notify_file_uploaded,
        "comment" => connection.notify_comments,
        "file_request" => connection.notify_file_requests,
        _ => true,
    };

    if !should_send {
        return Ok(());
    }

    let client = Client::new();
    let channel_response = client
        .post("https://discord.com/api/v10/users/@me/channels")
        .header(
            "Authorization",
            format!("Bearer {}", connection.access_token_encrypted),
        )
        .json(&json!({ "recipient_id": connection.discord_user_id }))
        .send()
        .await
        .map_err(|e| format!("Discord API error: {}", e))?;

    if !channel_response.status().is_success() {
        let error = channel_response.text().await.unwrap_or_default();
        return Err(format!("Failed to create DM channel: {}", error));
    }

    let channel: Value = channel_response
        .json()
        .await
        .map_err(|_| "Invalid channel response")?;
    let channel_id = channel["id"].as_str().ok_or("Missing channel ID")?;

    let message_response = client
        .post(format!(
            "https://discord.com/api/v10/channels/{}/messages",
            channel_id
        ))
        .header(
            "Authorization",
            format!("Bearer {}", connection.access_token_encrypted),
        )
        .json(&json!({ "content": message }))
        .send()
        .await
        .map_err(|e| format!("Discord API error: {}", e))?;

    if !message_response.status().is_success() {
        let error = message_response.text().await.unwrap_or_default();
        return Err(format!("Failed to send message: {}", error));
    }

    log::info!("Discord DM sent (user_id: {}, event: {})", user_id, event_type);
    Ok(())
}

/// Test Discord connection by sending a test DM
/// POST /api/discord/test
pub async fn test_connection(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
) -> Result<Json<Value>, StatusCode> {
    let result = send_dm(
        &state.store,
        auth.user_id,
        "test",
        "Hello from FileYard! Your Discord integration is working properly.",
    )
    .await;

    match result {
        Ok(_) => Ok(Json(json!({
            "success": true,
            "message": "Test DM sent successfully"
        }))),
        Err(_e) => Err(StatusCode::BAD_REQUEST),
    }
}

// ==================== Event Notification Helpers ====================

pub async fn notify_file_upload(
    store: &DataStore,
    tenant_id: Uuid,
    owner_id: Uuid,
    file_name: &str,
    uploader_name: &str,
    request_name: &str,
) {
    if !is_discord_enabled(store, tenant_id).await {
        return;
    }

    let message = format!(
        "**New File Uploaded**\n`{}` was uploaded to your request **{}** by {}",
        file_name, request_name, uploader_name
    );

    if let Err(e) = send_dm(store, owner_id, "file_uploaded", &message).await {
        log::debug!("Discord DM skipped or failed: {}", e);
    }
}

pub async fn notify_file_shared(
    store: &DataStore,
    tenant_id: Uuid,
    recipient_id: Uuid,
    file_name: &str,
    sharer_name: &str,
    share_link: Option<&str>,
) {
    if !is_discord_enabled(store, tenant_id).await {
        return;
    }

    let mut message = format!(
        "**File Shared With You**\n{} shared `{}` with you.",
        sharer_name, file_name
    );

    if let Some(link) = share_link {
        message.push_str(&format!("\nAccess link: {}", link));
    }

    if let Err(e) = send_dm(store, recipient_id, "file_shared", &message).await {
        log::debug!("Discord DM skipped or failed: {}", e);
    }
}

pub async fn notify_comment(
    store: &DataStore,
    tenant_id: Uuid,
    owner_id: Uuid,
    file_name: &str,
    commenter_name: &str,
    comment_preview: &str,
) {
    if !is_discord_enabled(store, tenant_id).await {
        return;
    }

    let message = format!(
        "**New Comment**\n{} commented on `{}`:\n> {}",
        commenter_name, file_name, comment_preview
    );

    if let Err(e) = send_dm(store, owner_id, "comment", &message).await {
        log::debug!("Discord DM skipped or failed: {}", e);
    }
}

#[allow(dead_code)]
pub async fn notify_file_request(
    store: &DataStore,
    tenant_id: Uuid,
    recipient_id: Uuid,
    request_name: &str,
    requester_name: &str,
    request_link: &str,
) {
    if !is_discord_enabled(store, tenant_id).await {
        return;
    }

    let message = format!(
        "**File Request**\n{} is requesting files for **{}**.\nUpload link: {}",
        requester_name, request_name, request_link
    );

    if let Err(e) = send_dm(store, recipient_id, "file_request", &message).await {
        log::debug!("Discord DM skipped or failed: {}", e);
    }
}
