use crate::{
    api::{
        ai, api_usage, app_manage, approvals, audit, auth, comments, compliance, config_manage,
        cron, dashboard, departments, discord, email_templates, file_requests, gitlab,
        global_settings, groups, handlers, health, notifications, oidc, roles, saml, search,
        security, settings, settings_backup, sharing, sso_mappings, tenants, users, virus_scan,
    },
    AppState,
};

use axum::{
    routing::{delete, get, post, put},
    Router,
};
use std::sync::Arc;

/// 构建所有需要身份认证的受保护业务路由
///
/// 本函数集合了核心业务 API，并在最外层挂载数据库级权限认证中间件 `auth_middleware_with_db`：
/// 每次请求均从数据库实时验证当前 JWT Token 对应用户的启用状态（防已封禁用户携带有效 Token 访问）。
///
/// 包含以下业务领域：
/// - 当前用户个人中心、2FA 设置、会话管理、个人资料与密码修改
/// - 系统管理与运维端点（健康详情、版本信息、存储同步、哈希迁移）
/// - API 使用量分析与性能监控统计（仅超级管理员）
/// - 病毒防护与扫描隔离管理（ClamAV 状态与隔离区文件操作）
/// - 全局搜索与文件请求收集管理
/// - 文件审批流与审批策略管理
/// - 用户管理（用户列表、创建、编辑、封禁与解封、重置密码等）
/// - 多租户/公司管理（创建公司、切换租户、SMTP 测试、公司停用等）
/// - 部门管理与角色权限体系（RBAC 角色与权限矩阵）
/// - 租户级系统设置（密码策略、黑名单扩展名、IP 访问限制、合规配置）
/// - 全局平台设置（平台 Logo、Favicon、全局参数等，仅超级管理员）
/// - 配置备份与恢复（导出/导入、配置预检、自动备份策略）
/// - 邮件通知模板管理（全局模板与租户定制模板）
/// - 数据看板与统计报表
/// - S3 异地多活数据复制管理
/// - 消息通知中心与租户通知渠道设置
/// - 合规、GDPR 数据清除与安全预警事件
/// - 操作审计日志与导出
/// - 文件全生命周期管理（上传、下载、预览、重命名、移动、复制、星标、分享等）
/// - 文件评论与协作系统
/// - 用户文件定向共享与文件组管理
/// - 回收站管理与文件恢复/彻底清除
/// - 定时后台清理任务（Cron）
/// - AI 智能增强特性（文本提取、智能摘要、智能问答、语义搜索）
/// - Discord OAuth 绑定与 DM 机器人通知
/// - OIDC 与 SAML 身份提供商企业级管理与属性映射
pub(super) fn build_protect_routes(app_state: &Arc<AppState>) -> Router {
    Router::new()
        .route("/api/auth/me", get(auth::me))
        .route("/api/auth/2fa/setup", post(auth::setup_2fa))
        .route("/api/auth/2fa/verify", post(auth::verify_2fa))
        .route("/api/users/me/export", get(users::export_data))
        .route("/api/users/me/profile", put(users::update_my_profile))
        .route("/api/users/me/password", put(users::change_password))
        .route("/api/users/me/avatar", post(users::upload_avatar))
        .route("/api/users/me/sessions", get(users::list_sessions))
        .route("/api/users/me/sessions/{id}", delete(users::revoke_session))
        .route(
            "/api/users/me/preferences",
            get(users::get_preferences).put(users::update_preferences),
        )
        // Admin endpoints
        .route(
            "/api/admin/migrate-content-hashes",
            post(handlers::migrate_content_hashes),
        )
        .route("/api/admin/health", get(health::detailed_health))
        .route("/api/admin/version", get(health::get_version_info))
        .route("/api/admin/storage/sync", post(health::sync_storage))
        // API Usage / Performance Monitoring (SuperAdmin only)
        .route(
            "/api/admin/usage/summary",
            get(api_usage::get_usage_summary),
        )
        .route(
            "/api/admin/usage/by-tenant",
            get(api_usage::get_usage_by_tenant),
        )
        .route(
            "/api/admin/usage/by-user",
            get(api_usage::get_usage_by_user),
        )
        .route(
            "/api/admin/usage/by-endpoint",
            get(api_usage::get_usage_by_endpoint),
        )
        .route(
            "/api/admin/usage/slow-requests",
            get(api_usage::get_slow_requests),
        )
        .route(
            "/api/admin/usage/timeseries",
            get(api_usage::get_usage_timeseries),
        )
        .route("/api/admin/usage/errors", get(api_usage::get_recent_errors))
        .route(
            "/api/admin/usage/error-summary",
            get(api_usage::get_error_summary),
        )
        .route(
            "/api/admin/usage/aggregate",
            post(api_usage::aggregate_hourly_stats),
        )
        .route(
            "/api/admin/usage/cleanup",
            post(api_usage::cleanup_old_usage),
        )
        // Virus Scanning
        .route(
            "/api/admin/virus-scan/settings",
            get(virus_scan::get_settings).put(virus_scan::update_settings),
        )
        .route(
            "/api/admin/virus-scan/metrics",
            get(virus_scan::get_metrics),
        )
        .route(
            "/api/admin/virus-scan/results",
            get(virus_scan::get_scan_results),
        )
        .route(
            "/api/admin/virus-scan/quarantine",
            get(virus_scan::get_quarantined_files),
        )
        .route(
            "/api/admin/virus-scan/quarantine/{id}",
            delete(virus_scan::delete_quarantined_file),
        )
        .route(
            "/api/admin/virus-scan/rescan/{file_id}",
            post(virus_scan::rescan_file),
        )
        .route(
            "/api/admin/virus-scan/config",
            get(virus_scan::get_global_config),
        )
        // Global Search
        .route("/api/search", get(search::global_search))
        // File Requests
        .route(
            "/api/file-requests",
            get(file_requests::list_file_requests).post(file_requests::create_file_request),
        )
        .route(
            "/api/file-requests/{id}",
            get(file_requests::get_file_request).delete(file_requests::delete_file_request),
        )
        .route(
            "/api/file-requests/{id}/uploads",
            get(file_requests::get_file_request_uploads),
        )
        .route(
            "/api/file-requests/{id}/permanent",
            delete(file_requests::permanent_delete_file_request),
        )
        // Approvals
        .route(
            "/api/approvals/{company_id}/pending",
            get(approvals::list_pending),
        )
        .route(
            "/api/approvals/{company_id}/history",
            get(approvals::list_history),
        )
        .route(
            "/api/approvals/{company_id}/my-pending",
            get(approvals::list_my_pending),
        )
        .route(
            "/api/approvals/{company_id}/stats",
            get(approvals::get_stats),
        )
        .route(
            "/api/approvals/{company_id}/{request_id}/approve",
            post(approvals::approve_file),
        )
        .route(
            "/api/approvals/{company_id}/{request_id}/reject",
            post(approvals::reject_file),
        )
        .route(
            "/api/approvals/{company_id}/{file_id}/send",
            post(approvals::send_for_approval),
        )
        .route(
            "/api/approvals/{company_id}/{file_id}/resubmit",
            post(approvals::resubmit),
        )
        .route(
            "/api/approvals/{company_id}/policies",
            get(approvals::list_policies).post(approvals::create_policy),
        )
        .route(
            "/api/approvals/{company_id}/policies/{id}",
            put(approvals::update_policy).delete(approvals::delete_policy),
        )
        // Users
        .route(
            "/api/users",
            get(users::list_users).post(users::create_user),
        )
        .route(
            "/api/users/{id}",
            put(users::update_user).delete(users::delete_user),
        )
        .route("/api/users/{id}/suspend", post(users::suspend_user))
        .route("/api/users/{id}/unsuspend", post(users::unsuspend_user))
        .route(
            "/api/users/{id}/suspension",
            get(users::get_suspension_status),
        )
        .route(
            "/api/users/{id}/reset-password",
            post(users::admin_reset_password),
        )
        .route(
            "/api/users/{id}/send-reset-email",
            post(users::send_password_reset_email),
        )
        .route(
            "/api/users/{id}/change-email",
            post(users::admin_change_email),
        )
        .route(
            "/api/users/{id}/permanent",
            delete(users::permanent_delete_user),
        )
        .route(
            "/api/users/{id}/activity-logs",
            get(audit::get_user_activity_logs),
        )
        // Tenants/Companies
        .route(
            "/api/tenants",
            get(tenants::list_tenants).post(tenants::create_tenant),
        )
        // IMPORTANT: specific routes must come before parameterized routes
        .route("/api/tenants/accessible", get(tenants::accessible_tenants))
        .route(
            "/api/tenants/switch/{tenant_id}",
            post(tenants::switch_tenant),
        )
        .route("/api/tenants/{id}/smtp/test", post(tenants::test_smtp))
        .route("/api/tenants/{id}/edit", put(tenants::edit_my_company))
        .route("/api/tenants/{id}/suspend", post(tenants::suspend_tenant))
        .route(
            "/api/tenants/{id}/unsuspend",
            post(tenants::unsuspend_tenant),
        )
        .route(
            "/api/tenants/{id}",
            put(tenants::update_tenant).delete(tenants::delete_tenant),
        )
        // Departments
        .route(
            "/api/departments",
            get(departments::list_departments).post(departments::create_department),
        )
        .route(
            "/api/departments/{id}",
            put(departments::update_department).delete(departments::delete_department),
        )
        // Settings
        .route(
            "/api/settings/compliance",
            get(settings::get_compliance).put(settings::update_compliance),
        )
        .route(
            "/api/settings/blocked-extensions",
            get(settings::get_blocked_extensions).put(settings::update_blocked_extensions),
        )
        .route(
            "/api/settings/password-policy",
            get(settings::get_password_policy).put(settings::update_password_policy),
        )
        .route(
            "/api/settings/ip-restrictions",
            get(settings::get_ip_restrictions).put(settings::update_ip_restrictions),
        )
        // Global Settings (app-wide, SuperAdmin only for updates)
        .route(
            "/api/global-settings",
            get(global_settings::get_global_settings).put(global_settings::update_global_settings),
        )
        .route(
            "/api/global-settings/logo",
            post(global_settings::upload_logo).delete(global_settings::delete_logo),
        )
        .route(
            "/api/global-settings/favicon",
            post(global_settings::upload_favicon).delete(global_settings::delete_favicon),
        )
        // Backup & Restore
        .route(
            "/api/backup/export",
            get(settings_backup::export_tenant_backup),
        )
        .route(
            "/api/backup/import",
            post(settings_backup::import_tenant_backup),
        )
        .route(
            "/api/backup/import/preview",
            post(settings_backup::preview_import),
        )
        .route(
            "/api/backup/apply-profile",
            post(settings_backup::apply_profile),
        )
        .route(
            "/api/backup/apply-settings-profile",
            post(settings_backup::apply_settings_profile),
        )
        .route(
            "/api/backup/global/export",
            get(settings_backup::export_global),
        )
        .route(
            "/api/backup/global/import",
            post(settings_backup::import_global),
        )
        .route(
            "/api/backup/global/import/preview",
            post(settings_backup::preview_global_import),
        )
        .route(
            "/api/backup/global/apply-settings-profile",
            post(settings_backup::apply_global_settings_profile),
        )
        .route(
            "/api/backup/global/toggle",
            put(settings_backup::toggle_global_backup),
        )
        .route(
            "/api/backup/global/status",
            get(settings_backup::global_backup_status),
        )
        .route(
            "/api/backup/global/save",
            post(settings_backup::save_global_backup_to_storage),
        )
        .route(
            "/api/backup/global/schedule",
            get(settings_backup::get_global_backup_schedule)
                .put(settings_backup::set_global_backup_schedule),
        )
        .route(
            "/api/backup/current-settings",
            get(settings_backup::get_current_settings),
        )
        .route(
            "/api/backup/section-counts",
            get(settings_backup::section_counts),
        )
        .route(
            "/api/backup/save",
            post(settings_backup::save_backup_to_storage),
        )
        .route(
            "/api/backup/saved",
            get(settings_backup::list_saved_backups),
        )
        .route(
            "/api/backup/saved/{id}/download",
            get(settings_backup::download_saved_backup),
        )
        .route(
            "/api/backup/saved/{id}",
            delete(settings_backup::delete_saved_backup),
        )
        .route("/api/backup/health", get(settings_backup::backup_health))
        .route("/api/backup/metrics", get(settings_backup::backup_metrics))
        // Global Email Templates (SuperAdmin)
        .route(
            "/api/email-templates",
            get(email_templates::list_global_templates),
        )
        .route(
            "/api/email-templates/{key}",
            get(email_templates::get_global_template).put(email_templates::update_global_template),
        )
        // Tenant Email Templates (Admin)
        .route(
            "/api/settings/email-templates",
            get(email_templates::list_tenant_templates),
        )
        .route(
            "/api/settings/email-templates/{key}",
            get(email_templates::get_tenant_template)
                .put(email_templates::update_tenant_template)
                .delete(email_templates::reset_tenant_template),
        )
        .route(
            "/api/settings/email-templates/{key}/preview",
            post(email_templates::preview_template),
        )
        // Dashboard
        .route("/api/dashboard/stats", get(dashboard::get_dashboard_stats))
        .route("/api/dashboard/file-types", get(dashboard::get_file_types))
        // S3 Replication Admin (SuperAdmin only)
        .route(
            "/api/admin/replication/status",
            get(dashboard::get_replication_status),
        )
        .route(
            "/api/admin/replication/pending",
            get(dashboard::get_replication_jobs),
        )
        .route(
            "/api/admin/replication/retry-failed",
            post(dashboard::retry_failed_jobs),
        )
        // Notifications
        .route("/api/notifications", get(notifications::list_notifications))
        .route(
            "/api/notifications/unread-count",
            get(notifications::get_unread_count),
        )
        .route(
            "/api/notifications/read-all",
            put(notifications::mark_all_as_read),
        )
        .route(
            "/api/notifications/preferences",
            get(notifications::get_preferences).put(notifications::update_preferences),
        )
        .route(
            "/api/notifications/preferences-with-company",
            get(notifications::get_preferences_with_company_settings),
        )
        .route(
            "/api/notifications/preference-labels",
            get(notifications::get_preference_labels),
        )
        .route(
            "/api/notifications/{id}/read",
            put(notifications::mark_as_read),
        )
        .route(
            "/api/notifications/{id}",
            delete(notifications::delete_notification),
        )
        // Tenant Notification Settings
        .route(
            "/api/tenants/{id}/notification-settings",
            get(notifications::get_tenant_notification_settings)
                .put(notifications::update_tenant_notification_settings),
        )
        // Compliance
        .route(
            "/api/compliance/restrictions",
            get(compliance::get_compliance_restrictions),
        )
        // Security Alerts
        .route("/api/security/alerts", get(security::list_alerts))
        .route("/api/security/alerts/stats", get(security::get_alert_stats))
        .route("/api/security/alerts/badge", get(security::get_alert_badge))
        .route(
            "/api/security/alerts/bulk",
            post(security::bulk_alert_action),
        )
        .route(
            "/api/security/alerts/{id}/resolve",
            post(security::resolve_alert),
        )
        .route(
            "/api/security/alerts/{id}/dismiss",
            post(security::dismiss_alert),
        )
        .route("/api/compliance/consent", post(compliance::record_consent))
        .route(
            "/api/compliance/consent/user/{user_id}",
            get(compliance::get_consent_status),
        )
        .route(
            "/api/compliance/consent/revoke/{consent_type}",
            delete(compliance::revoke_consent),
        )
        // GDPR
        .route(
            "/api/gdpr/deletion-request",
            post(compliance::create_deletion_request),
        )
        .route(
            "/api/gdpr/deletion-requests",
            get(compliance::list_deletion_requests),
        )
        .route(
            "/api/gdpr/deletion-requests/{id}/process",
            post(compliance::process_deletion_request),
        )
        // Audit Logs
        .route("/api/activity-logs", get(audit::list_activity_logs))
        .route(
            "/api/activity-logs/export",
            get(audit::export_activity_logs),
        )
        .route("/api/activity-logs/actions", get(audit::get_action_types))
        .route(
            "/api/activity-logs/resource-types",
            get(audit::get_resource_types),
        )
        .route(
            "/api/audit-settings",
            get(audit::get_audit_settings).put(audit::update_audit_settings),
        )
        // Roles
        .route(
            "/api/roles",
            get(roles::list_roles).post(roles::create_role),
        )
        .route(
            "/api/roles/{id}",
            get(roles::get_role)
                .put(roles::update_role)
                .delete(roles::delete_role),
        )
        .route(
            "/api/roles/{id}/permissions",
            get(roles::get_role_permissions_handler).put(roles::update_role_permissions),
        )
        // File Management
        .route(
            "/api/upload/{company_id}",
            post(handlers::upload_file).layer(axum::extract::DefaultBodyLimit::disable()),
        )
        .route("/api/files/{company_id}", get(handlers::list_files))
        .route(
            "/api/files/{company_id}/export",
            get(handlers::export_files),
        )
        .route(
            "/api/download/{company_id}/{file_id}",
            get(handlers::download_file),
        )
        .route(
            "/api/preview/{company_id}/{file_id}",
            get(handlers::preview_office_file),
        )
        .route(
            "/api/files/{company_id}/{file_id}/content",
            put(handlers::update_file_content),
        )
        .route("/api/folders/{company_id}", post(handlers::create_folder))
        .route(
            "/api/files/{company_id}/rename",
            post(handlers::rename_file),
        )
        .route(
            "/api/files/{company_id}/delete",
            post(handlers::delete_file),
        )
        .route(
            "/api/files/{company_id}/{file_id}/lock",
            post(handlers::lock_file),
        )
        .route(
            "/api/files/{company_id}/{file_id}/unlock",
            post(handlers::unlock_file),
        )
        .route(
            "/api/files/{company_id}/{file_id}/move",
            put(handlers::move_file),
        )
        .route(
            "/api/files/{company_id}/{file_id}/copy",
            post(handlers::copy_file),
        )
        .route(
            "/api/files/{company_id}/{file_id}/star",
            post(handlers::toggle_star),
        )
        .route(
            "/api/files/{company_id}/starred",
            get(handlers::get_starred),
        )
        .route(
            "/api/files/{company_id}/{file_id}/activity",
            get(handlers::get_file_activity),
        )
        .route(
            "/api/files/{company_id}/{file_id}/company-folder",
            put(handlers::toggle_company_folder),
        )
        .route(
            "/api/files/{company_id}/{file_id}/share",
            post(handlers::create_file_share),
        )
        .route(
            "/api/files/{company_id}/{file_id}/convert-markdown",
            post(handlers::convert_file_to_markdown),
        )
        // File Comments
        .route(
            "/api/files/{company_id}/{file_id}/comments",
            get(comments::list_comments).post(comments::create_comment),
        )
        .route(
            "/api/files/{company_id}/{file_id}/comments/count",
            get(comments::get_comment_count),
        )
        .route(
            "/api/files/{company_id}/{file_id}/comments/{comment_id}",
            put(comments::update_comment).delete(comments::delete_comment),
        )
        // User-Specific Sharing
        .route(
            "/api/users/{company_id}/shareable",
            get(sharing::list_shareable_users),
        )
        .route("/api/shared-with-me", get(sharing::list_shared_with_me))
        .route("/api/shared-with-me/copy", post(sharing::copy_to_my_files))
        // File Groups
        .route(
            "/api/groups/{company_id}",
            get(groups::list_groups).post(groups::create_group),
        )
        .route(
            "/api/groups/{company_id}/{group_id}",
            put(groups::update_group).delete(groups::delete_group),
        )
        .route(
            "/api/groups/{company_id}/{group_id}/files",
            get(groups::get_group_files),
        )
        .route(
            "/api/groups/{company_id}/{group_id}/move",
            put(groups::move_group_to_folder),
        )
        .route(
            "/api/groups/{company_id}/{group_id}/star",
            post(groups::toggle_group_star),
        )
        .route(
            "/api/groups/{company_id}/{group_id}/lock",
            post(groups::lock_group),
        )
        .route(
            "/api/groups/{company_id}/{group_id}/unlock",
            post(groups::unlock_group),
        )
        .route(
            "/api/files/{company_id}/{file_id}/group",
            post(groups::add_file_to_group).delete(groups::remove_file_from_group),
        )
        .route("/api/trash/{company_id}", get(handlers::list_trash))
        .route(
            "/api/trash/{company_id}/restore/{filename}",
            post(handlers::restore_file),
        )
        .route(
            "/api/trash/{company_id}/delete/{filename}",
            post(handlers::permanent_delete),
        )
        .route(
            "/api/prefs/{company_id}",
            get(handlers::get_prefs).post(handlers::update_prefs),
        )
        // Cron Jobs
        .route("/api/cron/cleanup", post(cron::cleanup_expired_files))
        .route(
            "/api/cron/expiring-requests",
            post(cron::notify_expiring_requests),
        )
        .route(
            "/api/cron/storage-warnings",
            post(cron::check_storage_quotas),
        )
        // AI Features (per-tenant, role-based)
        .route("/api/ai/status", get(ai::get_ai_status))
        .route(
            "/api/ai/settings",
            get(ai::get_ai_settings).put(ai::update_ai_settings),
        )
        .route("/api/ai/test", post(ai::test_ai_connection))
        .route("/api/ai/usage", get(ai::get_ai_usage))
        .route("/api/ai/summarize", post(ai::summarize_file))
        .route("/api/ai/answer", post(ai::answer_question))
        .route("/api/ai/search", post(ai::semantic_search))
        .route("/api/ai/providers", get(ai::get_providers))
        // Discord OAuth & DM Notifications
        .route("/api/discord/settings", get(discord::get_discord_settings))
        .route(
            "/api/discord/settings/update",
            post(discord::update_discord_settings),
        )
        .route("/api/discord/status", get(discord::get_connection_status))
        .route("/api/discord/connect", get(discord::start_oauth))
        .route("/api/discord/callback", get(discord::oauth_callback))
        .route("/api/discord/disconnect", post(discord::disconnect))
        .route(
            "/api/discord/preferences",
            post(discord::update_preferences),
        )
        .route("/api/discord/test", post(discord::test_connection))
        // OIDC SSO Provider Management (SuperAdmin)
        .route(
            "/api/oidc/providers",
            get(oidc::list_providers).post(oidc::create_provider),
        )
        .route(
            "/api/oidc/providers/{id}",
            put(oidc::update_provider).delete(oidc::delete_provider),
        )
        .route("/api/oidc/providers/{id}/test", post(oidc::test_provider))
        // OIDC Account Linking (any authenticated user)
        .route(
            "/api/auth/oidc/link/{provider_id}",
            get(oidc::link_oidc_identity),
        )
        .route(
            "/api/auth/oidc/unlink/{identity_id}",
            delete(oidc::unlink_oidc_identity),
        )
        .route("/api/auth/oidc/identities", get(oidc::list_my_identities))
        // SAML SSO Provider Management (SuperAdmin)
        .route(
            "/api/saml/providers",
            get(saml::list_providers).post(saml::create_provider),
        )
        .route(
            "/api/saml/providers/{id}",
            put(saml::update_provider).delete(saml::delete_provider),
        )
        .route("/api/saml/providers/{id}/test", post(saml::test_provider))
        // SAML Account Linking (any authenticated user)
        .route(
            "/api/auth/saml/link/{provider_id}",
            get(saml::link_saml_identity),
        )
        .route(
            "/api/auth/saml/unlink/{identity_id}",
            delete(saml::unlink_saml_identity),
        )
        .route("/api/auth/saml/identities", get(saml::list_my_identities))
        // SSO Attribute Mappings (SuperAdmin, protocol-agnostic)
        .route(
            "/api/sso/mappings/{protocol}/{provider_id}",
            get(sso_mappings::list_mappings).post(sso_mappings::create_mapping),
        )
        .route(
            "/api/sso/mappings/{mapping_id}",
            put(sso_mappings::update_mapping).delete(sso_mappings::delete_mapping),
        )
        // GitLab API & CI/CD Endpoints
        .route("/api/gitlab/status", get(gitlab::get_status))
        .route("/api/gitlab/projects/{project_id}", get(gitlab::get_project))
        .route(
            "/api/gitlab/projects/{project_id}/branches",
            get(gitlab::list_branches),
        )
        .route(
            "/api/gitlab/projects/{project_id}/tags",
            get(gitlab::list_tags),
        )
        .route(
            "/api/gitlab/projects/{project_id}/pipelines",
            get(gitlab::list_pipelines).post(gitlab::create_pipeline),
        )
        .route(
            "/api/gitlab/projects/{project_id}/pipelines/{pipeline_id}",
            get(gitlab::get_pipeline),
        )
        .route(
            "/api/gitlab/projects/{project_id}/pipelines/{pipeline_id}/cancel",
            post(gitlab::cancel_pipeline),
        )
        .route(
            "/api/gitlab/projects/{project_id}/pipelines/{pipeline_id}/retry",
            post(gitlab::retry_pipeline),
        )
        .route(
            "/api/gitlab/projects/{project_id}/pipelines/{pipeline_id}/jobs",
            get(gitlab::list_pipeline_jobs),
        )
        .route(
            "/api/gitlab/projects/{project_id}/jobs/{job_id}",
            get(gitlab::get_job),
        )
        .route(
            "/api/gitlab/projects/{project_id}/jobs/{job_id}/log",
            get(gitlab::get_job_log),
        )
        .route(
            "/api/gitlab/projects/{project_id}/ci-file",
            get(gitlab::get_ci_file).post(gitlab::sync_ci_file),
        )
        .route(
            "/api/gitlab/projects/{project_id}/ci-lint",
            post(gitlab::lint_ci_file),
        )
        // ==================== 配置管理端点 (app_config) ====================
        .route("/api/config/page", get(config_manage::page_configs))
        .route(
            "/api/config",
            get(config_manage::get_config)
                .post(config_manage::save_config)
                .delete(config_manage::delete_config),
        )
        // ==================== 应用管理端点 (app, class, deploy, user, gitlab ci) ====================
        .route("/api/app/page", get(app_manage::page_apps))
        .route("/api/app/apply/page", get(app_manage::page_apply_apps))
        .route("/api/app/create", post(app_manage::create_app))
        .route(
            "/api/app/{appno}",
            get(app_manage::get_app).delete(app_manage::delete_app),
        )
        .route("/api/app/{appno}/save/basic", post(app_manage::save_app_basic))
        .route("/api/app/{appno}/save/charge", post(app_manage::save_app_charge))
        .route("/api/app/class/pages", get(app_manage::page_classes))
        .route("/api/app/class/page", get(app_manage::page_classes))
        .route(
            "/api/app/class",
            get(app_manage::get_class)
                .post(app_manage::save_class)
                .delete(app_manage::delete_class),
        )
        .route("/api/app/{appno}/user/list", get(app_manage::get_app_users))
        .route("/api/app/{appno}/user/create", post(app_manage::create_app_user))
        .route("/api/app/{appno}/user", delete(app_manage::delete_app_user))
        .route("/api/app/{appno}/deploy/list", get(app_manage::get_app_deploys))
        .route(
            "/api/app/{appno}/deploy",
            get(app_manage::get_deploy).delete(app_manage::delete_deploy),
        )
        .route("/api/app/{appno}/deploy/create", post(app_manage::create_deploy))
        .route("/api/app/{appno}/deploy/save", post(app_manage::save_deploy))
        .route("/api/app/{appno}/deploy/copy", get(app_manage::copy_deploy))
        .route("/api/gitlab/{appno}/branches", get(app_manage::get_app_branches))
        .route("/api/gitlab/{appno}/cifile", get(app_manage::get_app_ci_file))
        .route("/api/gitlab/{appno}/pipeline/history", get(app_manage::get_app_pipeline_history))
        .route("/api/gitlab/{appno}/triggerci", get(app_manage::trigger_app_ci))
        .route("/api/gitlab/{appno}/ci/synch", get(app_manage::sync_app_ci))

        // SECURITY: 挂载鉴权中间件检查用户登录与停用状态
        .layer(axum::middleware::from_fn_with_state(
            crate::auth::middleware::AuthDatabaseState {
                store: app_state.store.clone(),
            },
            crate::auth::middleware::auth_middleware_with_db,
        ))
        .with_state(app_state.clone())
}
