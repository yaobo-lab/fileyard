//! AI API Handlers
//!
//! Provides endpoints for AI features: summarization, Q&A, semantic search, and settings management.
//! All operations respect tenant settings, role permissions, and compliance requirements.

use axum::{
    extract::{Extension, Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::AppState;
use app_ai::{
    models::{
        AiActionResponse, AnswerRequest, SearchRequest, SummarizeRequest, TenantAiSettingsResponse,
        UpdateAiSettingsInput, UsageStats,
    },
    AiError, AiService,
};
use crate::auth::middleware::AuthUser;

/// Query params for tenant-scoped AI endpoints
#[derive(Debug, Deserialize)]
pub struct TenantQuery {
    pub tenant_id: Option<Uuid>,
}

/// Query params for usage with pagination
#[derive(Debug, Deserialize)]
pub struct UsageQuery {
    pub tenant_id: Option<Uuid>,
    pub page: Option<i32>,
    pub per_page: Option<i32>,
}

/// Error response for AI endpoints
#[derive(Debug, Serialize)]
pub struct AiErrorResponse {
    pub error: String,
    pub code: String,
}

/// Convert AiError to HTTP response tuple
fn ai_error_response(err: AiError) -> (StatusCode, Json<AiErrorResponse>) {
    let code = match &err {
        AiError::Disabled => "AI_DISABLED",
        AiError::NoApiKey => "NO_API_KEY",
        AiError::Forbidden => "FORBIDDEN",
        AiError::MonthlyLimitExceeded => "MONTHLY_LIMIT_EXCEEDED",
        AiError::DailyLimitExceeded => "DAILY_LIMIT_EXCEEDED",
        AiError::HipaaNotApproved => "HIPAA_NOT_APPROVED",
        AiError::SoxReadOnly => "SOX_READ_ONLY",
        AiError::MaintenanceMode(_) => "MAINTENANCE_MODE",
        AiError::ProviderError(_) => "PROVIDER_ERROR",
        AiError::NetworkError(_) => "NETWORK_ERROR",
        AiError::InvalidResponse => "INVALID_RESPONSE",
        AiError::FileNotFound => "FILE_NOT_FOUND",
        AiError::ContentExtractionFailed => "CONTENT_EXTRACTION_FAILED",
        AiError::FileContentEmpty => "FILE_CONTENT_EMPTY",
        AiError::DatabaseError(_) => "DATABASE_ERROR",
        AiError::InternalError => "INTERNAL_ERROR",
    };

    let status =
        StatusCode::from_u16(err.status_code()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);

    (
        status,
        Json(AiErrorResponse {
            error: err.to_string(),
            code: code.to_string(),
        }),
    )
}

/// Check if AI features are enabled for the current user's tenant
pub async fn get_ai_status(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    Query(query): Query<TenantQuery>,
) -> Result<Json<AiStatusResponse>, (StatusCode, Json<AiErrorResponse>)> {
    let service = AiService::new(state.store.clone());

    // Determine target tenant: only SuperAdmin can query other tenants
    let target_tenant_id = if auth.role == "SuperAdmin" {
        query.tenant_id.unwrap_or(auth.tenant_id)
    } else {
        auth.tenant_id
    };

    let settings = service
        .get_settings(target_tenant_id)
        .await
        .map_err(ai_error_response)?;

    // Check if user's role has access
    let has_access = settings.allowed_roles.iter().any(|r| r.eq_ignore_ascii_case(&auth.role));

    Ok(Json(AiStatusResponse {
        enabled: settings.enabled,
        has_access,
        provider: if settings.enabled {
            Some(settings.provider)
        } else {
            None
        },
    }))
}

#[derive(Debug, Serialize)]
pub struct AiStatusResponse {
    pub enabled: bool,
    pub has_access: bool,
    pub provider: Option<String>,
}

/// Get AI settings (Admin or SuperAdmin)
/// - Admin can only view their own tenant's settings
/// - SuperAdmin can view any tenant's settings via tenant_id query param
pub async fn get_ai_settings(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    Query(query): Query<TenantQuery>,
) -> Result<Json<TenantAiSettingsResponse>, (StatusCode, Json<AiErrorResponse>)> {
    // Admin or SuperAdmin only
    if auth.role != "Admin" && auth.role != "SuperAdmin" {
        return Err(ai_error_response(AiError::Forbidden));
    }

    // Determine which tenant to get settings for
    let target_tenant_id = if let Some(tid) = query.tenant_id {
        // Only SuperAdmin can view other tenants' settings
        if auth.role != "SuperAdmin" && tid != auth.tenant_id {
            return Err(ai_error_response(AiError::Forbidden));
        }
        tid
    } else {
        auth.tenant_id
    };

    let service = AiService::new(state.store.clone());
    let settings = service
        .get_settings(target_tenant_id)
        .await
        .map_err(ai_error_response)?;

    Ok(Json(TenantAiSettingsResponse::from(settings)))
}

/// Update AI settings (Admin or SuperAdmin)
/// - Admin can only update their own tenant's settings
/// - SuperAdmin can update any tenant's settings via tenant_id in body
pub async fn update_ai_settings(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    Json(input): Json<UpdateAiSettingsInput>,
) -> Result<Json<TenantAiSettingsResponse>, (StatusCode, Json<AiErrorResponse>)> {
    // Admin or SuperAdmin only
    if auth.role != "Admin" && auth.role != "SuperAdmin" {
        return Err(ai_error_response(AiError::Forbidden));
    }

    // Determine which tenant to update settings for
    let target_tenant_id = if let Some(tid) = input.tenant_id {
        // Only SuperAdmin can update other tenants' settings
        if auth.role != "SuperAdmin" && tid != auth.tenant_id {
            return Err(ai_error_response(AiError::Forbidden));
        }
        tid
    } else {
        auth.tenant_id
    };

    let service = AiService::new(state.store.clone());
    let settings = service
        .update_settings(target_tenant_id, input)
        .await
        .map_err(ai_error_response)?;

    log::info!(
        "AI settings updated (tenant_id: {}, user_id: {})",
        target_tenant_id,
        auth.user_id
    );

    Ok(Json(TenantAiSettingsResponse::from(settings)))
}

/// Test AI provider connection (Admin or SuperAdmin)
/// - Admin can only test their own tenant's connection
/// - SuperAdmin can test any tenant's connection via tenant_id query param
pub async fn test_ai_connection(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    Query(query): Query<TenantQuery>,
) -> Result<Json<TestConnectionResponse>, (StatusCode, Json<AiErrorResponse>)> {
    // Admin or SuperAdmin only
    if auth.role != "Admin" && auth.role != "SuperAdmin" {
        return Err(ai_error_response(AiError::Forbidden));
    }

    // Determine which tenant to test connection for
    let target_tenant_id = if let Some(tid) = query.tenant_id {
        // Only SuperAdmin can test other tenants' connections
        if auth.role != "SuperAdmin" && tid != auth.tenant_id {
            return Err(ai_error_response(AiError::Forbidden));
        }
        tid
    } else {
        auth.tenant_id
    };

    let service = AiService::new(state.store.clone());
    let success = service
        .test_connection(target_tenant_id)
        .await
        .map_err(ai_error_response)?;

    Ok(Json(TestConnectionResponse { success }))
}

#[derive(Debug, Serialize)]
pub struct TestConnectionResponse {
    pub success: bool,
}

/// Get AI usage statistics with pagination (Admin or SuperAdmin)
/// - Admin can only view their own tenant's usage
/// - SuperAdmin can view any tenant's usage via tenant_id query param
pub async fn get_ai_usage(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    Query(query): Query<UsageQuery>,
) -> Result<Json<UsageStats>, (StatusCode, Json<AiErrorResponse>)> {
    // Admin or SuperAdmin only
    if auth.role != "Admin" && auth.role != "SuperAdmin" {
        return Err(ai_error_response(AiError::Forbidden));
    }

    // Determine which tenant to get usage for
    let target_tenant_id = if let Some(tid) = query.tenant_id {
        // Only SuperAdmin can view other tenants' usage
        if auth.role != "SuperAdmin" && tid != auth.tenant_id {
            return Err(ai_error_response(AiError::Forbidden));
        }
        tid
    } else {
        auth.tenant_id
    };

    // Pagination defaults
    let page = query.page.unwrap_or(1).max(1);
    let per_page = query.per_page.unwrap_or(10).clamp(1, 100);

    let service = AiService::new(state.store.clone());
    let stats = service
        .get_usage_stats(target_tenant_id, page, per_page)
        .await
        .map_err(ai_error_response)?;

    Ok(Json(stats))
}

/// Summarize a file (with caching to avoid repeated API calls)
/// Cache is served even during maintenance mode to avoid re-calling the API
pub async fn summarize_file(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    Json(request): Json<SummarizeRequest>,
) -> Result<Json<AiActionResponse>, (StatusCode, Json<AiErrorResponse>)> {
    // Get file info for logging
    let file_info = state
        .store
        .files()
        .by_tenant_id(auth.tenant_id, request.file_id)
        .await
        .ok()
        .flatten();
    let file_name = file_info.map(|f| f.name);

    // Get file content (includes permission check)
    let content = get_file_content(
        &state,
        auth.tenant_id,
        request.file_id,
        auth.user_id,
        &auth.role,
    )
    .await
    .map_err(ai_error_response)?;

    // Calculate content hash to detect changes
    use sha2::{Digest, Sha256};
    let lang_key = request.language.as_deref().unwrap_or("auto");
    let content_hash = format!("{:x}:{}", Sha256::digest(content.as_bytes()), lang_key);

    // Check for cached summary BEFORE maintenance mode check
    // This allows returning cached summaries even during maintenance
    let cached = state
        .store
        .ai()
        .get_file_summary(request.file_id, auth.tenant_id)
        .await
        .map_err(|e| ai_error_response(AiError::DatabaseError(e.to_string())))?;

    // Return cached summary if content hasn't changed
    if let Some(cache) = cached {
        if cache.content_hash == content_hash {
            // Log the view even for cached summaries
            let _ = state
                .store
                .audit()
                .log(
                    auth.tenant_id,
                    Some(auth.user_id),
                    "ai_summary_viewed",
                    "file",
                    Some(request.file_id),
                    Some(serde_json::json!({
                        "file_name": file_name,
                        "cached": true,
                    })),
                    auth.ip_address.clone(),
                )
                .await;

            return Ok(Json(AiActionResponse {
                success: true,
                content: Some(cache.summary),
                error: None,
                tokens_used: Some(0), // Cached, no tokens used
            }));
        }
    }

    // No valid cache - check maintenance mode before making new API call
    let service = AiService::new(state.store.clone());
    let (is_maintenance, maintenance_msg) = service
        .is_maintenance_mode(auth.tenant_id)
        .await
        .map_err(ai_error_response)?;

    if is_maintenance {
        let msg = maintenance_msg.unwrap_or_else(|| {
            "AI features are temporarily unavailable for maintenance. Please try again later."
                .to_string()
        });
        return Err(ai_error_response(AiError::MaintenanceMode(msg)));
    }

    // Call AI
    let response = service
        .summarize(
            auth.tenant_id,
            auth.user_id,
            &auth.role,
            request.file_id,
            &content,
            request.max_length,
            request.language.as_deref(),
        )
        .await
        .map_err(ai_error_response)?;

    // Cache the new summary
    if response.success {
        if let Some(ref summary) = response.content {
            let _ = state
                .store
                .ai()
                .upsert_file_summary(
                    request.file_id,
                    auth.tenant_id,
                    summary.clone(),
                    content_hash.clone(),
                )
                .await;
        }

        // Log to main audit_logs table
        let _ = state
            .store
            .audit()
            .log(
                auth.tenant_id,
                Some(auth.user_id),
                "ai_summarize",
                "file",
                Some(request.file_id),
                Some(serde_json::json!({
                    "file_name": file_name,
                    "tokens_used": response.tokens_used,
                })),
                auth.ip_address.clone(),
            )
            .await;
    }

    Ok(Json(response))
}

/// Answer a question about a file
pub async fn answer_question(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    Json(request): Json<AnswerRequest>,
) -> Result<Json<AiActionResponse>, (StatusCode, Json<AiErrorResponse>)> {
    // Get file info for logging
    let file_info = state
        .store
        .files()
        .by_tenant_id(auth.tenant_id, request.file_id)
        .await
        .ok()
        .flatten();
    let file_name = file_info.map(|f| f.name);

    // Get file content (includes permission check)
    let content = get_file_content(
        &state,
        auth.tenant_id,
        request.file_id,
        auth.user_id,
        &auth.role,
    )
    .await
    .map_err(ai_error_response)?;

    let service = AiService::new(state.store.clone());
    let response = service
        .answer(
            auth.tenant_id,
            auth.user_id,
            &auth.role,
            request.file_id,
            &content,
            &request.question,
            request.language.as_deref(),
        )
        .await
        .map_err(ai_error_response)?;

    // Log to main audit_logs table (without the question content for privacy)
    if response.success {
        let _ = state
            .store
            .audit()
            .log(
                auth.tenant_id,
                Some(auth.user_id),
                "ai_answer",
                "file",
                Some(request.file_id),
                Some(serde_json::json!({
                    "file_name": file_name,
                    "tokens_used": response.tokens_used,
                })),
                auth.ip_address.clone(),
            )
            .await;
    }

    Ok(Json(response))
}

/// Semantic search across files
pub async fn semantic_search(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthUser>,
    Json(_request): Json<SearchRequest>,
) -> Result<Json<SemanticSearchResponse>, (StatusCode, Json<AiErrorResponse>)> {
    // For now, return a placeholder - full implementation requires embeddings index
    // This would need to:
    // 1. Embed the query
    // 2. Search file_embeddings table using vector similarity
    // 3. Return matching files

    let service = AiService::new(state.store.clone());

    // Check if user has access first
    let settings = service
        .get_settings(auth.tenant_id)
        .await
        .map_err(ai_error_response)?;
    if !settings.enabled {
        return Err(ai_error_response(AiError::Disabled));
    }
    if !settings.allowed_roles.iter().any(|r| r.eq_ignore_ascii_case(&auth.role)) {
        return Err(ai_error_response(AiError::Forbidden));
    }

    // Placeholder: return empty results
    // Full implementation would query file_embeddings with vector similarity
    Ok(Json(SemanticSearchResponse {
        results: vec![],
        message: Some("Semantic search is available once files have been indexed. Use the 'Summarize' feature on files to build the search index.".to_string()),
    }))
}

#[derive(Debug, Serialize)]
pub struct SemanticSearchResponse {
    pub results: Vec<SearchResultItem>,
    pub message: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SearchResultItem {
    pub file_id: String,
    pub file_name: String,
    pub file_path: Option<String>,
    pub score: f32,
    pub snippet: Option<String>,
}

/// Get available AI providers
pub async fn get_providers() -> Json<ProvidersResponse> {
    Json(ProvidersResponse {
        providers: vec![
            ProviderInfo {
                id: "openai".to_string(),
                name: "OpenAI".to_string(),
                hipaa_approved: false,
                models: vec!["gpt-4o-mini".to_string(), "gpt-4o".to_string()],
            },
            ProviderInfo {
                id: "deepseek".to_string(),
                name: "DeepSeek".to_string(),
                hipaa_approved: false,
                models: vec![
                    "deepseek-chat".to_string(),
                    "deepseek-reasoner".to_string(),
                ],
            },
            ProviderInfo {
                id: "anthropic".to_string(),
                name: "Anthropic".to_string(),
                hipaa_approved: false,
                models: vec![
                    "claude-3-haiku".to_string(),
                    "claude-3-sonnet".to_string(),
                    "claude-3-opus".to_string(),
                ],
            },
            ProviderInfo {
                id: "google".to_string(),
                name: "Google".to_string(),
                hipaa_approved: false,
                models: vec!["gemini-1.5-flash".to_string(), "gemini-1.5-pro".to_string()],
            },
            ProviderInfo {
                id: "azure".to_string(),
                name: "Azure OpenAI".to_string(),
                hipaa_approved: false,
                models: vec!["gpt-4o-mini".to_string(), "gpt-4o".to_string()],
            },
            ProviderInfo {
                id: "mistral".to_string(),
                name: "Mistral AI".to_string(),
                hipaa_approved: false,
                models: vec!["mistral-small".to_string(), "mistral-large".to_string()],
            },
            ProviderInfo {
                id: "cohere".to_string(),
                name: "Cohere".to_string(),
                hipaa_approved: false,
                models: vec!["command-r".to_string(), "command-r-plus".to_string()],
            },
            ProviderInfo {
                id: "custom".to_string(),
                name: "Self-Hosted / Custom".to_string(),
                hipaa_approved: false,
                models: vec!["custom".to_string()],
            },
        ],
    })
}

#[derive(Debug, Serialize)]
pub struct ProvidersResponse {
    pub providers: Vec<ProviderInfo>,
}

#[derive(Debug, Serialize)]
pub struct ProviderInfo {
    pub id: String,
    pub name: String,
    pub hipaa_approved: bool,
    pub models: Vec<String>,
}

/// Helper: Get file content from storage
/// Includes security check to ensure user has permission to access the file
async fn get_file_content(
    state: &AppState,
    tenant_id: Uuid,
    file_id: Uuid,
    user_id: Uuid,
    user_role: &str,
) -> Result<String, AiError> {
    // SECURITY: Verify user has permission to access this file
    // This checks: Admin bypass, file locks, private file ownership, department membership
    let has_access = crate::handlers::can_access_file(
        &state.store,
        file_id,
        tenant_id,
        user_id,
        user_role,
        "read",
    )
    .await
    .map_err(|_| AiError::Forbidden)?;

    if !has_access {
        log::warn!(
            "AI access denied: user {} attempted to access file {} without permission",
            user_id,
            file_id
        );
        return Err(AiError::Forbidden);
    }

    // Get file metadata
    let file = state
        .store
        .files()
        .by_tenant_id(tenant_id, file_id)
        .await
        .map_err(|e| AiError::DatabaseError(e.to_string()))?
        .filter(|f| !f.is_deleted)
        .ok_or(AiError::FileNotFound)?;

    // Check if format is supported for text extraction
    let mut mime = file
        .content_type
        .as_deref()
        .unwrap_or("application/octet-stream")
        .to_string();

    let base_mime = mime.split(';').next().unwrap_or("").trim().to_ascii_lowercase();
    if base_mime == "application/octet-stream" || base_mime.is_empty() || !crate::text_extract::is_extractable(&mime) {
        let ext = file.name.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
        let inferred = match ext.as_str() {
            "md" | "markdown" => Some("text/markdown"),
            "txt" | "log" | "ini" | "conf" => Some("text/plain"),
            "json" => Some("application/json"),
            "xml" => Some("application/xml"),
            "csv" => Some("text/csv"),
            "tsv" => Some("text/tab-separated-values"),
            "pdf" => Some("application/pdf"),
            "docx" => Some("application/vnd.openxmlformats-officedocument.wordprocessingml.document"),
            "xlsx" => Some("application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"),
            "pptx" => Some("application/vnd.openxmlformats-officedocument.presentationml.presentation"),
            "py" | "rs" | "js" | "ts" | "jsx" | "tsx" | "html" | "css" | "sql" | "sh" | "yaml" | "yml" => Some("text/plain"),
            _ => None,
        };
        if let Some(inf) = inferred {
            mime = inf.to_string();
        }
    }

    if !crate::text_extract::is_extractable(&mime) {
        log::warn!("Unsupported file format for AI text extraction: file={}, mime={}", file.name, mime);
        return Err(AiError::ContentExtractionFailed);
    }

    // Download content
    let bytes = state
        .storage
        .download(&file.storage_path)
        .await
        .map_err(|e| {
            log::error!("Failed to download file for AI: {:?}", e);
            AiError::ContentExtractionFailed
        })?;

    if file.size_bytes == 0 || bytes.is_empty() {
        log::warn!("File {} ({}) is empty (0 bytes), cannot process with AI", file.name, file_id);
        return Err(AiError::FileContentEmpty);
    }

    // Extract text based on file type (PDF, Office docs, plain text, etc.)
    let content = crate::text_extract::extract_text(&bytes, &mime).map_err(|e| {
        log::warn!("Text extraction failed for {} ({}): {}", file.name, mime, e);
        AiError::ContentExtractionFailed
    })?;

    if content.trim().is_empty() {
        log::warn!("Extracted text content is empty for {} ({})", file.name, file_id);
        return Err(AiError::FileContentEmpty);
    }

    // Limit content size (max 100KB for AI processing)
    const MAX_CONTENT_SIZE: usize = 100 * 1024;
    if content.len() > MAX_CONTENT_SIZE {
        Ok(content.chars().take(MAX_CONTENT_SIZE).collect())
    } else {
        Ok(content)
    }
}

