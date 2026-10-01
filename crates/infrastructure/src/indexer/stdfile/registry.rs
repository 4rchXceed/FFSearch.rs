use std::{collections::HashMap, sync::Arc};

use application::{
    ai::traits::ai_provider::AiProvider,
    indexer::traits::{
        file_indexer::FileIndexerProvider,
        registry::{FileIndexerRegistry, IndexerId},
    },
    searcher::traits::database::DatabaseProvider,
};
use async_trait::async_trait;
use tokio::sync::RwLock;

pub struct RegistryEntry {
    pub indexer: Arc<dyn FileIndexerProvider>,
    pub database: Arc<dyn DatabaseProvider>,
    pub agent: Option<Arc<dyn AiProvider>>,
    pub folder: String,
}

pub struct InMemoryRegistry {
    file_indexers: Arc<RwLock<HashMap<IndexerId, RegistryEntry>>>,
    current: Arc<RwLock<u32>>,
}

impl InMemoryRegistry {
    pub fn new() -> Self {
        return Self {
            file_indexers: Arc::new(RwLock::new(HashMap::new())),
            current: Arc::new(RwLock::new(0)),
        };
    }
}

#[async_trait]
impl FileIndexerRegistry for InMemoryRegistry {
    async fn add(
        &self,
        file_indexer: Arc<dyn FileIndexerProvider>,
        db: Arc<dyn DatabaseProvider>,
        folder: String,
    ) -> IndexerId {
        let id = {
            let mut current = self.current.write().await;
            *current += 1;
            current.clone()
        };

        self.file_indexers.write().await.insert(
            id,
            RegistryEntry {
                indexer: file_indexer,
                database: db,
                agent: None,
                folder: folder,
            },
        );

        return id;
    }

    async fn assign_agent(&self, id: IndexerId, agent: Arc<dyn AiProvider>) -> Result<(), ()> {
        let mut indexers = self.file_indexers.write().await;

        if let Some(indexer) = indexers.get_mut(&id) {
            indexer.agent = Some(agent);

            return Ok(());
        } else {
            return Err(());
        }
    }

    async fn list_indexers(&self) -> HashMap<IndexerId, String> {
        let indexers = self.file_indexers.read().await;

        return indexers
            .iter()
            .map(|(k, i)| (*k, i.folder.clone()))
            .collect();
    }

    async fn get_indexer(&self, id: IndexerId) -> Option<Arc<dyn FileIndexerProvider>> {
        let indexers = self.file_indexers.read().await;

        return Some(indexers.get(&id)?.indexer.clone());
    }

    async fn get_database(&self, id: IndexerId) -> Option<Arc<dyn DatabaseProvider>> {
        let indexers = self.file_indexers.read().await;

        return Some(indexers.get(&id)?.database.clone());
    }
    async fn get_agent(&self, id: IndexerId) -> Option<Option<Arc<dyn AiProvider>>> {
        let indexers = self.file_indexers.read().await;

        return Some(indexers.get(&id)?.agent.clone());
    }
    async fn get_folder(&self, id: IndexerId) -> Option<String> {
        let indexers = self.file_indexers.read().await;

        return Some(indexers.get(&id)?.folder.clone());
    }
}
