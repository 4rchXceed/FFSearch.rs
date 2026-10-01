use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;

use crate::{
    ai::traits::ai_provider::AiProvider, indexer::traits::file_indexer::FileIndexerProvider,
    searcher::traits::database::DatabaseProvider,
};

pub type IndexerId = u32;

#[async_trait]
pub trait FileIndexerRegistry: Send + Sync {
    async fn add(
        &self,
        file_indexer: Arc<dyn FileIndexerProvider>,
        db: Arc<dyn DatabaseProvider>,
        folder: String,
    ) -> IndexerId;

    async fn assign_agent(&self, id: IndexerId, agent: Arc<dyn AiProvider>) -> Result<(), ()>;

    async fn list_indexers(&self) -> HashMap<IndexerId, String>;

    async fn get_indexer(&self, id: IndexerId) -> Option<Arc<dyn FileIndexerProvider>>;
    async fn get_database(&self, id: IndexerId) -> Option<Arc<dyn DatabaseProvider>>;
    async fn get_agent(&self, id: IndexerId) -> Option<Option<Arc<dyn AiProvider>>>;
    async fn get_folder(&self, id: IndexerId) -> Option<String>;
}
