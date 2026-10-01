#![cfg(feature = "daemon_worker")]
use std::{sync::Arc, time::Duration};

use application::{
    indexer::{actions::sync::FileIndexerSync, traits::registry::FileIndexerRegistry},
    searcher::{
        actions::{get::DatabaseGet, modification::UpdateDatabase},
        traits::database::DatabaseProvider,
    },
};
use daemon::worker::{DaemonEvent, DaemonWorker};
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

    let path_str = path.to_string_lossy().to_string();
    let th = tokio::spawn(async move {
        let (mut worker, event_sender) =
            DaemonWorker::new(get, update, sync).expect("Failed to create Daemon");

        event_sender
            .send(DaemonEvent::Add(path_str))
            .await
            .expect("Failed to send add folder");

        worker.start_worker().await;
    });

    sleep(Duration::from_millis(1000)).await; // TODO: Maybe find a better way to do it?

    // ---------------------------------------
    // Tests adding a file
    // ---------------------------------------

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

    th.abort();
}
