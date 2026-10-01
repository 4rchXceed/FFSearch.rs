use std::sync::Arc;

use application::{
    indexer::{actions::sync::FileIndexerSync, traits::registry::FileIndexerRegistry},
    searcher::actions::{get::DatabaseGet, search::SearchDocument},
};
use domain::{indexer::config::FileIndexerConfig, searcher::config::DatabaseConfig};
use infrastructure::{
    indexer::{
        stdfile::{indexer::StdFileIndexerProvider, registry::InMemoryRegistry},
        xberg::extractor::XbergFileExtractor,
    },
    searcher::tantivy::database::TantivyDatabase,
};
use mcp::{config::McpServerConfig, server::McpServer};

const FOLDER: &str = "E:\\IN_3\\1_4\\M107\\S_Scripts";

pub async fn test_mcp() {
    let indexer = Arc::new(StdFileIndexerProvider::new(FileIndexerConfig::new()));

    let db = Arc::new(
        TantivyDatabase::new(FOLDER.to_string(), DatabaseConfig::new())
            .expect("Failed to create db"),
    );

    let extractor = Arc::new(XbergFileExtractor::new(&FileIndexerConfig::new()));

    let registry = Arc::new(InMemoryRegistry::new());

    let id = registry.add(indexer, db, FOLDER.to_string()).await;

    let sync = FileIndexerSync::new(registry.clone(), extractor.clone());

    sync.sync(id).await.expect("Failed");

    let search = SearchDocument::new(registry.clone());

    let get = DatabaseGet::new(registry.clone());

    McpServer::run_server(Arc::new(get), Arc::new(search), McpServerConfig::new())
        .await
        .expect("Failed to run server");
}
