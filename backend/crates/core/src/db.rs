use sqlx::PgPool;

#[derive(Clone)]
pub struct Db {
    pub pool: PgPool,
}
