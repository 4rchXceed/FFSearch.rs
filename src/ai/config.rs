use crate::ai::{DEFAULT_PREAMBLE, rag::config::RagConfig};

/// The agent's provider (API/Local)
/// 2 choices:
/// - OpenAI
/// - Ollama
#[derive(Clone)]
pub enum AgentProvider {
    /// OpenAI API provider (requires API key)
    OpenAI,
    /// Ollama local model provider (recommended)
    Ollama,
}

/// Configuration for the AI agent
#[derive(Clone)]
pub struct AgentConfig {
    /// The agent's provider (see [`AgentProvider`])
    pub provider: AgentProvider,
    /// The model to use for the agent
    pub model: String,
    /// Whether the agent should think before responding
    pub think: bool,
    /// The preamble to use for the agent
    pub preamble: String,
    /// The RAG configuration to use for the agent
    pub rag_config: RagConfig,
    /// The provider's API key
    pub api_key: String,
}

impl AgentConfig {
    /// Creates an OpenAI confg
    pub fn new(model: &str, rag_config: RagConfig, provider: AgentProvider) -> Self {
        Self {
            provider: provider,
            model: String::from(model),
            think: false,
            preamble: String::from(DEFAULT_PREAMBLE),
            rag_config: rag_config,
            api_key: String::new(),
        }
    }

    /// [Only available for Ollama]
    /// Sets the think flag for the agent
    /// Disables the thinking for thinking models
    pub fn set_think(&mut self, think: bool) {
        self.think = think;
    }
}
