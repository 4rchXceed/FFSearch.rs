use tantivy::TantivyError;
use thiserror::Error;

/// This enum represents the possible errors that can occur when using the database
#[derive(Debug, Clone, Error)]
pub enum TantivyDatabaseError {
    #[error("Could not create db reader: {0}")]
    CouldNotCreateReaderError(TantivyError),
    #[error("Search failed: {0}")]
    SearchFailed(TantivyError),
    #[error("No result, expected one but got none")]
    NoResult,
    #[error("Failed to add document: {0}")]
    FailedToAddDocument(TantivyError),
    #[error("Failed to remove document: {0}")]
    FailedToRemoveDocument(TantivyError),
    #[error("Failed to save changes: {0}")]
    IndexCommitError(TantivyError),
    #[error("Failed to close db: {0}")]
    IndexCloseError(TantivyError),
    #[error("Failed to create index: {0}")]
    IndexCreationError(TantivyError),
    #[error("Failed to create index writer: {0}")]
    IndexWriterCreationError(TantivyError),
    #[error("Could not create db directory")]
    CouldNotCreateDirError,
    #[error("Should not happen")]
    ShouldNotHappenError,
    #[error("Could not open index: {0}")]
    CouldNotOpenIndexError(TantivyError),
    #[error("Could not parse the query: {0}, error: {1}")]
    CouldNotParseQuery(String, String),
}
