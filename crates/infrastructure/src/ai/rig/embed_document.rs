use rig::Embed;

/// Represents a document for the RAG
#[derive(Debug, Clone, Embed)]
pub struct EmbedDocument {
    /// The path to the document
    pub path: String,
    /// The content of the document
    #[embed]
    pub content: String,
}
