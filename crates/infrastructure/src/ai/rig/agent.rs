use std::{path::Path, sync::Arc};

use application::{
    ai::traits::ai_provider::AiProvider, searcher::traits::database::DatabaseProvider,
};
use async_trait::async_trait;
use domain::ai::{config::AgentConfig, message::AIMessage};
use rig::{
    agent::MultiTurnStreamItem,
    client::{AgentClientExt, EmbeddingsClient},
    embeddings::EmbeddingModel,
    message::Message,
    streaming::{StreamedAssistantContent, StreamingChat},
};
use serde_json::json;
use tokio::sync::mpsc::Sender;
use tokio_stream::StreamExt;

use crate::ai::rig::{errors::AIError, rag::RagStorage};

/// Agent is responsible for managing all the AI stuff, such as:
/// - Creating the AI agent
/// - Managing the AI messages
/// - Managing the RAG storage
pub struct RigAgent {
    /// The AI agent (RIG agent) -> also contains the RAG storage
    agent: rig::Agent,
}

impl RigAgent {
    pub async fn new_auto_provider(
        config: AgentConfig,
        db: Arc<dyn DatabaseProvider>,
    ) -> Result<RigAgent, AIError> {
        let agent = match config.provider.as_str() {
            "openai" => {
                let client = rig::providers::openai::Client::new(config.api_key.clone())
                    .map_err(|e| AIError::ApiConnectionError(e.to_string()))?;

                let embeddings = client.embedding_model(config.rag_config.embedding_model.clone());

                Self::new_auto(client, config, db, embeddings).await
            }
            "ollama" => {
                let client = rig::providers::ollama::Client::new(config.api_key.clone())
                    .map_err(|e| AIError::ApiConnectionError(e.to_string()))?;

                let embeddings = client.embedding_model(config.rag_config.embedding_model.clone());

                Self::new_auto(client, config.clone(), db, embeddings).await
            }
            _ => return Err(AIError::InvalidProvider(config.provider)),
        };

        return agent;
    }

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
        db: Arc<dyn DatabaseProvider>,
        embedding_model: Model,
    ) -> Result<Self, AIError> {
        let indexer_path = db.get_index_path();

        let embeddings_path = Path::new(&indexer_path).join("embeddings");

        if embeddings_path.exists() {
            return Self::new_load_storage(client, config, indexer_path, embedding_model).await;
        } else {
            return Self::new_create_storage(client, config, db, indexer_path, embedding_model)
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
    async fn new_load_storage<
        T: AgentClientExt + EmbeddingsClient + 'static,
        Model: EmbeddingModel + Clone + Send + Sync + 'static,
    >(
        client: T,
        config: AgentConfig,
        index_path: String,
        embedding_model: Model,
    ) -> Result<Self, AIError> {
        let mut rag_storage = RagStorage::new(
            Path::new(&index_path)
                .join("embeddings")
                .to_string_lossy()
                .to_string(),
            embedding_model,
            config.rag_config.clone(),
        )
        .await?;

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
    async fn new_create_storage<
        T: AgentClientExt + EmbeddingsClient + 'static,
        Model: EmbeddingModel + Clone + Send + Sync + 'static,
    >(
        client: T,
        config: AgentConfig,
        database: Arc<dyn DatabaseProvider>,
        index_path: String,
        embedding_model: Model,
    ) -> Result<Self, AIError> {
        let mut rag_storage = RagStorage::new(
            Path::new(&index_path)
                .join("embeddings")
                .to_string_lossy()
                .to_string(),
            embedding_model,
            config.rag_config.clone(),
        )
        .await?;

        rag_storage.init_from_db(database).await?;

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

        agent = match config.provider.as_str() {
            "ollama" => agent.additional_params(json!({
                "think": config.think
            })),
            _ => agent,
        }
        .preamble(&config.preamble)
        .dynamic_context(config.rag_config.top_results, storage);

        let agent = agent.build();

        return Ok(Self { agent: agent });
    }
}

#[async_trait]
impl AiProvider for RigAgent {
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
    async fn ask(
        &self,
        message: String,
        sender: Sender<String>,
        messages: Vec<AIMessage>,
    ) -> Result<(AIMessage, AIMessage), String> {
        let mut response = self
            .agent
            .stream_chat(
                message.clone(),
                messages.iter().map(|m| match m.sender.as_str() {
                    "assistant" => Message::assistant(m.content.clone()),
                    "system" => Message::system(m.content.clone()),
                    "user" => Message::user(m.content.clone()),
                    _ => Message::system(m.content.clone()),
                }),
            )
            .await;

        let mut final_message = String::new();

        while let Some(chunk) = response.next().await {
            match chunk {
                Ok(msg) => {
                    let chunk = match msg {
                        MultiTurnStreamItem::StreamAssistantItem(
                            StreamedAssistantContent::Text(text),
                        ) => text.text,
                        _ => String::new(),
                    };

                    final_message.push_str(chunk.as_str());

                    if let Err(_) = sender.send(chunk).await {
                        break; // RX disconnected, we stop else we'll send these messages in the VOID
                    }
                }
                Err(e) => {
                    return Err(AIError::StreamError(e).to_string());
                }
            }
        }

        return Ok((
            AIMessage::user(message),
            AIMessage::assistant(final_message),
        ));
    }
}
