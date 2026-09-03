/// The configuration for the RAG system
#[derive(Debug, Clone)]
pub struct RagConfig {
    /// [Advanced] The number of top results to return
    pub top_results: usize,
}

impl RagConfig {
    /// Creates a new "RagConfig" with default values
    pub fn new() -> Self {
        return Self { top_results: 5 };
    }
}
