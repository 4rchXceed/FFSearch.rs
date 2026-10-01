use std::sync::Arc;

use domain::searcher::{document::Document, search_query::SearchQuery};

use crate::{
    indexer::traits::registry::{FileIndexerRegistry, IndexerId},
    searcher::error::DatabaseError,
};

pub struct SearchDocument {
    registry: Arc<dyn FileIndexerRegistry>,
}

impl SearchDocument {
    pub fn new(registry: Arc<dyn FileIndexerRegistry>) -> Self {
        return Self { registry: registry };
    }

    pub async fn search(
        &self,
        folder: IndexerId,
        query: String,
        limit: usize,
    ) -> Result<Vec<Document>, DatabaseError> {
        let query = SearchQuery {
            query: query,
            limit: limit,
        };

        let db = self
            .registry
            .get_database(folder)
            .await
            .ok_or(DatabaseError::Database(String::from(
                "Database deleted / doesn't exists",
            )))?;

        return db.search(query);
    }
}
