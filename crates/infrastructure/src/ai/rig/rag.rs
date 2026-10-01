use std::sync::Arc;

use application::searcher::traits::database::DatabaseProvider;
use domain::{ai::rag_config::RagConfig, searcher::get_mode::DocumentGetMode};
use lancedb::{Connection, Table, index::vector::IvfPqIndexBuilder};
use rig::{
    embeddings::{Embedding, EmbeddingModel, EmbeddingsBuilder},
    serde_json,
    vector_store::{
        TopNResults, VectorSearchRequest, VectorStoreError, VectorStoreIndexDyn,
        request::{DynamicSearchFilter, Filter},
    },
    wasm_compat::WasmBoxedFuture,
};
use rig_lancedb::{LanceDBFilter, LanceDbVectorIndex, SearchParams};

use crate::ai::rig::{embed_document::EmbedDocument, errors::AIError, rig_utils::as_record_batch};

/// The storage for RAG DB
///
/// This struct manages the storage for RAG embeddings using LanceDB.
///
/// Type parameters:
/// - Model: The embedding model to use for RAG embeddings. (ex. rig::provider::ollama::EmbeddingModel)
pub struct RagStorage<Model: EmbeddingModel + Clone + Sync + Send + 'static> {
    /// The LanceDB connection as Database for the RAG.
    /// It's a local folder
    db: Connection,
    /// The LanceDB vector index for the RAG embeddings.
    index: Option<LanceDbVectorIndex<Model>>,
    /// The embedding model to use for RAG embeddings.
    model: Model,
    /// The config
    config: RagConfig,
}

impl<Model> RagStorage<Model>
where
    Model: EmbeddingModel + Clone + Sync + Send + 'static,
{
    /// Creates a new RagStorage with the index path and the model.
    ///
    /// Parameters:
    /// - index_path: The path to ".ffsearch" folder (or other prefix)
    /// - model: The embedding model to use for RAG embeddings.
    pub async fn new(
        embeddings_path: String,
        model: Model,
        config: RagConfig,
    ) -> Result<Self, AIError> {
        let db = lancedb::connect(&embeddings_path)
            .execute()
            .await
            .map_err(|e| AIError::ConnectToStorageError(e))?;

        return Ok(Self {
            db: db,
            index: None,
            model: model,
            config: config,
        });
    }

    /// Loads the index from the database into the index field.
    /// First loads the table, then creates LanceDbVectorIndex from it.
    ///
    /// Returns:
    /// - Ok(()) if the index was successfully loaded.
    /// - Err(AIError) if an error occurred.
    pub async fn load_index(&mut self) -> Result<(), AIError> {
        let table = self
            .db
            .open_table("files")
            .execute()
            .await
            .map_err(|_| AIError::OpenTableError)?;

        let search_params = SearchParams::default();
        let vector_index = LanceDbVectorIndex::new(table, self.model.clone(), "id", search_params)
            .await
            .map_err(|e| AIError::Other(e.to_string()))?;

        self.index = Some(vector_index);

        return Ok(());
    }

    /// Create the tables, indexes, and embeddings from the database.
    ///
    /// Parameters:
    /// - db: the Database to initialize from.
    ///
    /// Returns:
    /// - Ok(()) if the storage was successfully initialized.
    /// - Err(AIError) if an error occurred.
    pub async fn init_from_db(&mut self, db: Arc<dyn DatabaseProvider>) -> Result<(), AIError> {
        // Get all the documents from the DB and split them into batches (See RAG_MAX_BATCH_SIZE).
        let documents_raw = db
            .get_all(DocumentGetMode::WithContent)
            .map_err(|e| AIError::FailedToGetAllDocuments(e))?;

        let mut documents = Vec::new();

        for doc in documents_raw {
            if let Some(mut content) = doc.content {
                while content.len() > self.config.max_batch_size {
                    let part: String = content.chars().take(self.config.max_batch_size).collect();

                    documents.push(EmbedDocument {
                        content: part,
                        path: doc.path.clone(),
                    });

                    content = content.chars().skip(self.config.max_batch_size).collect();
                }

                documents.push(EmbedDocument {
                    path: doc.path,
                    content: content,
                });
            }
        }

        // Build the embeddings from the documents using the embedding model.
        let builder = EmbeddingsBuilder::new(self.model.clone())
            .documents(documents)
            .map_err(|e| AIError::SetDocumentsFailed(e))?;

        let embeddings = builder
            .build()
            .await
            .map_err(|e| AIError::BuildEmbeddingsFailed(e))?;

        // Save the embeddings to the database, and load the index.
        let index = self.generate_index(embeddings).await?;

        self.index = Some(index);

        return Ok(());
    }

    /// Generate the index from the embeddings (Vec<_>)
    /// Also saves the embeddings to the database.
    ///
    /// Parameters:
    /// - documents: The documents to generate the index from. (created from the embeddings)
    async fn generate_index(
        &mut self,
        documents: Vec<(EmbedDocument, Vec<Embedding>)>,
    ) -> Result<LanceDbVectorIndex<Model>, AIError> {
        let table_names = self
            .db
            .table_names()
            .execute()
            .await
            .map_err(|e| AIError::ListTableNamesError(e))?;

        if table_names.contains(&String::from("files")) {
            self.db
                .drop_table("files", &[])
                .await
                .map_err(|_| AIError::OpenTableError)?;
        }

        let table = self.ensure_table(documents).await?;

        let search_params = SearchParams::default();
        let vector_index = LanceDbVectorIndex::new(table, self.model.clone(), "id", search_params)
            .await
            .map_err(|e| AIError::CreateIndexError(e))?;

        return Ok(vector_index);
    }

    /// Ensure the table exists, and create it if it doesn't.
    /// Also saves the embeddings to the database.
    /// Also (2) creates an index on the table for faster search. (if pertinent)
    ///
    /// Parameters:
    /// - embeddings: The embeddings to save to the database.
    ///
    /// Returns:
    /// - The database table.
    async fn ensure_table(
        &self,
        embeddings: Vec<(EmbedDocument, Vec<Embedding>)>,
    ) -> Result<Table, AIError> {
        let dims = embeddings
            .first()
            .and_then(|(_, vecs)| vecs.first())
            .map(|e| e.vec.len())
            .unwrap_or_else(|| self.model.ndims());

        if dims == 0 {
            return Err(AIError::FaildToGetNbrDimentions);
        }

        // Create the table & save the data
        let record_batch =
            as_record_batch(embeddings, dims).map_err(|_| AIError::FailedToBuildRagDbDatas)?;

        let table = self
            .db
            .create_table("files", vec![record_batch])
            .execute()
            .await
            .map_err(|e| AIError::CreateTableError(e))?;

        // https://github.com/0xPlaygrounds/rig/blob/475cfddc28cc5b367108a3d387b84d5f838d613b/crates/rig-lancedb/examples/vector_search_local_ann_agent.rs#L59
        let row_count = table.count_rows(None).await.unwrap_or(0);

        // Else it's not worth creating an index
        if row_count >= 256 {
            let index_stats = table
                .index_stats("embedding")
                .await
                .map_err(|e| AIError::GetIndexStatsError(e))?;

            if index_stats.is_none() {
                table
                    .create_index(
                        &["embedding"],
                        lancedb::index::Index::IvfPq(IvfPqIndexBuilder::default()),
                    )
                    .execute()
                    .await
                    .map_err(|e| AIError::CreateIndexError(e))?;
            }
        }

        return Ok(table);
    }
}

/// Implements the VectorStoreIndexDyn trait, so we can use it with RIG agents.
impl<Model> VectorStoreIndexDyn for RagStorage<Model>
where
    Model: EmbeddingModel + Clone + Sync + Send + 'static,
{
    fn top_n<'a>(
        &'a self,
        req: VectorSearchRequest<Filter<serde_json::Value>>,
    ) -> WasmBoxedFuture<'a, TopNResults> {
        Box::pin(async move {
            let index = self.index.as_ref().ok_or_else(|| {
                VectorStoreError::DatastoreError(Box::new(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "LanceDB index is not initialized",
                )))
            })?;

            let req = req.try_map_filter(Filter::from_dynamic_filter)?;

            let results = index.top_n(req).await?;

            return Ok(results
                .into_iter()
                .map(|(score, id, doc)| (score, id, LanceDBFilter::normalize_dynamic_document(doc)))
                .collect());
        })
    }

    fn top_n_ids<'a>(
        &'a self,
        req: VectorSearchRequest<Filter<serde_json::Value>>,
    ) -> WasmBoxedFuture<'a, Result<Vec<(f64, String)>, VectorStoreError>> {
        Box::pin(async move {
            let index = self.index.as_ref().ok_or_else(|| {
                VectorStoreError::DatastoreError(Box::new(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "LanceDB index is not initialized",
                )))
            })?;

            let req = req.try_map_filter(Filter::from_dynamic_filter)?;

            return index.top_n_ids(req).await;
        })
    }
}
