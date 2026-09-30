use std::sync::Arc;

use domain::searcher::{document::Document, get_mode::DocumentGetMode};

use crate::searcher::{error::DatabaseError, traits::database::DatabaseProvider};

pub struct DatabaseGet {
    database: Arc<dyn DatabaseProvider>,
}

impl DatabaseGet {
    pub fn new(database: Arc<dyn DatabaseProvider>) -> Self {
        return Self { database: database };
    }

    /// Gets a document from the database
    /// Parameters:
    /// - path: the path to the file
    /// Returns:
    /// - Option<Document>: the document if it exists, None if it doesn't
    pub fn get_one(&self, path: String) -> Result<Document, DatabaseError> {
        return self.database.get(path);
    }

    /// Get all Database's documents WITH CONTENTS
    ///
    /// Returns:
    /// - Vec<Document>: all the documents
    pub fn get_all_with_content(&self) -> Result<Vec<Document>, DatabaseError> {
        return self.database.get_all(DocumentGetMode::WithContent);
    }

    /// Get all Database's documents WITHOUT CONTENT
    ///
    /// Returns:
    /// - Vec<Document>: all the documents
    pub fn get_all_without_content(&self) -> Result<Vec<Document>, DatabaseError> {
        return self.database.get_all(DocumentGetMode::WithoutContent);
    }
}
