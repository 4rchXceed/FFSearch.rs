use std::path::Path;

use crate::{
    ai::{config::AgentConfig, rag::storage::RagStorage},
    controller::controller::FolderController,
    searcher::database::DatabaseError,
    utils::join_path,
};
use rig::{
    agent::{MultiTurnStreamItem, StreamingError},
    client::{AgentClientExt, EmbeddingsClient},
    embeddings::{EmbedError, EmbeddingError, EmbeddingModel},
    message::Message,
    streaming::{StreamedAssistantContent, StreamingChat},
};
use serde_json::json;
use thiserror::Error;
use tokio::sync::mpsc::Sender;
use tokio_stream::StreamExt;

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
}

/// Agent is responsible for managing all the AI stuff, such as:
/// - Creating the AI agent
/// - Managing the AI messages
/// - Managing the RAG storage
pub struct Agent {
    /// The AI agent (RIG agent) -> also contains the RAG storage
    agent: rig::Agent,
    /// The RAG messages
    messages: Vec<Message>,
}

impl Agent {
    /// Creates a new Agent with the given configuration and database.
    ///
    /// Type parameters:
    /// - "T": The agent client type (example rig::provider::ollama::Client)
    /// - "Model": The embedding model type (example rig::provider::ollama::EmbeddingModel)
    ///
    /// Parameters:
    /// - client: The connection client
    /// - config: The agent configuration (see AgentConfig)
    /// - indexer: An Arc ref to the file indexer
    /// - embedding_model: The embedding model to use for RAG storage
    ///
    /// Returns:
    /// - A new Agent instance
    pub async fn new_auto<
        T: AgentClientExt + EmbeddingsClient + 'static,
        Model: EmbeddingModel + Clone + Send + Sync + 'static,
    >(
        client: T,
        config: AgentConfig,
        indexer: FolderController,
        embedding_model: Model,
    ) -> Result<Self, AIError> {
        let indexer_path = indexer.get_db_path().await.ok_or(AIError::IndexerDeleted)?;

        if Path::new(&join_path(&indexer_path, "embeddings")).exists() {
            return Self::new_load_storage(client, config, indexer_path, embedding_model).await;
        } else {
            return Self::new_create_storage(
                client,
                config,
                indexer,
                indexer_path,
                embedding_model,
            )
            .await;
        }
    }

    /// Loads the embeddings & creates the rag from an existing folder
    ///
    /// Type parameters:
    /// - "T": The agent client type (example rig::provider::ollama::Client)
    /// - "Model": The embedding model type (example rig::provider::ollama::EmbeddingModel)
    ///
    /// Parameters:
    /// - client: The rig client
    /// - config: The agent's config
    /// - index_path: the embeddings path
    /// - embedding_model: the embeddings model
    ///
    /// Returns:
    /// - Result<Self, AIError>: the result of the operation
    pub async fn new_load_storage<
        T: AgentClientExt + EmbeddingsClient + 'static,
        Model: EmbeddingModel + Clone + Send + Sync + 'static,
    >(
        client: T,
        config: AgentConfig,
        index_path: String,
        embedding_model: Model,
    ) -> Result<Self, AIError> {
        let mut rag_storage = RagStorage::new(index_path, embedding_model).await?;

        rag_storage.load_index().await?;

        return Self::new_from_rag(client, config, rag_storage);
    }

    /// Creates the embeddings from a file indexer / database
    ///
    /// Type parameters:
    /// - "T": The agent client type (example rig::provider::ollama::Client)
    /// - "Model": The embedding model type (example rig::provider::ollama::EmbeddingModel)
    ///
    /// Parameters:
    /// - client: The rig client
    /// - config: The agent's config
    /// - indexer: An Arc ref to the file indexer
    /// - index_path: the embeddings path
    /// - embedding_model: the embeddings model
    ///
    /// Returns:
    /// - Result<Self, AIError>: the result of the operation
    pub async fn new_create_storage<
        T: AgentClientExt + EmbeddingsClient + 'static,
        Model: EmbeddingModel + Clone + Send + Sync + 'static,
    >(
        client: T,
        config: AgentConfig,
        indexer: FolderController,
        index_path: String,
        embedding_model: Model,
    ) -> Result<Self, AIError> {
        let mut rag_storage = RagStorage::new(index_path, embedding_model).await?;

        rag_storage.init_from_db(indexer).await?;

        return Self::new_from_rag(client, config, rag_storage);
    }

    /// Creates the Agent from a already existing RAG
    ///
    /// Type parameters:
    /// - "T": The agent client type (example rig::provider::ollama::Client)
    /// - "Model": The embedding model type (example rig::provider::ollama::EmbeddingModel)
    ///
    /// Parameters:
    /// - client: the rig client
    /// - config: the agent's config
    /// - storage: the RAG
    ///
    /// Returns:
    /// - Result(Self, AIError): the result of the operation
    fn new_from_rag<
        T: AgentClientExt + EmbeddingsClient + 'static,
        Model: EmbeddingModel + Clone + Send + Sync + 'static,
    >(
        client: T,
        config: AgentConfig,
        storage: RagStorage<Model>,
    ) -> Result<Self, AIError> {
        let mut agent = client.agent(config.model.clone());

        agent = match config.provider {
            super::config::AgentProvider::Ollama => agent.additional_params(json!({
                "think": config.think
            })),
            _ => agent,
        }
        .preamble(&config.preamble)
        .dynamic_context(config.rag_config.top_results, storage);

        let messages = Vec::new();
        let agent = agent.build();

        return Ok(Self {
            agent: agent,
            messages: messages,
        });
    }

    /// Sends a message to the agent and streams the response.
    ///
    /// Parameters:
    /// - message: The message to send
    /// - callback: The callback function which will be called with each chunk of the response
    /// - this: The caller's context, used to pass a mutable reference to the callback
    ///
    /// Returns:
    /// - Ok(()) if the response was successfully streamed
    /// - Err(AIError) if an error occurred (see AIError)
    pub async fn send(&mut self, message: String, sender: Sender<String>) -> Result<(), AIError> {
        let mut response = self.agent.stream_chat(message, &self.messages).await;
        while let Some(chunk) = response.next().await {
            match chunk {
                Ok(msg) => {
                    if let Err(_) = sender
                        .send(match msg {
                            MultiTurnStreamItem::StreamAssistantItem(
                                StreamedAssistantContent::Text(text),
                            ) => text.text,
                            _ => String::new(),
                        })
                        .await
                    {
                        break; // RX disconnected, we stop else we'll send these messages in the VOID
                    }
                }
                Err(e) => {
                    return Err(AIError::StreamError(e));
                }
            }
        }
        Ok(())
    }

    /// Clears the agent's context
    pub fn clear(&mut self) {
        self.messages.clear();
    }

    /// Returns the agent's messages
    ///
    /// Returns:
    /// - The agent's messages (ref)
    pub fn get_messages(&self) -> &Vec<Message> {
        return &self.messages;
    }
}
