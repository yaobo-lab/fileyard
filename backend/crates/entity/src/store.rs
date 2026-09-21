use sea_orm::DatabaseConnection;

use crate::repositories::{
    AiRepository, ApiUsageRepository, ApprovalRepository, AuditRepository, AuthRepository, BackupRepository, CommentRepository, ComplianceRepository, DashboardRepository, DepartmentRepository,
    DiscordRepository, EmailTemplateRepository, ExtensionPermissionRepository, ExtensionRuntimeRepository, FileRepository, FileRequestRepository,
    GlobalSettingsRepository, GroupRepository, MqttClientRepository, NotificationRepository, OidcRepository,
    ReplicationRepository, RoleRepository, SamlRepository, SearchRepository, SecurityRepository, ShareRepository, SsoRepository,
    SystemRepository, TenantRepository, UserRepository, VirusScanRepository,
};

/// The only database capability exposed to application crates.
///
/// The underlying connection is deliberately private so callers cannot bypass
/// repositories with ad-hoc SeaORM or SQL queries.
#[derive(Clone)]
pub struct DataStore {
    db: DatabaseConnection,
}

impl DataStore {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    pub fn ai(&self) -> AiRepository<'_> {
        AiRepository::new(&self.db)
    }

    pub fn api_usage(&self) -> ApiUsageRepository<'_> {
        ApiUsageRepository::new(&self.db)
    }

    pub fn approvals(&self) -> ApprovalRepository<'_> {
        ApprovalRepository::new(&self.db)
    }

    pub fn audit(&self) -> AuditRepository<'_> {
        AuditRepository::new(&self.db)
    }

    pub fn auth(&self) -> AuthRepository<'_> {
        AuthRepository::new(&self.db)
    }

    pub fn backup(&self) -> BackupRepository<'_> {
        BackupRepository::new(&self.db)
    }
    pub fn comments(&self) -> CommentRepository<'_> {
        CommentRepository::new(&self.db)
    }
    pub fn compliance(&self) -> ComplianceRepository<'_> {
        ComplianceRepository::new(&self.db)
    }
    pub fn dashboard(&self) -> DashboardRepository<'_> {
        DashboardRepository::new(&self.db)
    }

    pub fn departments(&self) -> DepartmentRepository<'_> {
        DepartmentRepository::new(&self.db)
    }

    pub fn discord(&self) -> DiscordRepository<'_> {
        DiscordRepository::new(&self.db)
    }

    pub fn email_templates(&self) -> EmailTemplateRepository<'_> {
        EmailTemplateRepository::new(&self.db)
    }

    pub fn extension_permissions(&self) -> ExtensionPermissionRepository<'_> {
        ExtensionPermissionRepository::new(&self.db)
    }

    pub fn extension_runtime(&self) -> ExtensionRuntimeRepository<'_> {
        ExtensionRuntimeRepository::new(&self.db)
    }

    pub fn replication(&self) -> ReplicationRepository<'_> {
        ReplicationRepository::new(&self.db)
    }
    pub fn roles(&self) -> RoleRepository<'_> {
        RoleRepository::new(&self.db)
    }

    pub fn files(&self) -> FileRepository<'_> {
        FileRepository::new(&self.db)
    }

    pub fn file_requests(&self) -> FileRequestRepository<'_> {
        FileRequestRepository::new(&self.db)
    }

    pub fn global_settings(&self) -> GlobalSettingsRepository<'_> {
        GlobalSettingsRepository::new(&self.db)
    }
    pub fn groups(&self) -> GroupRepository<'_> {
        GroupRepository::new(&self.db)
    }

    pub fn notifications(&self) -> NotificationRepository<'_> {
        NotificationRepository::new(&self.db)
    }
    pub fn oidc(&self) -> OidcRepository<'_> {
        OidcRepository::new(&self.db)
    }
    pub fn saml(&self) -> SamlRepository<'_> {
        SamlRepository::new(&self.db)
    }

    pub fn security(&self) -> SecurityRepository<'_> {
        SecurityRepository::new(&self.db)
    }
    pub fn search(&self) -> SearchRepository<'_> { SearchRepository::new(&self.db) }

    pub fn sso(&self) -> SsoRepository<'_> {
        SsoRepository::new(&self.db)
    }

    pub fn system(&self) -> SystemRepository<'_> {
        SystemRepository::new(&self.db)
    }

    pub fn tenants(&self) -> TenantRepository<'_> {
        TenantRepository::new(&self.db)
    }

    pub fn virus_scan(&self) -> VirusScanRepository<'_> {
        VirusScanRepository::new(&self.db)
    }
    pub fn shares(&self) -> ShareRepository<'_> {
        ShareRepository::new(&self.db)
    }

    pub fn users(&self) -> UserRepository<'_> {
        UserRepository::new(&self.db)
    }

    pub fn mqtt_clients(&self) -> MqttClientRepository<'_> {
        MqttClientRepository::new(&self.db)
    }

    /// Access the underlying SeaORM database connection.
    pub fn db(&self) -> &sea_orm::DatabaseConnection {
        &self.db
    }
}

