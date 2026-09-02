use sea_orm::DatabaseConnection;

use crate::repositories::{
    AiRepository, AuthRepository, DepartmentRepository, ExtensionPermissionRepository,
    ExtensionRuntimeRepository, FileRepository, NotificationRepository,
    ReplicationRepository, SecurityRepository, VirusScanRepository,
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

    pub fn auth(&self) -> AuthRepository<'_> {
        AuthRepository::new(&self.db)
    }

    pub fn departments(&self) -> DepartmentRepository<'_> {
        DepartmentRepository::new(&self.db)
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

    pub fn files(&self) -> FileRepository<'_> {
        FileRepository::new(&self.db)
    }

    pub fn notifications(&self) -> NotificationRepository<'_> {
        NotificationRepository::new(&self.db)
    }

    pub fn security(&self) -> SecurityRepository<'_> {
        SecurityRepository::new(&self.db)
    }

    pub fn virus_scan(&self) -> VirusScanRepository<'_> {
        VirusScanRepository::new(&self.db)
    }
}
