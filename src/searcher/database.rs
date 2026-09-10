use std::path::Path;

use tantivy::{
    DateTime, DocAddress, Index, IndexReader, IndexWriter, TantivyDocument, TantivyError, Term,
    collector::TopDocs,
    query::{AllQuery, QueryClone, QueryParser, TermQuery},
    schema::{
        DateOptions, DateTimePrecision, Field, INDEXED, IndexRecordOption, STORED, STRING, Schema,
        TEXT, Value,
    },
};
use thiserror::Error;
use uuid::Uuid;

use crate::{searcher::config::DatabaseConfig, utils::join_path};

/// Max limit for search results, used because we need to.
static MAX_LIMIT: usize = 9_999_999;

/// The Document struct is a basic struct that represents a Document with it's title and content, it's unique ID, and the latest modification date.
/// It is used to represent a document in the database.
#[derive(Debug, Clone)]
pub struct Document {
    /// The title
    pub path: String,
    /// The content
    pub content: Option<String>,
    /// The uuid
    pub uuid: String,
    /// The creation date
    pub modification_date: DateTime,
}

impl Document {
    /// Creates a new document
    /// Parameters:
    /// - title: the title
    /// - content: the content
    pub fn new(
        path: String,
        content: Option<String>,
        uuid: String,
        modification_date: DateTime,
    ) -> Self {
        return Self {
            path: path,
            content: content,
            uuid: uuid,
            modification_date: modification_date,
        };
    }
}

/// This enum represents the possible errors that can occur when using the database
#[derive(Debug, Clone, Error)]
pub enum DatabaseError {
    #[error("Could not create db reader: {0}")]
    CouldNotCreateReaderError(TantivyError),
    #[error("Search failed: {0}")]
    SearchFailed(TantivyError),
    #[error("No result, expected one but got none")]
    NoResult,
    #[error("Failed to add document: {0}")]
    FailedToAddDocument(TantivyError),
    #[error("Failed to remove document: {0}")]
    FailedToRemoveDocument(TantivyError),
    #[error("Failed to save changes: {0}")]
    IndexCommitError(TantivyError),
    #[error("Failed to close db: {0}")]
    IndexCloseError(TantivyError),
    #[error("Failed to create index: {0}")]
    IndexCreationError(TantivyError),
    #[error("Failed to create index writer: {0}")]
    IndexWriterCreationError(TantivyError),
    #[error("Could not create db directory")]
    CouldNotCreateDirError,
    #[error("Should not happen")]
    ShouldNotHappenError,
    #[error("Could not open index: {0}")]
    CouldNotOpenIndexError(TantivyError),
    #[error("Could not parse the query: {0}, error: {1}")]
    CouldNotParseQuery(String, String),
}

/// The database is the main search engine of the program.
/// It uses https://github.com/quickwit-oss/tantivy under the hood.
/// It contains a folder path, database save folder, config, index writer (the db), and a schema
pub struct Database {
    /// Config
    config: DatabaseConfig,
    /// The write head for the index, used to add documents to the database
    index_writer: IndexWriter,
    /// The index
    index: Index,
    /// The path fields:
    path_field: Field,
    /// The content field
    content_field: Field,
    /// The unique ID field
    uuid_field: Field,
    /// The modification date field
    mod_date_field: Field,
    /// The path to the index
    index_path: String,
}

impl Database {
    /// Creates a new database
    /// Parameters:
    /// - folder: the folder to search in
    /// - config: the config to use
    /// Returns:
    /// - Ok(Database): the database if everything went well
    /// - Err(DatabaseInitError): the error if something went wrong
    pub fn new(folder: String, config: DatabaseConfig) -> Result<Self, DatabaseError> {
        // Create index path
        let index_path = join_path(folder.as_str(), config.database_folder_prefix.as_str());

        let schema: Schema;
        let index = if !Path::new(&index_path).exists() {
            std::fs::create_dir_all(&index_path)
                .map_err(|_| DatabaseError::CouldNotCreateDirError)?;

            schema = Self::build_schema();

            // Create the index
            let index = Index::create_in_dir(&index_path, schema.clone())
                .map_err(|e| DatabaseError::IndexCreationError(e))?;
            index
        } else {
            let index = Index::open_in_dir(&index_path)
                .map_err(|e| DatabaseError::CouldNotOpenIndexError(e))?;
            schema = index.schema();
            index
        };

        // Create the index writer
        let index_writer = index
            .writer(config.max_db_size)
            .map_err(|e| DatabaseError::IndexWriterCreationError(e))?;

        // Get the path and content fields
        let path_field = schema
            .get_field("path")
            .map_err(|_| DatabaseError::ShouldNotHappenError)?;
        let content_field = schema
            .get_field("content")
            .map_err(|_| DatabaseError::ShouldNotHappenError)?;
        let uuid_field = schema
            .get_field("uuid")
            .map_err(|_| DatabaseError::ShouldNotHappenError)?;
        let mod_date_field = schema
            .get_field("moddate")
            .map_err(|_| DatabaseError::ShouldNotHappenError)?;

        // Return the database
        return Ok(Self {
            config: config,
            index_writer: index_writer,
            index: index,
            content_field: content_field,
            path_field: path_field,
            uuid_field: uuid_field,
            mod_date_field: mod_date_field,
            index_path: index_path,
        });
    }

    /// Returns the index path
    pub fn get_index_path(&self) -> String {
        return self.index_path.clone();
    }

    /// Builds the index reader
    /// Returns:
    /// - Ok(IndexReader): the index reader if everything went well
    /// - Err(DatabaseInitError): the error if something went wrong
    fn get_reader(&self) -> Result<IndexReader, DatabaseError> {
        let reader = self
            .index
            .reader_builder()
            // .reload_policy(ReloadPolicy::OnCommitWithDelay)
            .try_into()
            .map_err(|e| DatabaseError::CouldNotCreateReaderError(e))?;

        return Ok(reader);
    }

    /// This builds the Tantivy schema
    /// Two fields are created: path and content, both are text fields
    /// Returns:
    /// - Schema: the schema
    fn build_schema() -> Schema {
        let mut schema_builder = Schema::builder();

        let date_options = DateOptions::from(INDEXED)
            .set_stored()
            .set_fast()
            .set_precision(DateTimePrecision::Seconds);

        schema_builder.add_text_field("path", STRING | STORED);
        schema_builder.add_text_field("content", TEXT | STORED);
        schema_builder.add_text_field("uuid", STRING | STORED);
        schema_builder.add_date_field("moddate", date_options);

        return schema_builder.build();
    }

    /// Adds a document to the database
    /// Parameters:
    /// - path: the path of the document
    pub fn add_document(
        &mut self,
        path: String,
        content: String,
        mod_date: i64,
    ) -> Result<String, DatabaseError> {
        let uuid = Uuid::new_v4().to_string();

        return self
            .add_document_internal(path, content, mod_date, uuid.clone())
            .map_err(|e| DatabaseError::FailedToAddDocument(e));
    }

    /// Since it's also used by update, we create an internal version that doesn't change it's UUID
    fn add_document_internal(
        &mut self,
        path: String,
        content: String,
        mod_date: i64,
        uuid: String,
    ) -> Result<String, TantivyError> {
        let mut doc = TantivyDocument::default();

        doc.add_text(self.path_field, path);
        doc.add_text(self.content_field, content);
        doc.add_text(self.uuid_field, uuid.clone());
        doc.add_date(self.mod_date_field, DateTime::from_timestamp_secs(mod_date));

        self.index_writer.add_document(doc)?;

        return Ok(uuid);
    }

    /// Searches the database for a query
    /// Parameters:
    /// - query: the query to search for
    /// - limit: the maximum number of results to return
    /// Returns:
    /// - Vec<Document>: a vector of documents that match the query
    pub fn search(&self, query: String, limit: usize) -> Result<Vec<Document>, DatabaseError> {
        let reader = self.get_reader()?;

        let searcher = reader.searcher();
        let query_parser =
            QueryParser::for_index(&self.index, vec![self.path_field, self.content_field]);

        let query = query_parser
            .parse_query(query.as_str())
            .map_err(|e| DatabaseError::CouldNotParseQuery(query, e.to_string()))?;

        let top_docs = searcher
            .search(&query, &TopDocs::with_limit(limit).order_by_score())
            .map_err(|e| DatabaseError::SearchFailed(e))?;

        return Ok(top_docs
            .iter()
            .map(|(_, addr)| self.document_from_tantivy_doc(*addr, true))
            .filter(|d| d.is_some())
            .map(|d| d.unwrap())
            .collect());
    }

    /// Utility function to convert a Tantivy document to a Document
    /// Parameters:
    /// - doc: the Tantivy document to convert
    /// Returns:
    /// - Option<Document>: the converted document, or None if the conversion failed
    fn document_from_tantivy_doc(&self, doc: DocAddress, optimized: bool) -> Option<Document> {
        let reader = self.get_reader().ok()?;

        let doc: TantivyDocument = reader.searcher().doc(doc).ok()?;

        let path = doc.get_first(self.path_field)?;
        let path = path.as_str()?;

        let uuid = doc.get_first(self.uuid_field)?;
        let uuid = uuid.as_str()?;

        let mod_date = doc.get_first(self.mod_date_field)?;
        let mod_date = mod_date.as_datetime()?;

        let content = if optimized {
            None
        } else {
            let content = doc.get_first(self.content_field)?;
            let content = content.as_str()?;
            Some(content.to_string())
        };

        return Some(Document::new(
            String::from(path),
            content,
            String::from(uuid),
            mod_date.clone(),
        ));
    }

    /// Removes a document from the database
    /// Parameters:
    /// - path: the path of the document to remove
    ///
    /// Returns:
    /// - Result<String, DatabaseError>: Ok(String) the uuid of the document that was removed, Err(DatabaseError) if there's an error
    pub fn remove_document(&mut self, path: String) -> Result<String, DatabaseError> {
        let document = self.get(path.clone())?;

        let term_query = TermQuery::new(
            Term::from_field_text(self.uuid_field, &document.uuid),
            IndexRecordOption::Basic,
        );

        self.index_writer
            .delete_query(term_query.box_clone())
            .map_err(|e| DatabaseError::FailedToRemoveDocument(e))?;

        return Ok(document.uuid);
    }

    /// Removes a document from the database
    /// Parameters:
    /// - path: the path of the document to remove
    /// Returns:
    /// - Result<(), ()>: Ok(()) if the document was removed, Err(()) if the document was not found
    pub fn update_document(
        &mut self,
        path: String,
        new_content: String,
        mod_date: i64,
    ) -> Result<(), DatabaseError> {
        let old_doc_uuid = self.remove_document(path.clone())?;

        self.add_document_internal(path, new_content, mod_date, old_doc_uuid)
            .map_err(|e| DatabaseError::FailedToAddDocument(e))?;

        return Ok(());
    }

    /// Commits the changes to the database
    /// I did it like this, so it's MUUUUCH faster
    pub fn commit(&mut self) -> Result<(), DatabaseError> {
        self.index_writer
            .commit()
            .map_err(|e| DatabaseError::IndexCommitError(e))?;
        return Ok(());
    }

    /// Gets a document from the database
    /// Parameters:
    /// - path: the path to the file
    /// Returns:
    /// - Option<Document>: the document if it exists, None if it doesn't
    pub fn get(&self, path: String) -> Result<Document, DatabaseError> {
        let reader = self.get_reader()?;

        let searcher = reader.searcher();

        let term = Term::from_field_text(self.path_field, path.as_str());
        let term_query = TermQuery::new(term, IndexRecordOption::Basic);

        let top_docs = searcher
            .search(&term_query, &TopDocs::with_limit(1).order_by_score())
            .map_err(|e| DatabaseError::SearchFailed(e))?;

        let the_only_doc = top_docs
            .iter()
            .map(|(_, addr)| self.document_from_tantivy_doc(*addr, false))
            .filter(|doc| doc.is_some())
            .map(|doc| doc.unwrap())
            .find(|doc| doc.path == path);

        return the_only_doc.ok_or(DatabaseError::NoResult);
    }

    /// Gets the config of the Database
    /// Returns:
    /// - DatabaseConfig: the config
    pub fn get_config(&self) -> DatabaseConfig {
        return self.config.clone();
    }

    /// Get all Database's documents
    /// Returns:
    /// - Vec<Document>: all the documents WITHOUT THEIR CONENT
    pub fn get_all(&self, optimized: bool) -> Result<Vec<Document>, DatabaseError> {
        let reader = self.get_reader()?;

        let searcher = reader.searcher();
        let all_query = AllQuery {};

        let top_docs = searcher
            .search(&all_query, &TopDocs::with_limit(MAX_LIMIT).order_by_score())
            .map_err(|e| DatabaseError::SearchFailed(e))?;

        return Ok(top_docs
            .iter()
            .map(|(_, addr)| self.document_from_tantivy_doc(*addr, optimized))
            .filter(|doc| doc.is_some())
            .map(|doc| doc.unwrap())
            .collect());
    }

    pub fn close(self) -> Result<(), DatabaseError> {
        self.index_writer
            .wait_merging_threads()
            .map_err(|e| DatabaseError::IndexCloseError(e))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    // use std::{
    //     env::join_paths,
    //     ffi::OsString,
    //     fs::{File, create_dir},
    //     io::Write,
    // };

    use super::*;
    use chrono;
    use tempdir::TempDir;

    static FIRST_FILE_CONTENT: &str = "Catgirls are the best. YAY!";
    static SECOND_FILE_CONTENT: &str = "...Hatsune Miku... oo-ee-oo. YAY!";

    /// Setup function for the tests
    fn setup() -> (Database, TempDir) {
        let temp_dir = TempDir::new("ffsearch_unittest_");
        assert!(temp_dir.is_ok());
        let temp_dir = temp_dir.unwrap();
        let temp_dir_path = temp_dir.path().to_str();
        assert!(temp_dir_path.is_some());
        // generate_test_datas(temp_dir_path.unwrap());
        let db = Database::new(String::from(temp_dir_path.unwrap()), DatabaseConfig::new());
        assert!(db.is_ok());
        let mut db = db.unwrap();
        assert!(
            db.add_document(
                String::from("file1.txt"),
                String::from(FIRST_FILE_CONTENT),
                chrono::offset::Local::now().timestamp()
            )
            .is_ok()
        );
        assert!(
            db.add_document(
                String::from("dir1/file2.txt"),
                String::from(SECOND_FILE_CONTENT),
                chrono::offset::Local::now().timestamp()
            )
            .is_ok()
        );
        db.commit().unwrap();
        return (db, temp_dir);
    }

    /// Tests the get and add_document functions of the database
    #[test]
    fn test_get_and_add_document() {
        let (database, temp_dir) = setup();

        let doc = database.get(String::from("file1.txt"));
        assert!(doc.is_ok());
        assert_eq!(doc.unwrap().content.unwrap(), FIRST_FILE_CONTENT);

        let not_found_doc = database.get(String::from("file2.txt"));
        assert!(match not_found_doc.clone().err().unwrap() {
            DatabaseError::NoResult => true,
            _ => false,
        });

        let doc2 = database.get(String::from("dir1/file2.txt"));
        assert!(doc2.is_ok());
        assert_eq!(doc2.unwrap().content.unwrap(), SECOND_FILE_CONTENT);
        let _ = temp_dir.close();
    }

    /// Tests the remove_document function of the database
    #[test]
    fn test_remove_document() {
        let (mut database, temp_dir) = setup();
        let doc = database.get(String::from("file1.txt"));
        assert!(doc.is_ok());

        let _ = database.remove_document(String::from("file1.txt"));

        database.commit().unwrap();

        let doc2 = database.get(String::from("file1.txt"));
        assert!(doc2.is_err());
        let _ = temp_dir.close();
    }

    /// Tests the search function of the database
    #[test]
    fn test_search() {
        let (database, temp_dir) = setup();
        let results = database.search(String::from("catgirls"), 10).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].path, String::from("file1.txt"));
        assert_eq!(results[0].content, None);

        let no_results = database.search(String::from("NOO"), 10).unwrap();
        assert_eq!(no_results.len(), 0);

        let results2 = database.search(String::from("YAY"), 10).unwrap();
        assert_eq!(results2.len(), 2);
        let _ = temp_dir.close();
    }

    /// Tests the get_all function of the database
    #[test]
    fn test_all() {
        let (database, temp_dir) = setup();

        let results = database.get_all(true).unwrap();

        assert_eq!(results.len(), 2);

        let _ = temp_dir.close();
    }

    #[test]
    fn test_two_times_same_folder() {
        let (mut database, temp_dir) = setup();
        let _ = database.add_document(
            String::from("file1.txt"),
            String::from(FIRST_FILE_CONTENT),
            0,
        );
        let _ = database.add_document(
            String::from("file2.txt"),
            String::from(SECOND_FILE_CONTENT),
            0,
        );
        database.commit().unwrap();
        let temp_dir_path = temp_dir.path().to_str().unwrap().to_string();
        assert!(database.close().is_ok());
        let database2 = Database::new(String::from(temp_dir_path), DatabaseConfig::new());

        assert!(database2.is_ok());
        let database2 = database2.unwrap();

        let res = database2.get(String::from("file1.txt"));
        assert!(res.is_ok());

        let _ = temp_dir.close();
    }
}
