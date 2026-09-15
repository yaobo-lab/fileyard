#[tokio::main]
async fn main() {
    sea_orm_migration::cli::run_cli(app_migration::Migrator).await;
}
