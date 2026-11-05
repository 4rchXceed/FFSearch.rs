extern crate application;
extern crate domain;
extern crate infrastructure;
extern crate tempdir;

use std::sync::Arc;

use application::{
    indexer::traits::registry::FileIndexerRegistry,
    searcher::actions::{get::DatabaseGet, modification::UpdateDatabase, search::SearchDocument},
};
use domain::{
    indexer::config::FileIndexerConfig,
    searcher::{config::DatabaseConfig, document::Document, search_query::SearchContent},
};
use infrastructure::{
    indexer::stdfile::{indexer::StdFileIndexerProvider, registry::InMemoryRegistry},
    searcher::tantivy::database::TantivyDatabase,
};
use tempdir::TempDir;

#[tokio::test]
async fn test_database_add_remove_update_get_all_with_content() {
    // ---------------------------------------
    // Tear up
    // ---------------------------------------

    let tempdir = TempDir::new("ffsearch_test").expect("Failed to create tempdir");
    let path = tempdir.path().to_string_lossy().to_string();

    let db =
        TantivyDatabase::new(path.clone(), DatabaseConfig::new()).expect("Failed to create db");
    let db_provider = Arc::new(db);

    let registry = InMemoryRegistry::new();
    let id = registry
        .add(
            Arc::new(StdFileIndexerProvider::new(FileIndexerConfig::new())),
            db_provider,
            path,
        )
        .await;
    let registry = Arc::new(registry);

    let modification = UpdateDatabase::new(registry.clone());
    let get = DatabaseGet::new(registry);

    let document = Document::new(
        String::from("file1.txt"),
        Some(String::from("Hatsune Miku~")),
        0,
    );

    // ---------------------------------------
    // Test: add()
    // ---------------------------------------

    modification
        .add(id, document.clone())
        .await
        .expect("Failed to add new doc");

    // ---------------------------------------
    // Test: apply_changes()
    // ---------------------------------------
    modification
        .apply_changes(id)
        .await
        .expect("Failed to apply changes");

    // ---------------------------------------
    // Test: get_one()
    // ---------------------------------------

    let result = get
        .get_one(1, String::from("file1.txt"))
        .await
        .expect("Failed to get file");

    assert_eq!(result.content, Some(String::from("Hatsune Miku~")));

    // ---------------------------------------
    // Test: update()
    // ---------------------------------------

    let mut new_document = document.clone();

    new_document.content = Some(String::from("Hatsune Miku ^.^"));
    new_document.modification_date = 1;

    modification
        .update(id, String::from("file1.txt"), new_document.clone())
        .await
        .expect("Failed to update document");

    modification
        .apply_changes(id)
        .await
        .expect("Failed to apply changes");

    // ---------------------------------------
    // Test: get_all_with_content()
    // ---------------------------------------

    let result = get
        .get_all_with_content(id)
        .await
        .expect("Failed to get file");

    assert_eq!(result, vec![new_document]);
}

#[tokio::test]
async fn test_database_add_remove_search() {
    // ---------------------------------------
    // Tear up
    // ---------------------------------------
    let tempdir = TempDir::new("ffsearch_test").expect("Failed to create tempdir");
    let path = tempdir.path().to_string_lossy().to_string();

    let db =
        TantivyDatabase::new(path.clone(), DatabaseConfig::new()).expect("Failed to create db");
    let db_provider = Arc::new(db);

    let registry = InMemoryRegistry::new();
    let id = registry
        .add(
            Arc::new(StdFileIndexerProvider::new(FileIndexerConfig::new())),
            db_provider,
            path,
        )
        .await;
    let registry = Arc::new(registry);

    let modification = UpdateDatabase::new(registry.clone());
    let search = SearchDocument::new(registry);

    let document = Document::new(
        String::from("file1.txt"),
        Some(String::from("Hatsune Miku~ hehe")),
        0,
    );
    let document_2 = Document::new(
        String::from("file2.txt"),
        Some(String::from("Kasane Teto~ hehe")),
        0,
    );

    // ---------------------------------------
    // Test: add()
    // ---------------------------------------

    modification
        .add(id, document)
        .await
        .expect("Failed to add new doc");

    modification
        .add(id, document_2)
        .await
        .expect("Failed to add 2nd doc");

    // ---------------------------------------
    // Test: apply_changes()
    // ---------------------------------------
    modification
        .apply_changes(id)
        .await
        .expect("Failed to apply changes");

    // ---------------------------------------
    // Test: search()
    // ---------------------------------------
    let result = search
        .search(
            id,
            String::from("Hatsune"),
            2,
            SearchContent::ContentAndPath,
        )
        .await
        .expect("Failed to search");

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].path, "file1.txt");

    let result = search
        .search(id, String::from("HEHE"), 2, SearchContent::ContentAndPath)
        .await
        .expect("Failed to search");

    assert_eq!(result.len(), 2);

    // ---------------------------------------
    // Test: remove()
    // ---------------------------------------
    modification
        .remove(id, String::from("file1.txt"))
        .await
        .expect("Failed to remove");

    modification
        .apply_changes(id)
        .await
        .expect("Failed to apply changes");

    let result = search
        .search(
            id,
            String::from("Hatsune"),
            2,
            SearchContent::ContentAndPath,
        )
        .await
        .expect("Failed to search");

    assert_eq!(result.len(), 0);
}

#[tokio::test]
async fn test_database_add_get_one_apply_get_all_without_content() {
    // ---------------------------------------
    // Tear up
    // ---------------------------------------
    let tempdir = TempDir::new("ffsearch_test").expect("Failed to create tempdir");
    let path = tempdir.path().to_string_lossy().to_string();

    let db =
        TantivyDatabase::new(path.clone(), DatabaseConfig::new()).expect("Failed to create db");
    let db_provider = Arc::new(db);
    let registry = InMemoryRegistry::new();
    let id = registry
        .add(
            Arc::new(StdFileIndexerProvider::new(FileIndexerConfig::new())),
            db_provider,
            path,
        )
        .await;
    let registry = Arc::new(registry);

    let modification = UpdateDatabase::new(registry.clone());
    let get = DatabaseGet::new(registry);

    let document = Document::new(
        String::from("file1.txt"),
        Some(String::from("Hatsune Miku~")),
        0,
    );

    // ---------------------------------------
    // Test: add()
    // ---------------------------------------

    modification
        .add(id, document)
        .await
        .expect("Failed to add new doc");

    // ---------------------------------------
    // Test: apply_changes()
    // ---------------------------------------
    modification
        .apply_changes(id)
        .await
        .expect("Failed to apply changes");

    // ---------------------------------------
    // Test: get_one()
    // ---------------------------------------

    let result = get
        .get_one(id, String::from("file1.txt"))
        .await
        .expect("Failed to get file");

    assert_eq!(result.content, Some(String::from("Hatsune Miku~")));

    // ---------------------------------------
    // Test: get_all_without_content()
    // ---------------------------------------

    let result = get
        .get_all_without_content(id)
        .await
        .expect("Failed to get file");

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].path, "file1.txt");
}
