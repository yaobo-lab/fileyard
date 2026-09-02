#[derive(Debug, thiserror::Error)]
pub enum DataError {
    #[error("database operation failed: {0}")]
    Database(#[from] sea_orm::DbErr),
    #[error("record not found")]
    NotFound,
    #[error("record already exists")]
    Conflict,
    #[error("invalid data query: {0}")]
    InvalidQuery(String),
}

pub type DataResult<T> = Result<T, DataError>;
