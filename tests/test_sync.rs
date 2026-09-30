use std::{
    ffi::OsString,
    fs::{File, create_dir},
    io::Write,
    path::Path,
    str::FromStr,
    sync::Arc,
    time::Duration,
};

use application::{
    indexer::actions::sync::FileIndexerSync,
    searcher::{error::DatabaseError, traits::database::DatabaseProvider},
};
use domain::{
    indexer::config::FileIndexerConfig,
    searcher::{config::DatabaseConfig, error::DatabaseDomainError},
};
use infrastructure::{
    indexer::{stdfile::indexer::StdFileIndexerProvider, xberg::extractor::XbergFileExtractor},
    searcher::tantivy::database::TantivyDatabase,
};
use tempdir::TempDir;
use tokio::{fs::remove_file, time::sleep};

const FIRST_FILE_CONTENT: &str = "This is some t-text, yeah I guess I wiww wwite some mowe text";
const SECOND_FILE_CONTENT: &str = "Love: Rust tackwes scale and c-c-concuwwency. Love: Rust is a modewn wanguage. Love: You can teww the *boops your nose* Rust compiler what to do. Love: Rust has gweat backwawd compatibiwity. Love: Pwogwamming without abstwactions. Love: A bettew w-way to do gawbage cowwection";
const THIRD_FILE_CONTENT: &str = "Wowaka rest in peace <3";

fn write_to_file(path: OsString, content: &str) {
    let file = File::create(path);
    assert!(file.is_ok());

    let res = file.unwrap().write_all(content.as_bytes());

    assert!(res.is_ok());
}

fn generate_test_datas(path: &Path) {
    let file1_path = OsString::from_str(&path.join("file1.txt").to_string_lossy().to_string());
    let dir1_path = OsString::from_str(&path.join("dir1").to_string_lossy().to_string());
    let file2_path = OsString::from_str(&path.join("file2.txt").to_string_lossy().to_string());

    assert!(file1_path.is_ok());
    assert!(dir1_path.is_ok());
    assert!(file2_path.is_ok());

    write_to_file(file1_path.unwrap(), FIRST_FILE_CONTENT);

    let res = create_dir(dir1_path.unwrap());
    assert!(res.is_ok());

    write_to_file(file2_path.unwrap(), SECOND_FILE_CONTENT);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_sync_once() {
    // ---------------------------------------
    // Tear up
    // ---------------------------------------

    let tempdir = TempDir::new("ffsearch_test").expect("Failed to create tempdir");
    let path = tempdir.path();

    generate_test_datas(path);

    let db = TantivyDatabase::new(path.to_string_lossy().to_string(), DatabaseConfig::new())
        .expect("Failed to create db");
    let db = Arc::new(db);

    let indexer = StdFileIndexerProvider::new(FileIndexerConfig::new());
    let indexer = Arc::new(indexer);

    let extractor = XbergFileExtractor::new(&FileIndexerConfig::new());
    let extractor = Arc::new(extractor);

    let syncer = FileIndexerSync::new(
        indexer,
        db.clone(),
        extractor,
        path.to_string_lossy().to_string(),
    );

    // ---------------------------------------
    // Tests FileIndexerSync sync()
    // ---------------------------------------

    let errors = syncer.sync().await.expect("Failed to sync");
    assert!(errors.is_empty());

    // ---------------------------------------
    // Tests get()
    // ---------------------------------------

    let file1 = db.get(String::from("file1.txt"));
    let file2 = db.get(String::from("file2.txt"));
    let file3 = db.get(String::from("dir1/file2.txt"));

    assert!(file1.is_ok());

    let first_content = String::from(FIRST_FILE_CONTENT);
    assert_eq!(file1.unwrap().content.unwrap(), first_content);

    let second_content = String::from(SECOND_FILE_CONTENT);
    assert_eq!(file2.unwrap().content.unwrap(), second_content);

    assert!(match file3.err().unwrap() {
        DatabaseError::DomainError(DatabaseDomainError::NoResult) => true,
        _ => false,
    });
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_sync_twice() {
    // ---------------------------------------
    // Tear up
    // ---------------------------------------

    let tempdir = TempDir::new("ffsearch_test").expect("Failed to create tempdir");
    let path = tempdir.path();

    generate_test_datas(path);

    let db = TantivyDatabase::new(path.to_string_lossy().to_string(), DatabaseConfig::new())
        .expect("Failed to create db");
    let db = Arc::new(db);

    let indexer = StdFileIndexerProvider::new(FileIndexerConfig::new());
    let indexer = Arc::new(indexer);

    let extractor = XbergFileExtractor::new(&FileIndexerConfig::new());
    let extractor = Arc::new(extractor);

    let syncer = FileIndexerSync::new(
        indexer,
        db.clone(),
        extractor,
        path.to_string_lossy().to_string(),
    );

    // ---------------------------------------
    // Tests FileIndexerSync sync()
    // ---------------------------------------

    let errors = syncer.sync().await.expect("Failed to sync");
    assert!(errors.is_empty());

    sleep(Duration::from_secs(2)).await;

    let file2_path = OsString::from_str(&path.join("file2.txt").to_string_lossy().to_string());
    assert!(file2_path.is_ok());

    // ---------------------------------------
    // Tests FileIndexerSync sync() (with a new file)
    // ---------------------------------------

    write_to_file(file2_path.unwrap(), THIRD_FILE_CONTENT);

    let errors = syncer.sync().await.expect("Failed to sync");
    assert!(errors.is_empty());

    let file1 = db.get(String::from("file1.txt"));
    assert!(file1.is_ok());

    let first_content = String::from(FIRST_FILE_CONTENT);
    assert_eq!(file1.unwrap().content.unwrap(), first_content);

    let file3 = db.get(String::from("file2.txt"));
    assert!(file3.is_ok());

    let third_content = String::from(THIRD_FILE_CONTENT);
    assert_eq!(file3.unwrap().content.unwrap(), third_content);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_sync_remove() {
    // ---------------------------------------
    // Tear up
    // ---------------------------------------

    let tempdir = TempDir::new("ffsearch_test").expect("Failed to create tempdir");
    let path = tempdir.path();

    generate_test_datas(path);

    let db = TantivyDatabase::new(path.to_string_lossy().to_string(), DatabaseConfig::new())
        .expect("Failed to create db");
    let db = Arc::new(db);

    let indexer = StdFileIndexerProvider::new(FileIndexerConfig::new());
    let indexer = Arc::new(indexer);

    let extractor = XbergFileExtractor::new(&FileIndexerConfig::new());
    let extractor = Arc::new(extractor);

    let syncer = FileIndexerSync::new(
        indexer,
        db.clone(),
        extractor,
        path.to_string_lossy().to_string(),
    );

    // ---------------------------------------
    // Tests FileIndexerSync sync()
    // ---------------------------------------

    let errors = syncer.sync().await.expect("Failed to sync");
    assert!(errors.is_empty());

    // ---------------------------------------
    // Tests FileIndexerSync sync()
    // ---------------------------------------

    remove_file(path.join("file1.txt"))
        .await
        .expect("Failed to remove file");

    let errors = syncer.sync().await.expect("Failed to sync");
    assert!(errors.is_empty());

    let file1 = db.get(String::from("file1.txt"));
    assert!(match file1.err().unwrap() {
        DatabaseError::DomainError(DatabaseDomainError::NoResult) => true,
        _ => false,
    });
}
