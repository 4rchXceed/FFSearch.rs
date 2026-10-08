use async_trait::async_trait;

#[async_trait]
pub trait TextExtractor: Send + Sync {
    /// Extracts text from a file at the given path.
    /// Returns the extracted text as a string, or an error if extraction fails.
    /// Parameters:
    /// - path: the path to the file to extract text from
    async fn extract(&self, path: String) -> Result<Option<String>, String>;
}
