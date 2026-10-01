/// The configuration for the RAG system
#[derive(Debug, Clone)]
pub struct RagConfig {
    /// [Advanced] The number of top results to return
    pub top_results: usize,
    /// The embedding model name
    pub embedding_model: String,
    /// [Advanced] The max size of every batch
    pub max_batch_size: usize,
}

impl RagConfig {
    /// Creates a new "RagConfig" with default values
    pub fn new(model: String) -> Self {
        return Self {
            top_results: 5,
            embedding_model: model,
            max_batch_size: 2000,
        };
    }
}
