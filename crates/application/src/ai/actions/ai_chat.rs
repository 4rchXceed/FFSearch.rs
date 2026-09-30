use std::sync::Arc;

use domain::ai::message::AIMessage;
use tokio::sync::{RwLock, mpsc::Sender};

use crate::ai::traits::ai_provider::AiProvider;

pub struct AiChat {
    messages: Arc<RwLock<Vec<AIMessage>>>,
    ai: Arc<dyn AiProvider>,
}

impl AiChat {
    pub fn new(ai: Arc<dyn AiProvider>) -> Self {
        return Self {
            messages: Arc::new(RwLock::new(Vec::new())),
            ai: ai,
        };
    }

    pub async fn ask(
        &self,
        query: String,
        msg_sender: Sender<String>,
    ) -> Result<AIMessage, String> {
        let (message, response) = {
            self.ai
                .ask(query, msg_sender, self.messages.read().await.clone())
                .await?
        };

        self.messages.write().await.push(message.clone());
        self.messages.write().await.push(response.clone());

        return Ok(message);
    }

    pub async fn reset(&self) {
        self.messages.write().await.clear();
    }

    pub async fn get_all(&self) -> Vec<AIMessage> {
        return self.messages.read().await.clone();
    }
}
