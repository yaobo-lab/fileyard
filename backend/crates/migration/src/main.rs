#[tokio::main]
async fn main() {
    sea_orm_migration::cli::run_cli(clovalink_migration::Migrator).await;
}
