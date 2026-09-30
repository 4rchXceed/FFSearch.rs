use thiserror::Error;

/// Represents all possible errors that can occur during file indexing.
#[derive(Debug, Error, Clone)]
pub enum FileIndexerError {
    #[error("Failed to get file metadata: {0}")]
    GetFileMetaError(String),
    #[error("Failed to open file to write modification date: {0}")]
    OpenFileToWriteModDateError(String),
    #[error("Failed to update document in the database: {0}")]
    CouldNotUpdateDocument(String),
    #[error("Failed to add document to the database: {0}")]
    CouldNotAddDocument(String),
    #[error("Invalid file path: {0}")]
    InvalidFilePath(String),
    #[error("Failed to remove document from the database: {0}")]
    FailedToRemoveDocument(String),
    #[error("The database failed to init: {0}")]
    DatabaseInitError(String),
    #[error("Failed to get all the documents from the DB: {0}")]
    GetAllDocumentsFailed(String),
    #[error(
        "Thread panicked during file indexing, this is likely a bug in the code. You can check the logs."
    )]
    ThreadError,
    #[error("Failed to get file content: {0}")]
    GetFileContentError(String),
    #[error("Failed to commit changes to the database: {0}")]
    DatabaseCommitError(String),
}
