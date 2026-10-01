use std::{path::Path, sync::Arc};

use application::searcher::{error::DatabaseError, traits::database::DatabaseProvider};
use async_trait::async_trait;
use domain::searcher::{
    config::DatabaseConfig, document::Document, error::DatabaseDomainError,
    get_mode::DocumentGetMode, search_query::SearchQuery,
};
use tantivy::{
    DateTime, DocAddress, Index, IndexReader, IndexWriter, TantivyDocument, Term,
    collector::TopDocs,
    query::{AllQuery, QueryClone, QueryParser, TermQuery},
    schema::{
        DateOptions, DateTimePrecision, Field, INDEXED, IndexRecordOption, STORED, STRING, Schema,
        TEXT, Value,
    },
};
use tokio::sync::RwLock;

use crate::searcher::tantivy::errors::TantivyDatabaseError;

static MAX_LIMIT: usize = 9_999_999;

/// The database is the main search engine of the program.
/// It uses https://github.com/quickwit-oss/tantivy under the hood.
/// It contains a folder path, database save folder, config, index writer (the db), and a schema
#[derive(Clone)]
pub struct TantivyDatabase {
    /// The write head for the index, used to add documents to the database
    index_writer: Arc<RwLock<IndexWriter>>,
    /// The index
    index: Arc<Index>,
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

impl TantivyDatabase {
    /// Creates a new database
    /// Parameters:
    /// - folder: the folder to search in
    /// - config: the config to use
    /// Returns:
    /// - Ok(Database): the database if everything went well
    /// - Err(DatabaseInitError): the error if something went wrong
    pub fn new(folder: String, config: DatabaseConfig) -> Result<Self, TantivyDatabaseError> {
        // Create index path
        let index_path = Path::new(folder.as_str()).join(config.database_folder_prefix.as_str());

        let schema: Schema;
        let index = if !index_path.exists() {
            std::fs::create_dir_all(&index_path)
                .map_err(|_| TantivyDatabaseError::CouldNotCreateDirError)?;

            schema = Self::build_schema();

            // Create the index
            let index = Index::create_in_dir(&index_path, schema.clone())
                .map_err(|e| TantivyDatabaseError::IndexCreationError(e))?;
            index
        } else {
            let index = Index::open_in_dir(&index_path)
                .map_err(|e| TantivyDatabaseError::CouldNotOpenIndexError(e))?;
            schema = index.schema();
            index
        };

        // Create the index writer
        let index_writer = index
            .writer(config.max_db_size)
            .map_err(|e| TantivyDatabaseError::IndexWriterCreationError(e))?;

        // Get the path and content fields
        let path_field = schema
            .get_field("path")
            .map_err(|_| TantivyDatabaseError::ShouldNotHappenError)?;
        let content_field = schema
            .get_field("content")
            .map_err(|_| TantivyDatabaseError::ShouldNotHappenError)?;
        let uuid_field = schema
            .get_field("uuid")
            .map_err(|_| TantivyDatabaseError::ShouldNotHappenError)?;
        let mod_date_field = schema
            .get_field("moddate")
            .map_err(|_| TantivyDatabaseError::ShouldNotHappenError)?;

        let index_path_str = index_path.to_string_lossy().to_string();

        // Return the database
        return Ok(Self {
            index_writer: Arc::new(RwLock::new(index_writer)),
            index: Arc::new(index),
            content_field: content_field,
            path_field: path_field,
            uuid_field: uuid_field,
            mod_date_field: mod_date_field,
            index_path: index_path_str,
        });
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

    /// Since it's also used by update, we create an internal version that doesn't change it's UUID
    async fn add_document_internal(
        &self,
        path: String,
        content: String,
        mod_date: i64,
        uuid: String,
    ) -> Result<String, TantivyDatabaseError> {
        let mut doc = TantivyDocument::default();

        doc.add_text(self.path_field, path);
        doc.add_text(self.content_field, content);
        doc.add_text(self.uuid_field, uuid.clone());
        doc.add_date(self.mod_date_field, DateTime::from_timestamp_secs(mod_date));

        self.index_writer
            .write()
            .await
            .add_document(doc)
            .map_err(|e| TantivyDatabaseError::FailedToAddDocument(e))?;

        return Ok(uuid);
    }

    /// Builds the index reader
    /// Returns:
    /// - Ok(IndexReader): the index reader if everything went well
    /// - Err(DatabaseInitError): the error if something went wrong
    fn get_reader(&self) -> Result<IndexReader, TantivyDatabaseError> {
        let reader = self
            .index
            .reader_builder()
            // .reload_policy(ReloadPolicy::OnCommitWithDelay)
            .try_into()
            .map_err(|e| TantivyDatabaseError::CouldNotCreateReaderError(e))?;

        return Ok(reader);
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

        return Some(Document::from_parts(
            String::from(path),
            content,
            String::from(uuid),
            mod_date.into_timestamp_secs(),
        ));
    }
}

#[async_trait]
impl DatabaseProvider for TantivyDatabase {
    fn get_index_path(&self) -> String {
        return self.index_path.clone();
    }

    async fn add_document(&self, document: Document) -> Result<String, DatabaseError> {
        let operation_result = self
            .add_document_internal(
                document.path,
                document.content.unwrap_or_default(),
                document.modification_date,
                document.uuid,
            )
            .await
            .map_err(|e| DatabaseError::Database(e.to_string()))?;

        return Ok(operation_result);
    }

    fn search(&self, search_query: SearchQuery) -> Result<Vec<Document>, DatabaseError> {
        let reader = self
            .get_reader()
            .map_err(|e| DatabaseError::Database(e.to_string()))?;

        let searcher = reader.searcher();
        let query_parser =
            QueryParser::for_index(&self.index, vec![self.path_field, self.content_field]);

        let query = query_parser
            .parse_query(&search_query.query.as_str())
            .map_err(|e| {
                DatabaseError::Database(
                    TantivyDatabaseError::CouldNotParseQuery(search_query.query, e.to_string())
                        .to_string(),
                )
            })?;

        let top_docs = searcher
            .search(
                &query,
                &TopDocs::with_limit(search_query.limit).order_by_score(),
            )
            .map_err(|e| {
                DatabaseError::Database(TantivyDatabaseError::SearchFailed(e).to_string())
            })?;

        return Ok(top_docs
            .iter()
            .map(|(_, addr)| self.document_from_tantivy_doc(*addr, true))
            .filter(|d| d.is_some())
            .map(|d| d.unwrap())
            .collect());
    }

    async fn remove_document(&self, path: String) -> Result<String, DatabaseError> {
        let document = self.get(path.clone())?;

        let term_query = TermQuery::new(
            Term::from_field_text(self.uuid_field, &document.uuid),
            IndexRecordOption::Basic,
        );

        let index_writer = self.index_writer.write().await;

        index_writer
            .delete_query(term_query.box_clone())
            .map_err(|e| {
                DatabaseError::Database(TantivyDatabaseError::FailedToRemoveDocument(e).to_string())
            })?;

        return Ok(document.uuid);
    }

    async fn update_document(
        &self,
        path: String,
        new_document: Document,
    ) -> Result<(), DatabaseError> {
        let old_doc_uuid = self.remove_document(path.clone()).await?;

        self.add_document_internal(
            path,
            new_document.content.unwrap_or_default(),
            new_document.modification_date,
            old_doc_uuid,
        )
        .await
        .map_err(|e| DatabaseError::Database(e.to_string()))?;

        return Ok(());
    }

    async fn commit(&self) -> Result<(), DatabaseError> {
        let mut index_writer = self.index_writer.write().await;

        index_writer.commit().map_err(|e| {
            DatabaseError::Database(TantivyDatabaseError::IndexCommitError(e).to_string())
        })?;

        return Ok(());
    }

    fn get(&self, path: String) -> Result<Document, DatabaseError> {
        let reader = self
            .get_reader()
            .map_err(|e| DatabaseError::Database(e.to_string()))?;

        let searcher = reader.searcher();

        let term = Term::from_field_text(self.path_field, path.as_str());
        let term_query = TermQuery::new(term, IndexRecordOption::Basic);

        let top_docs = searcher
            .search(&term_query, &TopDocs::with_limit(1).order_by_score())
            .map_err(|e| {
                DatabaseError::Database(TantivyDatabaseError::SearchFailed(e).to_string())
            })?;

        let the_only_doc = top_docs
            .iter()
            .map(|(_, addr)| self.document_from_tantivy_doc(*addr, false))
            .filter(|doc| doc.is_some())
            .map(|doc| doc.unwrap())
            .find(|doc| doc.path == path);

        return the_only_doc.ok_or(DatabaseError::DomainError(DatabaseDomainError::NoResult));
    }

    fn get_all(&self, mode: DocumentGetMode) -> Result<Vec<Document>, DatabaseError> {
        let reader = self
            .get_reader()
            .map_err(|e| DatabaseError::Database(e.to_string()))?;

        let searcher = reader.searcher();
        let all_query = AllQuery {};

        let top_docs = searcher
            .search(&all_query, &TopDocs::with_limit(MAX_LIMIT).order_by_score())
            .map_err(|e| {
                DatabaseError::Database(TantivyDatabaseError::SearchFailed(e).to_string())
            })?;

        return Ok(top_docs
            .iter()
            .map(|(_, addr)| {
                self.document_from_tantivy_doc(
                    *addr,
                    match mode {
                        DocumentGetMode::WithContent => false,
                        DocumentGetMode::WithoutContent => true,
                    },
                )
            })
            .filter(|doc| doc.is_some())
            .map(|doc| doc.unwrap())
            .collect());
    }
}
