use std::sync::Arc;

use domain::ai::message::AIMessage;
use tokio::sync::{RwLock, mpsc::Sender};

use crate::indexer::traits::registry::{FileIndexerRegistry, IndexerId};

pub struct AiChat {
    messages: Arc<RwLock<Vec<AIMessage>>>,
    registry: Arc<dyn FileIndexerRegistry>,
}

impl AiChat {
    pub fn new(registry: Arc<dyn FileIndexerRegistry>) -> Self {
        return Self {
            messages: Arc::new(RwLock::new(Vec::new())),
            registry: registry,
        };
    }

    pub async fn ask(
        &self,
        for_indexer: IndexerId,
        query: String,
        msg_sender: Sender<String>,
    ) -> Result<AIMessage, String> {
        let ai = self
            .registry
            .get_agent(for_indexer)
            .await
            .ok_or(String::from("File indexer not found / delete"))?
            .ok_or(String::from("No agent created for this folder"))?;

        let (message, response) = {
            ai.ask(query, msg_sender, self.messages.read().await.clone())
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
