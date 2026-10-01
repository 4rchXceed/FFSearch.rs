use async_trait::async_trait;
use domain::ai::message::AIMessage;
use tokio::sync::mpsc::Sender;

#[async_trait]
pub trait AiProvider: Send + Sync {
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
    ) -> Result<(AIMessage, AIMessage), String>;
}
