/// The configuration for the RAG system
#[derive(Debug, Clone)]
pub struct RagConfig {
    /// [Advanced] The number of top results to return
    pub top_results: usize,
    /// The embedding model name
    pub embedding_model: String,
}

impl RagConfig {
    /// Creates a new "RagConfig" with default values
    pub fn new(model: String) -> Self {
        return Self {
            top_results: 5,
            embedding_model: model,
        };
    }
}
