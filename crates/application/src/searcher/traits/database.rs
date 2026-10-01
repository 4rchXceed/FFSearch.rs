use async_trait::async_trait;
use domain::searcher::{document::Document, get_mode::DocumentGetMode, search_query::SearchQuery};

use crate::searcher::error::DatabaseError;

#[async_trait]
pub trait DatabaseProvider: Send + Sync {
    /// Returns the index path
    fn get_index_path(&self) -> String;

    /// Adds a document to the database
    /// Parameters:
    /// - document: The document
    ///
    /// Returns:
    /// - String: the uuid of the new document
    async fn add_document(&self, document: Document) -> Result<String, DatabaseError>;

    /// Searches the database for a query
    /// Parameters:
    /// - query: the query (with parameters) to search for
    /// Returns:
    /// - Vec<Document>: a vector of documents that match the query
    fn search(&self, query: SearchQuery) -> Result<Vec<Document>, DatabaseError>;

    /// Removes a document from the database
    /// Parameters:
    /// - path: the path of the document to remove
    ///
    /// Returns:
    /// - Result<String, DatabaseError>: Ok(String) the uuid of the document that was removed, Err(DatabaseError) if there's an error
    async fn remove_document(&self, path: String) -> Result<String, DatabaseError>;

    /// Removes a document from the database
    /// Parameters:
    /// - path: the path of the document to remove
    /// - new_document: the new document values
    /// Returns:
    /// - Result<(), ()>: Ok(()) if the document was removed, Err(()) if the document was not found
    async fn update_document(
        &self,
        path: String,
        new_document: Document,
    ) -> Result<(), DatabaseError>;

    /// Commits the changes to the database
    /// I did it like this, so it's MUUUUCH faster
    async fn commit(&self) -> Result<(), DatabaseError>;

    /// Gets a document from the database
    /// Parameters:
    /// - path: the path to the file
    /// Returns:
    /// - Option<Document>: the document if it exists, None if it doesn't
    fn get(&self, path: String) -> Result<Document, DatabaseError>;

    /// Get all Database's documents
    ///
    /// Parameters:
    /// - mode: the content mode (WithContent / WithoutContent)
    ///
    /// Returns:
    /// - Vec<Document>: all the documents
    fn get_all(&self, mode: DocumentGetMode) -> Result<Vec<Document>, DatabaseError>;
}
