//! AI Service Layer
//!
//! Orchestrates AI operations with all required guards:
//! - Tenant enablement check
//! - API key validation
//! - Role-based access
//! - Compliance enforcement (HIPAA/SOX)
//! - Usage limits
//! - PII redaction
//! - Audit logging

use crate::error::AiError;
use crate::models::*;
use crate::provider::{AiProvider, ProviderRegistry};
use crate::redact::RedactionService;
use clovalink_entity::{
    entities::tenant_ai_settings,
    repositories::{AiSettingsPatch, NewAiUsage},
    DataStore,
};
use uuid::Uuid;

/// AI Service - main entry point for AI operations
pub struct AiService {
    store: DataStore,
}

impl AiService {
    pub fn new(store: DataStore) -> Self {
        Self { store }
    }

    fn settings_from_entity(model: tenant_ai_settings::Model) -> TenantAiSettings {
        TenantAiSettings {
            tenant_id: model.tenant_id,
            enabled: model.enabled,
            provider: model.provider,
            api_key_encrypted: model.api_key_encrypted,
            allowed_roles: model.allowed_roles,
            hipaa_approved_only: model.hipaa_approved_only,
            sox_read_only: model.sox_read_only,
            monthly_token_limit: model.monthly_token_limit,
            daily_request_limit: model.daily_request_limit,
            tokens_used_this_month: model.tokens_used_this_month,
            requests_today: model.requests_today,
            last_usage_reset: model.last_usage_reset,
            maintenance_mode: model.maintenance_mode,
            maintenance_message: model.maintenance_message,
            custom_endpoint: model.custom_endpoint,
            custom_model: model.custom_model,
            created_at: model.created_at.with_timezone(&chrono::Utc),
            updated_at: model.updated_at.with_timezone(&chrono::Utc),
        }
    }

    /// Get or create tenant AI settings
    pub async fn get_settings(&self, tenant_id: Uuid) -> Result<TenantAiSettings, AiError> {
        self.store
            .ai()
            .get_or_create_settings(tenant_id)
            .await
            .map(Self::settings_from_entity)
            .map_err(|e| AiError::DatabaseError(e.to_string()))
    }

    /// Update tenant AI settings (SuperAdmin only - checked at handler level)
    pub async fn update_settings(
        &self,
        tenant_id: Uuid,
        input: UpdateAiSettingsInput,
    ) -> Result<TenantAiSettings, AiError> {
        let patch = AiSettingsPatch {
            enabled: input.enabled,
            provider: input.provider,
            api_key: input.api_key,
            allowed_roles: input.allowed_roles,
            hipaa_approved_only: input.hipaa_approved_only,
            sox_read_only: input.sox_read_only,
            monthly_token_limit: input.monthly_token_limit,
            daily_request_limit: input.daily_request_limit,
            maintenance_mode: input.maintenance_mode,
            maintenance_message: input.maintenance_message,
            custom_endpoint: input.custom_endpoint,
            custom_model: input.custom_model,
        };
        self.store
            .ai()
            .update_settings(tenant_id, patch)
            .await
            .map(Self::settings_from_entity)
            .map_err(|e| AiError::DatabaseError(e.to_string()))
    }

    /// Run all pre-flight checks before an AI operation
    async fn preflight_check(
        &self,
        tenant_id: Uuid,
        _user_id: Uuid,
        user_role: &str,
        action: &str,
    ) -> Result<(TenantAiSettings, Box<dyn AiProvider>), AiError> {
        let settings = self.get_settings(tenant_id).await?;

        // 1. Check if AI is enabled
        if !settings.enabled {
            return Err(AiError::Disabled);
        }

        // 2. Check maintenance mode
        if settings.maintenance_mode {
            let msg = settings.maintenance_message.clone()
                .unwrap_or_else(|| "AI features are temporarily unavailable for maintenance. Please try again later.".to_string());
            return Err(AiError::MaintenanceMode(msg));
        }

        // 3. Check API key exists
        let api_key = settings
            .api_key_encrypted
            .as_ref()
            .filter(|k| !k.is_empty())
            .ok_or(AiError::NoApiKey)?;

        // 4. Check user role is allowed
        if !settings.allowed_roles.iter().any(|r| r == user_role) {
            return Err(AiError::Forbidden);
        }

        // 5. Get provider
        let provider = ProviderRegistry::get(&settings.provider, api_key).ok_or_else(|| {
            AiError::ProviderError(format!("Unknown provider: {}", settings.provider))
        })?;

        // 6. HIPAA compliance check
        if settings.hipaa_approved_only && !provider.is_hipaa_approved() {
            return Err(AiError::HipaaNotApproved);
        }

        // 7. SOX compliance check (read-only mode)
        if settings.sox_read_only && (action == "summarize" || action == "answer") {
            return Err(AiError::SoxReadOnly);
        }

        // 8. Reset daily counter if new day
        self.maybe_reset_daily_counter(tenant_id, &settings).await?;

        // 9. Check usage limits
        if settings.requests_today >= settings.daily_request_limit {
            return Err(AiError::DailyLimitExceeded);
        }
        if settings.tokens_used_this_month >= settings.monthly_token_limit {
            return Err(AiError::MonthlyLimitExceeded);
        }

        Ok((settings, provider))
    }

    /// Check if maintenance mode is active (for handlers that need to check before using cache)
    pub async fn is_maintenance_mode(
        &self,
        tenant_id: Uuid,
    ) -> Result<(bool, Option<String>), AiError> {
        let settings = self.get_settings(tenant_id).await?;
        Ok((settings.maintenance_mode, settings.maintenance_message))
    }

    async fn maybe_reset_daily_counter(
        &self,
        tenant_id: Uuid,
        settings: &TenantAiSettings,
    ) -> Result<(), AiError> {
        let today = chrono::Utc::now().date_naive();
        if settings.last_usage_reset.map(|d| d < today).unwrap_or(true) {
            self.store
                .ai()
                .reset_daily(tenant_id, today)
                .await
                .map_err(|e| AiError::DatabaseError(e.to_string()))?;
        }
        Ok(())
    }

    /// Log AI usage (without content)
    pub async fn log_usage(
        &self,
        tenant_id: Uuid,
        user_id: Uuid,
        file_id: Option<Uuid>,
        file_name: Option<&str>,
        action: &str,
        provider: &str,
        model: Option<&str>,
        tokens_used: i32,
        status: &str,
        error_message: Option<&str>,
    ) -> Result<(), AiError> {
        self.store
            .ai()
            .log_usage(NewAiUsage {
                tenant_id,
                user_id,
                file_id,
                file_name,
                action,
                provider,
                model,
                tokens_used,
                status,
                error_message,
            })
            .await
            .map_err(|e| AiError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    /// Update usage counters
    async fn update_usage_counters(&self, tenant_id: Uuid, tokens: i32) -> Result<(), AiError> {
        self.store
            .ai()
            .increment_usage(tenant_id, tokens)
            .await
            .map_err(|e| AiError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    /// Summarize file content
    pub async fn summarize(
        &self,
        tenant_id: Uuid,
        user_id: Uuid,
        user_role: &str,
        file_id: Uuid,
        content: &str,
        max_tokens: Option<u32>,
    ) -> Result<AiActionResponse, AiError> {
        let (settings, provider) = self
            .preflight_check(tenant_id, user_id, user_role, "summarize")
            .await?;

        // Redact PII
        let redacted_content = RedactionService::redact(content);

        // Chunk if too long (max ~4000 tokens for context)
        let chunks = RedactionService::chunk_text(&redacted_content, 4000);
        let chunk_to_summarize = chunks.first().cloned().unwrap_or_default();

        match provider
            .summarize(&chunk_to_summarize, max_tokens.unwrap_or(500))
            .await
        {
            Ok(response) => {
                self.update_usage_counters(tenant_id, response.tokens_used as i32)
                    .await?;
                self.log_usage(
                    tenant_id,
                    user_id,
                    Some(file_id),
                    None,
                    "summarize",
                    &settings.provider,
                    Some(&response.model),
                    response.tokens_used as i32,
                    "success",
                    None,
                )
                .await?;

                Ok(AiActionResponse {
                    success: true,
                    content: Some(response.content),
                    error: None,
                    tokens_used: Some(response.tokens_used),
                })
            }
            Err(e) => {
                self.log_usage(
                    tenant_id,
                    user_id,
                    Some(file_id),
                    None,
                    "summarize",
                    &settings.provider,
                    None,
                    0,
                    "error",
                    Some(&e.to_string()),
                )
                .await?;

                Err(e)
            }
        }
    }

    /// Answer question about file content
    pub async fn answer(
        &self,
        tenant_id: Uuid,
        user_id: Uuid,
        user_role: &str,
        file_id: Uuid,
        content: &str,
        question: &str,
    ) -> Result<AiActionResponse, AiError> {
        let (settings, provider) = self
            .preflight_check(tenant_id, user_id, user_role, "answer")
            .await?;

        // Redact PII from both content and question
        let redacted_content = RedactionService::redact(content);
        let redacted_question = RedactionService::redact(question);

        // Use only first chunk as context
        let chunks = RedactionService::chunk_text(&redacted_content, 3000);
        let context = chunks.first().cloned().unwrap_or_default();

        match provider.answer(&redacted_question, &context).await {
            Ok(response) => {
                self.update_usage_counters(tenant_id, response.tokens_used as i32)
                    .await?;
                self.log_usage(
                    tenant_id,
                    user_id,
                    Some(file_id),
                    None,
                    "answer",
                    &settings.provider,
                    Some(&response.model),
                    response.tokens_used as i32,
                    "success",
                    None,
                )
                .await?;

                Ok(AiActionResponse {
                    success: true,
                    content: Some(response.content),
                    error: None,
                    tokens_used: Some(response.tokens_used),
                })
            }
            Err(e) => {
                self.log_usage(
                    tenant_id,
                    user_id,
                    Some(file_id),
                    None,
                    "answer",
                    &settings.provider,
                    None,
                    0,
                    "error",
                    Some(&e.to_string()),
                )
                .await?;

                Err(e)
            }
        }
    }

    /// Test provider connection
    pub async fn test_connection(&self, tenant_id: Uuid) -> Result<bool, AiError> {
        let settings = self.get_settings(tenant_id).await?;

        let api_key = settings
            .api_key_encrypted
            .as_ref()
            .filter(|k| !k.is_empty())
            .ok_or(AiError::NoApiKey)?;

        let provider = ProviderRegistry::get(&settings.provider, api_key).ok_or_else(|| {
            AiError::ProviderError(format!("Unknown provider: {}", settings.provider))
        })?;

        provider.test_connection().await
    }

    /// Get usage statistics with pagination
    pub async fn get_usage_stats(
        &self,
        tenant_id: Uuid,
        page: i32,
        per_page: i32,
    ) -> Result<UsageStats, AiError> {
        let settings = self.get_settings(tenant_id).await?;

        let offset = (page - 1) * per_page;
        let usage = self
            .store
            .ai()
            .usage_page(tenant_id, offset.max(0) as u64, per_page.max(1) as u64)
            .await
            .map_err(|e| AiError::DatabaseError(e.to_string()))?;
        let total_pages = ((usage.total as f64) / (per_page as f64)).ceil() as i32;
        let mut recent = Vec::with_capacity(usage.rows.len());
        for (row, user_name) in usage.rows {
            recent.push(AiUsageLogWithUser {
                id: row.id,
                tenant_id: row.tenant_id,
                user_id: row.user_id,
                file_id: row.file_id,
                action: row.action,
                provider: row.provider,
                model: row.model,
                tokens_used: row.tokens_used,
                status: row.status,
                error_message: row.error_message,
                file_name: row.file_name,
                created_at: row.created_at.with_timezone(&chrono::Utc),
                user_name,
            });
        }

        Ok(UsageStats {
            tokens_used_today: usage.tokens_today,
            tokens_used_this_month: settings.tokens_used_this_month,
            requests_today: settings.requests_today,
            monthly_token_limit: settings.monthly_token_limit,
            daily_request_limit: settings.daily_request_limit,
            recent_actions: recent
                .into_iter()
                .map(|log| UsageLogSummary {
                    action: log.action,
                    tokens_used: log.tokens_used,
                    status: log.status,
                    created_at: log.created_at,
                    user_name: log.user_name,
                    file_name: log.file_name,
                })
                .collect(),
            total_count: usage.total as i64,
            page,
            per_page,
            total_pages,
        })
    }
}
