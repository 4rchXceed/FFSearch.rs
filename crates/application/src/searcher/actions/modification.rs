use std::sync::Arc;

use domain::searcher::document::Document;

use crate::{
    indexer::traits::registry::{FileIndexerRegistry, IndexerId},
    searcher::error::DatabaseError,
};

pub struct UpdateDatabase {
    registry: Arc<dyn FileIndexerRegistry>,
}

impl UpdateDatabase {
    pub fn new(registry: Arc<dyn FileIndexerRegistry>) -> Self {
        return Self { registry: registry };
    }

    /// Adds a document to the database
    /// Parameters:
    /// - document: The document
    ///
    /// Returns:
    /// - String: the uuid of the new document
    pub async fn add(
        &self,
        for_database: IndexerId,
        document: Document,
    ) -> Result<String, DatabaseError> {
        let database = self
            .registry
            .get_database(for_database)
            .await
            .ok_or(DatabaseError::Database(String::from("Indexer deleted")))?;

        return database.add_document(document).await;
    }

    /// Removes a document from the database
    /// Parameters:
    /// - path: the path of the document to remove
    ///
    /// Returns:
    /// - Result<String, DatabaseError>: Ok(String) the uuid of the document that was removed, Err(DatabaseError) if there's an error
    pub async fn remove(
        &self,
        for_database: IndexerId,
        path: String,
    ) -> Result<String, DatabaseError> {
        let database = self
            .registry
            .get_database(for_database)
            .await
            .ok_or(DatabaseError::Database(String::from("Indexer deleted")))?;

        return database.remove_document(path).await;
    }

    /// Removes a document from the database
    /// Parameters:
    /// - path: the path of the document to remove
    /// - new_document: the new document values
    /// Returns:
    /// - Result<(), ()>: Ok(()) if the document was removed, Err(()) if the document was not found
    pub async fn update(
        &self,
        for_database: IndexerId,
        path: String,
        new_document: Document,
    ) -> Result<(), DatabaseError> {
        let database = self
            .registry
            .get_database(for_database)
            .await
            .ok_or(DatabaseError::Database(String::from("Indexer deleted")))?;

        return database.update_document(path, new_document).await;
    }

    /// Commits the changes to the database
    /// I did it like this, so it's MUUUUCH faster
    pub async fn apply_changes(&self, for_database: IndexerId) -> Result<(), DatabaseError> {
        let database = self
            .registry
            .get_database(for_database)
            .await
            .ok_or(DatabaseError::Database(String::from("Indexer deleted")))?;

        return database.commit().await;
    }
}
