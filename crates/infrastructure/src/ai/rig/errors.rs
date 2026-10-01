use application::searcher::error::DatabaseError;
use rig::{
    agent::StreamingError,
    embeddings::{EmbedError, EmbeddingError},
};
use thiserror::Error;

/// These are all the errors that can occur for the AI component.
#[derive(Debug, Error)]
pub enum AIError {
    #[error("Error while streaming response: {0}")]
    StreamError(StreamingError),
    #[error("Failed to connect to the storage: {0}")]
    ConnectToStorageError(lancedb::Error),
    #[error("Failed to create index: {0}")]
    CreateIndexError(lancedb::Error),
    #[error("Failed to list table names: {0}")]
    ListTableNamesError(lancedb::Error),
    #[error("Failed to save the documents to the database: {0}")]
    SetDocumentsFailed(EmbedError),
    #[error("Failed to build the AI's database: {0}")]
    BuildEmbeddingsFailed(EmbeddingError),
    #[error("Failed to build the RAG database")]
    FailedToBuildRagDbDatas,
    #[error("Failed to get update the Index: {0}")]
    GetIndexStatsError(lancedb::Error),
    #[error("There's no document available")]
    FaildToGetNbrDimentions,
    #[error("Failed to create DB table: {0}")]
    CreateTableError(lancedb::Error),
    #[error("Failed to open DB table")]
    OpenTableError,
    #[error("Failed to get all the documents: {0}")]
    FailedToGetAllDocuments(DatabaseError),
    #[error("Other error: {0}")]
    Other(String),
    #[error("The file indexer has been deleted while creating the RAG")]
    IndexerDeleted,
    #[error("The provider: {0} is an invalid provider (valid: ollama, openai)")]
    InvalidProvider(String),
    #[error("Error while connecting to agent provider")]
    ApiConnectionError(String),
}
