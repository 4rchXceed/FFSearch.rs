use std::sync::Arc;

use tokio::sync::{RwLock, mpsc::Sender};

use crate::{
    daemon::worker::{DaemonEvent, DaemonWorker},
    file_indexer::indexer::FileIndexer,
};

pub struct DaemonManager {
    daemon: Arc<RwLock<DaemonWorker>>,
    sender: Sender<DaemonEvent>,
}

impl DaemonManager {
    pub fn new() -> Result<Self, String> {
        let (worker, sender) = DaemonWorker::new()?;

        return Ok(Self {
            daemon: Arc::new(RwLock::new(worker)),
            sender: sender,
        });
    }

    pub fn start(&mut self) {
        let daemon_ref = self.daemon.clone();

        tokio::spawn(async move {
            daemon_ref.write().await.start_worker().await;
        });
    }

    pub async fn add_folder(
        &mut self,
        folder: String,
        file_indexer: Arc<RwLock<FileIndexer>>,
    ) -> bool {
        return self
            .sender
            .send(DaemonEvent::Add(folder, file_indexer))
            .await
            .is_ok();
    }

    pub async fn remove_folder(&mut self, folder: String) -> bool {
        return self.sender.send(DaemonEvent::Remove(folder)).await.is_ok();
    }
}

#[cfg(test)]
mod tests {
    use std::{fs::remove_file, path::MAIN_SEPARATOR, time::Duration};

    use tempdir::TempDir;
    use tokio::{fs, io::AsyncWriteExt, time::sleep};

    use crate::file_indexer::config::FileIndexerConfig;

    use super::*;

    #[tokio::test]
    async fn test_worker_single_folder() {
        let indexer_temp_dir = TempDir::new("ffsearch_test")
            .expect("Tempdir failed")
            .path()
            .to_string_lossy()
            .to_string();
        let indexer = FileIndexer::new(indexer_temp_dir.clone(), FileIndexerConfig::new())
            .expect("Indexer failed");

        let arc = Arc::new(RwLock::new(indexer));
        let mut manager = DaemonManager::new().expect("Failed to create manager");

        manager
            .add_folder(indexer_temp_dir.clone(), arc.clone())
            .await;

        manager.start();

        sleep(Duration::from_millis(1000)).await; // TODO: Maybe find a better way to do it?

        let mut file = fs::File::create(format!("{}{}file1.txt", indexer_temp_dir, MAIN_SEPARATOR))
            .await
            .expect("Failed to open file");
        file.write(b"Hello World~").await.expect("Write failed");

        let mut tries = 0;
        loop {
            let file1 = arc.read().await.get_db_ro().get(String::from("file1.txt"));
            if let Ok(file1) = file1
                && file1.content == Some(String::from("Hello World~"))
            {
                break;
            } else if tries < 10 {
                tries += 1;
            } else {
                panic!("Test failed > 5s!!!");
            }
            sleep(Duration::from_millis(500)).await; // TODO: Maybe find a better way to do it?
        }

        remove_file(format!("{}{}file1.txt", indexer_temp_dir, MAIN_SEPARATOR))
            .expect("Failed to remove file");

        sleep(Duration::from_millis(1000)).await; // TODO: Maybe find a better way to do it?

        println!("{}", indexer_temp_dir);
        let file1 = arc.read().await.get_db_ro().get(String::from("file1.txt"));
        assert!(file1.is_err());

        manager.remove_folder(indexer_temp_dir.clone()).await;

        let mut file = fs::File::create(format!("{}{}file1.txt", indexer_temp_dir, MAIN_SEPARATOR))
            .await
            .expect("Failed to open file");
        file.write(b"Hello World~").await.expect("Write failed");

        sleep(Duration::from_millis(1000)).await; // TODO: Maybe find a better way to do it?

        let file1 = arc.read().await.get_db_ro().get(String::from("file1.txt"));
        assert!(file1.is_err());
    }
}
