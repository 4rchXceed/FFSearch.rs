#![cfg(feature = "daemon_worker")]
use std::{fs::remove_file, sync::Arc, time::Duration};

use application::{
    indexer::{actions::sync::FileIndexerSync, traits::registry::FileIndexerRegistry},
    searcher::{
        actions::{get::DatabaseGet, modification::UpdateDatabase},
        traits::database::DatabaseProvider,
    },
};
use daemon::manager::DaemonManager;
use domain::{indexer::config::FileIndexerConfig, searcher::config::DatabaseConfig};
use infrastructure::{
    indexer::{
        stdfile::{indexer::StdFileIndexerProvider, registry::InMemoryRegistry},
        xberg::extractor::XbergFileExtractor,
    },
    searcher::tantivy::database::TantivyDatabase,
};
use tempdir::TempDir;
use tokio::{fs, io::AsyncWriteExt, time::sleep};

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_worker_single_folder() {
    // ---------------------------------------
    // Tear up
    // ---------------------------------------

    simple_logger::SimpleLogger::new().env().init().unwrap();

    let tempdir = TempDir::new("ffsearch_test").expect("Failed to create tempdir");
    let path = tempdir.path();

    let db = TantivyDatabase::new(path.to_string_lossy().to_string(), DatabaseConfig::new())
        .expect("Failed to create db");
    let db = Arc::new(db);

    let indexer = StdFileIndexerProvider::new(FileIndexerConfig::new());
    let indexer = Arc::new(indexer);

    let extractor = XbergFileExtractor::new(&FileIndexerConfig::new());
    let extractor = Arc::new(extractor);

    let registry = InMemoryRegistry::new();
    let registry = Arc::new(registry);

    registry
        .add(indexer, db.clone(), path.to_string_lossy().to_string())
        .await;

    let sync = Arc::new(FileIndexerSync::new(registry.clone(), extractor));
    let get = Arc::new(DatabaseGet::new(registry.clone()));
    let update = Arc::new(UpdateDatabase::new(registry.clone()));

    // ---------------------------------------
    // Tests add_folder and start
    // ---------------------------------------

    let mut manager = DaemonManager::new(get, update, sync).expect("Failed to create manager");

    manager
        .add_folder(path.to_string_lossy().to_string())
        .await
        .expect("Failed to add the folder");

    manager.start();

    // ---------------------------------------
    // Tests adding a file
    // ---------------------------------------

    sleep(Duration::from_millis(1000)).await; // TODO: Maybe find a better way to do it?

    let mut file = fs::File::create(path.join("file1.txt"))
        .await
        .expect("Failed to open file");
    file.write(b"Hello World~").await.expect("Write failed");

    let mut tries = 0;

    loop {
        let file1 = db.get(String::from("file1.txt"));

        if let Ok(file1) = file1
            && file1.content == Some(String::from("Hello World~"))
        {
            break;
        } else if tries < 10 {
            tries += 1;
        } else {
            panic!("Test failed >5s waiting !!!");
        }

        sleep(Duration::from_millis(500)).await; // TODO: Maybe find a better way to do it?
    }

    // ---------------------------------------
    // Tests removing a file
    // ---------------------------------------

    sleep(Duration::from_millis(1000)).await; // TODO: Maybe find a better way to do it?

    remove_file(path.join("file1.txt")).expect("Failed to remove file");

    sleep(Duration::from_millis(2000)).await; // TODO: Maybe find a better way to do it?

    let file1 = db.get(String::from("file1.txt"));

    assert!(file1.is_err());

    // ---------------------------------------
    // Tests remove_folder()
    // ---------------------------------------

    manager
        .remove_folder(path.to_string_lossy().to_string())
        .await
        .expect("Failed to remove the folder");

    sleep(Duration::from_millis(1000)).await; // TODO: Maybe find a better way to do it?

    let mut file = fs::File::create(path.join("file1.txt"))
        .await
        .expect("Failed to open file");

    file.write(b"Hello World~").await.expect("Write failed");

    sleep(Duration::from_millis(1000)).await; // TODO: Maybe find a better way to do it?

    let file1 = db.get(String::from("file1.txt"));

    assert!(file1.is_err());

    sleep(Duration::from_millis(1000)).await; // TODO: Maybe find a better way to do it?

    // ---------------------------------------
    // Tests stop_daemon()
    // ---------------------------------------

    manager
        .stop_daemon()
        .await
        .expect("Failed to stop the daemon");

    sleep(Duration::from_millis(1000)).await; // TODO: Maybe find a better way to do it?

    let mut file = fs::File::create(path.join("file2.txt"))
        .await
        .expect("Failed to open file");
    file.write(b"Hello World~").await.expect("Write failed");

    sleep(Duration::from_millis(1000)).await; // TODO: Maybe find a better way to do it?

    let file2 = db.get(String::from("file2.txt"));

    assert!(file2.is_err());
}
