use std::sync::Arc;

use domain::searcher::{
    document::Document,
    search_query::{SearchContent, SearchQuery},
};

use crate::{
    indexer::traits::registry::{FileIndexerRegistry, IndexerId},
    searcher::error::DatabaseError,
};

#[derive(Clone)]
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
        search_inside_of: SearchContent,
    ) -> Result<Vec<Document>, DatabaseError> {
        let query = SearchQuery {
            query: query,
            limit: limit,
            search_for: search_inside_of,
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
