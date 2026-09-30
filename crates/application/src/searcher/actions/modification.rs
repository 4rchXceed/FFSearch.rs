use std::sync::Arc;

use domain::searcher::document::Document;

use crate::searcher::{error::DatabaseError, traits::database::DatabaseProvider};

pub struct UpdateDatabase {
    database: Arc<dyn DatabaseProvider>,
}

impl UpdateDatabase {
    pub fn new(database: Arc<dyn DatabaseProvider>) -> Self {
        return Self { database: database };
    }

    /// Adds a document to the database
    /// Parameters:
    /// - document: The document
    ///
    /// Returns:
    /// - String: the uuid of the new document
    pub async fn add(&self, document: Document) -> Result<String, DatabaseError> {
        return self.database.add_document(document).await;
    }

    /// Removes a document from the database
    /// Parameters:
    /// - path: the path of the document to remove
    ///
    /// Returns:
    /// - Result<String, DatabaseError>: Ok(String) the uuid of the document that was removed, Err(DatabaseError) if there's an error
    pub async fn remove(&self, path: String) -> Result<String, DatabaseError> {
        return self.database.remove_document(path).await;
    }

    /// Removes a document from the database
    /// Parameters:
    /// - path: the path of the document to remove
    /// - new_document: the new document values
    /// Returns:
    /// - Result<(), ()>: Ok(()) if the document was removed, Err(()) if the document was not found
    pub async fn update(&self, path: String, new_document: Document) -> Result<(), DatabaseError> {
        return self.database.update_document(path, new_document).await;
    }

    /// Commits the changes to the database
    /// I did it like this, so it's MUUUUCH faster
    pub async fn apply_changes(&self) -> Result<(), DatabaseError> {
        return self.database.commit().await;
    }
}
