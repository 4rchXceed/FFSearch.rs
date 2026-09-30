use std::sync::Arc;

use domain::searcher::{document::Document, search_query::SearchQuery};

use crate::searcher::{error::DatabaseError, traits::database::DatabaseProvider};

pub struct SearchDocument {
    database: Arc<dyn DatabaseProvider>,
}

impl SearchDocument {
    pub fn new(database: Arc<dyn DatabaseProvider>) -> Self {
        return Self { database: database };
    }

    pub fn search(&self, query: String, limit: usize) -> Result<Vec<Document>, DatabaseError> {
        let query = SearchQuery {
            query: query,
            limit: limit,
        };

        return self.database.search(query);
    }
}
