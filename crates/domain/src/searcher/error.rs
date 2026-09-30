use thiserror::Error;

/// This enum represents the possible errors that can occur when using the database
/// !! Only generic (domain) errors !!
#[derive(Debug, Clone, Error)]
pub enum DatabaseDomainError {
    #[error("No result, expected one but got none")]
    NoResult,
}
