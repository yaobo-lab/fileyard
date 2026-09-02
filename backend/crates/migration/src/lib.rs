pub use sea_orm_migration::prelude::*;

macro_rules! sql_migration {
    ($module:ident, $name:literal, $path:literal) => {
        mod $module {
            use sea_orm_migration::prelude::*;

            pub struct Migration;

            impl MigrationName for Migration {
                fn name(&self) -> &str {
                    $name
                }
            }

            #[async_trait::async_trait]
            impl MigrationTrait for Migration {
                async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
                    manager
                        .get_connection()
                        .execute_unprepared(include_str!($path))
                        .await?;
                    Ok(())
                }

                async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
                    Err(DbErr::Migration(format!(
                        "{} is an immutable baseline migration and cannot be reverted",
                        $name
                    )))
                }
            }
        }
    };
}

sql_migration!(
    m001_schema,
    "001_schema",
    "../../../migrations/001_schema.sql"
);
sql_migration!(
    m002_seed_data,
    "002_seed_data",
    "../../../migrations/002_seed_data.sql"
);
sql_migration!(
    m003_oidc_sso,
    "003_oidc_sso",
    "../../../migrations/003_oidc_sso.sql"
);
sql_migration!(
    m004_document_approvals,
    "004_document_approvals",
    "../../../migrations/004_document_approvals.sql"
);
sql_migration!(
    m005_backup_settings,
    "005_backup_settings",
    "../../../migrations/005_backup_settings.sql"
);
sql_migration!(
    m006_global_backup_history,
    "006_global_backup_history",
    "../../../migrations/006_global_backup_history.sql"
);

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m001_schema::Migration),
            Box::new(m002_seed_data::Migration),
            Box::new(m003_oidc_sso::Migration),
            Box::new(m004_document_approvals::Migration),
            Box::new(m005_backup_settings::Migration),
            Box::new(m006_global_backup_history::Migration),
        ]
    }
}
