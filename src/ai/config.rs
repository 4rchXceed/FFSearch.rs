use crate::ai::{DEFAULT_PREAMBLE, rag::config::RagConfig};

/// The agent's provider (API/Local)
/// 3 choices:
/// - OpenAI
/// - Anthropic
/// - Ollama
pub enum AgentProvider {
    /// OpenAI API provider (requires API key)
    OpenAI,
    /// Anthropic API provider (requires API key)
    Anthropic,
    /// Ollama local model provider (recommended)
    Ollama,
}

/// Configuration for the AI agent
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
        }
    }

    /// [Only available for Ollama]
    /// Sets the think flag for the agent
    /// Disables the thinking for thinking models
    pub fn set_think(&mut self, think: bool) {
        self.think = think;
    }
}
