use std::{collections::HashMap, sync::Arc};

use domain::searcher::{document::Document, get_mode::DocumentGetMode};

use crate::{
    indexer::traits::registry::{FileIndexerRegistry, IndexerId},
    searcher::error::DatabaseError,
};

pub struct DatabaseGet {
    registry: Arc<dyn FileIndexerRegistry>,
}

impl DatabaseGet {
    pub fn new(registry: Arc<dyn FileIndexerRegistry>) -> Self {
        return Self { registry: registry };
    }

    /// Lists the folders with their IDs
    ///
    /// Returns:
    /// - HashMap key => folder ID, value => folder's path
    pub async fn list_folders(&self) -> HashMap<IndexerId, String> {
        return self.registry.list_indexers().await;
    }

    /// Returns the folder path of an indexer
    ///
    /// Returns:
    /// - the path
    pub async fn get_folder(&self, for_database: IndexerId) -> Option<String> {
        return self.registry.get_folder(for_database).await;
    }

    /// Returns the database folder of an indexer
    ///
    /// Returns:
    /// - database folder (default: get_folder() + ".ffsearch")
    pub async fn get_db_folder(&self, for_database: IndexerId) -> Option<String> {
        let database = self.registry.get_database(for_database).await?;

        return Some(database.get_index_path());
    }

    /// Gets a document from the database
    /// Parameters:
    /// - path: the path to the file
    /// Returns:
    /// - Option<Document>: the document if it exists, None if it doesn't
    pub async fn get_one(
        &self,
        for_database: IndexerId,
        path: String,
    ) -> Result<Document, DatabaseError> {
        let database = self
            .registry
            .get_database(for_database)
            .await
            .ok_or(DatabaseError::Database(String::from("Indexer deleted")))?;

        return database.get(path);
    }

    /// Get all Database's documents WITH CONTENTS
    ///
    /// Returns:
    /// - Vec<Document>: all the documents
    pub async fn get_all_with_content(
        &self,
        for_database: IndexerId,
    ) -> Result<Vec<Document>, DatabaseError> {
        let database = self
            .registry
            .get_database(for_database)
            .await
            .ok_or(DatabaseError::Database(String::from("Indexer deleted")))?;

        return database.get_all(DocumentGetMode::WithContent);
    }

    /// Get all Database's documents WITHOUT CONTENT
    ///
    /// Returns:
    /// - Vec<Document>: all the documents
    pub async fn get_all_without_content(
        &self,
        for_database: IndexerId,
    ) -> Result<Vec<Document>, DatabaseError> {
        let database = self
            .registry
            .get_database(for_database)
            .await
            .ok_or(DatabaseError::Database(String::from("Indexer deleted")))?;

        return database.get_all(DocumentGetMode::WithoutContent);
    }
}
