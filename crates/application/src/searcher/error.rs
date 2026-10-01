use domain::searcher::error::DatabaseDomainError;
use thiserror::Error;

#[derive(Debug, Clone, Error)]
pub enum DatabaseError {
    #[error(transparent)]
    DomainError(DatabaseDomainError),
    #[error("Failure in the database: {0}")]
    Database(String),
}
