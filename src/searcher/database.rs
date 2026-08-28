use std::{io::Error, path::Path};

use tantivy::{
    DateTime, DocAddress, Index, IndexReader, IndexWriter, ReloadPolicy, TantivyDocument,
    TantivyError, Term,
    collector::TopDocs,
    query::{AllQuery, QueryClone, QueryParser, TermQuery},
    schema::{
        DATE_TIME_PRECISION_INDEXED, DateOptions, DateTimePrecision, Field, INDEXED,
        IndexRecordOption, STORED, STRING, Schema, TEXT, Value,
    },
};
use uuid::Uuid;

use crate::{not_err_check, not_null_check, searcher::config::DatabaseConfig, utils::join_path};

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
#[derive(Debug, Clone)]
pub enum DatabaseError {
    CouldNotCreateReaderError(TantivyError),
    SearchFailed(TantivyError),
    NoResult,
    FailedToAddDocument(TantivyError),
    FailedToRemoveDocument(TantivyError),
    IndexCommitError(TantivyError),
    IndexCloseError(TantivyError),
    IndexCreationError(TantivyError),
    IndexWriterCreationError(TantivyError),
    CouldNotCreateDirError,
    ShouldNotHappenError,
    CouldNotOpenIndexError(TantivyError),
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

        let index: Index;
        let schema: Schema;
        if !Path::new(&index_path).exists() {
            let res = std::fs::create_dir_all(&index_path);
            not_err_check!(res, res, Err(DatabaseError::CouldNotCreateDirError));

            schema = Self::build_schema();

            // Create the index
            let index_op = Index::create_in_dir(&index_path, schema.clone());
            if index_op.is_err() {
                return Err(DatabaseError::IndexCreationError(index_op.err().unwrap()));
            }
            index = index_op.unwrap();
        } else {
            let index_op = Index::open_in_dir(&index_path);
            if index_op.is_err() {
                return Err(DatabaseError::CouldNotOpenIndexError(
                    index_op.err().unwrap(),
                ));
            }
            index = index_op.unwrap();
            schema = index.schema();
        }

        // Create the index writer
        let index_writer = index.writer(config.max_db_size);
        not_err_check!(
            index_writer,
            index_writer,
            Err(DatabaseError::IndexWriterCreationError(
                index_writer.err().unwrap(),
            ))
        );

        // Get the path and content fields
        let path_field = schema.get_field("path");
        not_err_check!(
            path_field,
            path_field,
            Err(DatabaseError::ShouldNotHappenError)
        );
        let content_field = schema.get_field("content");
        not_err_check!(
            content_field,
            content_field,
            Err(DatabaseError::ShouldNotHappenError)
        );
        let uuid_field = schema.get_field("uuid");
        not_err_check!(
            uuid_field,
            uuid_field,
            Err(DatabaseError::ShouldNotHappenError)
        );
        let mod_date_field = schema.get_field("moddate");
        not_err_check!(
            mod_date_field,
            mod_date_field,
            Err(DatabaseError::ShouldNotHappenError)
        );

        // Return the database
        return Ok(Self {
            config: config,
            index_writer: index_writer,
            index: index,
            content_field: content_field,
            path_field: path_field,
            uuid_field: uuid_field,
            mod_date_field: mod_date_field,
        });
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
            .try_into();
        not_err_check!(
            reader,
            reader,
            Err(DatabaseError::CouldNotCreateReaderError(
                reader.err().unwrap()
            ))
        );
        return Ok(reader);
    }

    /// This builds the Tantivy schema
    /// Two fields are created: path and content, both are text fields
    /// Returns:
    /// - Schema: the schema
    fn build_schema() -> Schema {
        let mut schema_builder = Schema::builder();
        schema_builder.add_text_field("path", STRING | STORED);
        schema_builder.add_text_field("content", TEXT | STORED);
        schema_builder.add_text_field("uuid", STRING | STORED);
        schema_builder.add_date_field(
            "moddate",
            DateOptions::from(INDEXED)
                .set_stored()
                .set_fast()
                .set_precision(DateTimePrecision::Seconds),
        );
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
        let res = self.add_document_internal(path, content, mod_date, uuid.clone());
        if res.is_err() {
            return Err(DatabaseError::FailedToAddDocument(res.err().unwrap()));
        }
        return Ok(res.unwrap());
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
        let res = self.index_writer.add_document(doc);
        not_err_check!(res, res, Err(res.err().unwrap()));
        self.index_writer.commit()?;
        return Ok(uuid);
    }

    /// Searches the database for a query
    /// Parameters:
    /// - query: the query to search for
    /// - limit: the maximum number of results to return
    /// Returns:
    /// - Vec<Document>: a vector of documents that match the query
    pub fn search(&self, query: String, limit: usize) -> Vec<Document> {
        let reader = self.get_reader();
        not_err_check!(reader, reader, Vec::new());

        let searcher = reader.searcher();
        let query_parser =
            QueryParser::for_index(&self.index, vec![self.path_field, self.content_field]);

        let query = query_parser.parse_query(query.as_str());
        not_err_check!(query, query, Vec::new());

        let top_docs = searcher.search(&query, &TopDocs::with_limit(limit).order_by_score());
        not_err_check!(top_docs, top_docs, Vec::new());

        return top_docs
            .iter()
            .map(|(_, addr)| self.document_from_tantivy_doc(*addr, true))
            .filter(|d| d.is_some())
            .map(|d| d.unwrap())
            .collect();
    }

    /// Utility function to convert a Tantivy document to a Document
    /// Parameters:
    /// - doc: the Tantivy document to convert
    /// Returns:
    /// - Option<Document>: the converted document, or None if the conversion failed
    fn document_from_tantivy_doc(&self, doc: DocAddress, optimized: bool) -> Option<Document> {
        let reader = self.get_reader();
        not_err_check!(reader, reader, None);

        let doc: Result<TantivyDocument, _> = reader.searcher().doc(doc);
        not_err_check!(doc, doc, None);

        not_null_check!(path, doc.get_first(self.path_field), None);
        let path = path.as_str();
        not_null_check!(path, path, None);

        not_null_check!(uuid, doc.get_first(self.uuid_field), None);
        let uuid = uuid.as_str();
        not_null_check!(uuid, uuid, None);

        not_null_check!(mod_date, doc.get_first(self.mod_date_field), None);
        let mod_date = mod_date.as_datetime();
        not_null_check!(mod_date, mod_date, None);

        if !optimized {
            not_null_check!(content, doc.get_first(self.content_field), None);
            let content = content.as_str();
            not_null_check!(content, content, None);

            return Some(Document::new(
                String::from(path),
                Some(String::from(content)),
                String::from(uuid),
                mod_date.clone(),
            ));
        } else {
            return Some(Document::new(
                String::from(path),
                None,
                String::from(uuid),
                mod_date.clone(),
            ));
        }
    }

    /// Removes a document from the database
    /// Parameters:
    /// - path: the path of the document to remove
    /// Returns:
    /// - Result<(), ()>: Ok(()) if the document was removed, Err(()) if the document was not found
    pub fn remove_document(&mut self, path: String) -> Result<String, DatabaseError> {
        let document = self.get(path.clone());
        not_err_check!(document, document, Err(document.err().unwrap()));
        let term_query = TermQuery::new(
            Term::from_field_text(self.uuid_field, &document.uuid),
            IndexRecordOption::Basic,
        );

        let res = self.index_writer.delete_query(term_query.box_clone());
        not_err_check!(
            res,
            res,
            Err(DatabaseError::FailedToRemoveDocument(res.err().unwrap()))
        );

        let res = self.index_writer.commit();
        not_err_check!(
            res,
            res,
            Err(DatabaseError::IndexCommitError(res.err().unwrap()))
        );

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
        let old_doc_uuid = self.remove_document(path.clone());
        not_err_check!(old_doc_uuid, old_doc_uuid, Err(old_doc_uuid.err().unwrap()));
        let res = self.add_document_internal(path, new_content, mod_date, old_doc_uuid);
        not_err_check!(
            res,
            res,
            Err(DatabaseError::FailedToAddDocument(res.err().unwrap()))
        );

        return Ok(());
    }

    /// Gets a document from the database
    /// Parameters:
    /// - path: the path to the file
    /// Returns:
    /// - Option<Document>: the document if it exists, None if it doesn't
    pub fn get(&self, path: String) -> Result<Document, DatabaseError> {
        let reader = self.get_reader();
        not_err_check!(reader, reader, Err(reader.err().unwrap()));

        let searcher = reader.searcher();
        let term = Term::from_field_text(self.path_field, path.as_str());
        let term_query = TermQuery::new(term, IndexRecordOption::Basic);
        let top_docs = searcher.search(&term_query, &TopDocs::with_limit(1).order_by_score());

        not_err_check!(
            top_docs,
            top_docs,
            Err(DatabaseError::SearchFailed(top_docs.err().unwrap()))
        );

        let the_only_doc = top_docs
            .iter()
            .map(|(_, addr)| self.document_from_tantivy_doc(*addr, false))
            .filter(|doc| doc.is_some())
            .map(|doc| doc.unwrap())
            .find(|doc| doc.path == path);

        if the_only_doc.is_some() {
            return Ok(the_only_doc.unwrap());
        } else {
            return Err(DatabaseError::NoResult);
        }
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
    pub fn get_all(&self) -> Vec<Document> {
        let reader = self.get_reader();
        not_err_check!(reader, reader, Vec::new());

        let searcher = reader.searcher();
        let all_query = AllQuery {};

        let top_docs =
            searcher.search(&all_query, &TopDocs::with_limit(MAX_LIMIT).order_by_score());

        not_err_check!(top_docs, top_docs, Vec::new());

        return top_docs
            .iter()
            .map(|(_, addr)| self.document_from_tantivy_doc(*addr, true))
            .filter(|doc| doc.is_some())
            .map(|doc| doc.unwrap())
            .collect();
    }

    pub fn close(self) -> Result<(), DatabaseError> {
        let res = self.index_writer.wait_merging_threads();
        not_err_check!(
            res,
            res,
            Err(DatabaseError::IndexCloseError(res.err().unwrap()))
        );
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

    use crate::searcher::database;

    use super::*;
    use chrono;
    use tempdir::TempDir;

    static FIRST_FILE_CONTENT: &str = "Catgirls are the best. YAY!";
    static SECOND_FILE_CONTENT: &str = "...Hatsune Miku... oo-ee-oo. YAY!";

    // Saved for the index unittest
    // fn write_to_file(path: OsString, content: &str) {
    //     let file = File::create(path);
    //     assert!(file.is_ok());
    //     let res = file.unwrap().write_all(content.as_bytes());
    //     assert!(res.is_ok())
    // }

    // fn generate_test_datas(path: &str) {
    //     let file1_path = join_paths([path, "file1.txt"]);
    //     let dir1_path = join_paths([path, "dir1"]);
    //     let file2_path = join_paths([path, "file2.txt"]);
    //     assert!(file1_path.is_ok());
    //     assert!(dir1_path.is_ok());
    //     assert!(file2_path.is_ok());
    //     write_to_file(file1_path.unwrap(), FIRST_FILE_CONTENT);
    //     let res = create_dir(dir1_path.unwrap());
    //     assert!(res.is_ok());
    //     write_to_file(file2_path.unwrap(), SECOND_FILE_CONTENT);
    // }

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

        let doc2 = database.get(String::from("file1.txt"));
        assert!(doc2.is_err());
        let _ = temp_dir.close();
    }

    /// Tests the search function of the database
    #[test]
    fn test_search() {
        let (database, temp_dir) = setup();
        let results = database.search(String::from("catgirls"), 10);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].path, String::from("file1.txt"));
        assert_eq!(results[0].content, None);

        let no_results = database.search(String::from("NOO"), 10);
        assert_eq!(no_results.len(), 0);

        let results2 = database.search(String::from("YAY"), 10);
        assert_eq!(results2.len(), 2);
        let _ = temp_dir.close();
    }

    /// Tests the get_all function of the database
    #[test]
    fn test_all() {
        let (database, temp_dir) = setup();

        let results = database.get_all();

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
