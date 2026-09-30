extern crate application;
extern crate domain;
extern crate infrastructure;
extern crate tempdir;

use std::sync::Arc;

use application::searcher::actions::{
    get::DatabaseGet, modification::UpdateDatabase, search::SearchDocument,
};
use domain::searcher::{config::DatabaseConfig, document::Document};
use infrastructure::searcher::tantivy::database::TantivyDatabase;
use tempdir::TempDir;

#[tokio::test]
async fn test_database_add_remove_update_get_all_with_content() {
    // ---------------------------------------
    // Tear up
    // ---------------------------------------
    let tempdir = TempDir::new("ffsearch_test").expect("Failed to create tempdir");
    let path = tempdir.path().to_string_lossy().to_string();

    let db = TantivyDatabase::new(path, DatabaseConfig::new()).expect("Failed to create db");
    let db_provider = Arc::new(db);

    let modification = UpdateDatabase::new(db_provider.clone());
    let get = DatabaseGet::new(db_provider.clone());

    let document = Document::new(
        String::from("file1.txt"),
        Some(String::from("Hatsune Miku~")),
        0,
    );

    // ---------------------------------------
    // Test: add()
    // ---------------------------------------

    modification
        .add(document.clone())
        .await
        .expect("Failed to add new doc");

    // ---------------------------------------
    // Test: apply_changes()
    // ---------------------------------------
    modification
        .apply_changes()
        .await
        .expect("Failed to apply changes");

    // ---------------------------------------
    // Test: get_one()
    // ---------------------------------------

    let result = get
        .get_one(String::from("file1.txt"))
        .expect("Failed to get file");

    assert_eq!(result.content, Some(String::from("Hatsune Miku~")));

    // ---------------------------------------
    // Test: update()
    // ---------------------------------------

    let mut new_document = document.clone();

    new_document.content = Some(String::from("Hatsune Miku ^.^"));
    new_document.modification_date = 1;

    modification
        .update(String::from("file1.txt"), new_document.clone())
        .await
        .expect("Failed to update document");

    modification
        .apply_changes()
        .await
        .expect("Failed to apply changes");

    // ---------------------------------------
    // Test: get_all_with_content()
    // ---------------------------------------

    let result = get.get_all_with_content().expect("Failed to get file");

    assert_eq!(result, vec![new_document]);
}

#[tokio::test]
async fn test_database_add_remove_search() {
    // ---------------------------------------
    // Tear up
    // ---------------------------------------
    let tempdir = TempDir::new("ffsearch_test").expect("Failed to create tempdir");
    let path = tempdir.path().to_string_lossy().to_string();

    let db = TantivyDatabase::new(path, DatabaseConfig::new()).expect("Failed to create db");
    let db_provider = Arc::new(db);

    let modification = UpdateDatabase::new(db_provider.clone());
    let search = SearchDocument::new(db_provider.clone());

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
        .add(document)
        .await
        .expect("Failed to add new doc");

    modification
        .add(document_2)
        .await
        .expect("Failed to add 2nd doc");

    // ---------------------------------------
    // Test: apply_changes()
    // ---------------------------------------
    modification
        .apply_changes()
        .await
        .expect("Failed to apply changes");

    // ---------------------------------------
    // Test: search()
    // ---------------------------------------
    let result = search
        .search(String::from("Hatsune"), 2)
        .expect("Failed to search");

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].path, "file1.txt");

    let result = search
        .search(String::from("HEHE"), 2)
        .expect("Failed to search");

    assert_eq!(result.len(), 2);

    // ---------------------------------------
    // Test: remove()
    // ---------------------------------------
    modification
        .remove(String::from("file1.txt"))
        .await
        .expect("Failed to remove");

    modification
        .apply_changes()
        .await
        .expect("Failed to apply changes");

    let result = search
        .search(String::from("Hatsune"), 2)
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

    let db = TantivyDatabase::new(path, DatabaseConfig::new()).expect("Failed to create db");
    let db_provider = Arc::new(db);

    let modification = UpdateDatabase::new(db_provider.clone());
    let get = DatabaseGet::new(db_provider.clone());

    let document = Document::new(
        String::from("file1.txt"),
        Some(String::from("Hatsune Miku~")),
        0,
    );

    // ---------------------------------------
    // Test: add()
    // ---------------------------------------

    modification
        .add(document)
        .await
        .expect("Failed to add new doc");

    // ---------------------------------------
    // Test: apply_changes()
    // ---------------------------------------
    modification
        .apply_changes()
        .await
        .expect("Failed to apply changes");

    // ---------------------------------------
    // Test: get_one()
    // ---------------------------------------

    let result = get
        .get_one(String::from("file1.txt"))
        .expect("Failed to get file");

    assert_eq!(result.content, Some(String::from("Hatsune Miku~")));

    // ---------------------------------------
    // Test: get_all_without_content()
    // ---------------------------------------

    let result = get.get_all_without_content().expect("Failed to get file");

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].path, "file1.txt");
}
