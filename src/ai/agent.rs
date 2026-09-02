use crate::{
    ai::{config::AgentConfig, rag::storage::RagStorage},
    searcher::database::Database,
};
use rig::{
    agent::{MultiTurnStreamItem, StreamingError},
    client::{AgentClientExt, EmbeddingsClient},
    embeddings::EmbeddingModel,
    message::Message,
    streaming::{StreamedAssistantContent, StreamingChat},
};
use serde_json::json;
use thiserror::Error;
use tokio_stream::StreamExt;

/// These are all the errors that can occur for the AI component.
#[derive(Debug, Error)]
pub enum AIError {
    #[error("Error while streaming response: {0}")]
    StreamError(StreamingError),
    #[error("Failed to connect to the storage")]
    ConnectToStorageError,
    #[error("Failed to create index")]
    CreateIndexError,
    #[error("Failed to list table names")]
    ListTableNamesError,
    #[error("Failed to save the documents to the database")]
    SetDocumentsFailed,
    #[error("Failed to build the AI's database")]
    BuildEmbeddingsFailed,
    #[error("Failed to build the RAG database")]
    FailedToBuildRagDbDatas,
    #[error("Failed to get update the Index")]
    GetIndexStatsError,
    #[error("There's no document available")]
    FaildToGetNbrDimentions,
    #[error("Failed to create DB table")]
    CreateTableError,
    #[error("Failed to open DB table")]
    OpenTableError,
    #[error("Failed to {0}")]
    Other(String),
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
    /// - db: The database connection
    /// - embedding_model: The embedding model to use for RAG storage
    ///
    /// Returns:
    /// - A new Agent instance
    pub async fn new<
        T: AgentClientExt + EmbeddingsClient + 'static,
        Model: EmbeddingModel + Clone + Send + Sync + 'static,
    >(
        client: T,
        config: AgentConfig,
        db: &mut Database,
        embedding_model: Model,
    ) -> Result<Self, AIError> {
        let mut rag_storage = RagStorage::new(db.get_index_path().clone(), embedding_model).await?;
        rag_storage.init_from_db(db).await?;

        let mut agent = client.agent(config.model.clone());
        agent = match config.provider {
            super::config::AgentProvider::Ollama => agent.additional_params(json!({
                "think": config.think
            })),
            _ => agent,
        }
        .preamble(&config.preamble)
        .dynamic_context(config.rag_config.top_results, rag_storage);
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
    pub async fn send<This>(
        &mut self,
        message: &str,
        mut callback: impl FnMut(&mut This, String) + Send + 'static,
        this: &mut This,
    ) -> Result<(), AIError> {
        let mut response = self.agent.stream_chat(message, &self.messages).await;
        while let Some(chunk) = response.next().await {
            match chunk {
                Ok(msg) => {
                    callback(
                        this,
                        match msg {
                            MultiTurnStreamItem::StreamAssistantItem(
                                StreamedAssistantContent::Text(text),
                            ) => text.text,
                            _ => String::new(),
                        },
                    );
                }
                Err(e) => {
                    // println!("{:?}", e);
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
