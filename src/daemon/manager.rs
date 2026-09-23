use std::sync::Arc;

use tokio::sync::{
    RwLock,
    mpsc::{Sender, error::SendError},
};

use crate::{
    controller::controller::LockedController,
    daemon::worker::{DaemonEvent, DaemonWorker},
};

/// This struct is the main class for stopping / starting a daemon
/// You can add, remove folders to the daemon. You can also stop/start the daemon
pub struct DaemonManager {
    /// The daemon, as Arc so I don't need to use swap or something like that, when using it in the thread
    daemon: Arc<RwLock<DaemonWorker>>,
    /// The event sender (used for adding a folder/removing it)
    sender: Sender<DaemonEvent>,
    /// The killer sender, used to stop the app
    killer: Option<Sender<()>>,
}

impl DaemonManager {
    /// Creates a DaemonManager
    /// It creates a DaemonWorker with an event sender, and creates the manager
    /// Returns:
    /// - Ok(Self): if everything went as planned
    /// - Err(String): an Error, String => the error
    pub fn new(controller: LockedController) -> Result<Self, String> {
        let (worker, sender) = DaemonWorker::new(controller)?;

        return Ok(Self {
            daemon: Arc::new(RwLock::new(worker)),
            sender: sender,
            killer: None,
        });
    }

    /// Starts the Daemon Worker
    /// It's first going to clone the Arc of the DaemonWorker, then it creates a channel for the kill switch
    /// Then it borrows the Daemon Worker write permission, and starts the worker
    pub fn start(&mut self) {
        let daemon_ref = self.daemon.clone();
        let (killer, mut killed) = tokio::sync::mpsc::channel::<()>(100);

        tokio::spawn(async move {
            let mut write = daemon_ref.write().await;
            tokio::select! {
                _ = write.start_worker() => {},
                _ = killed.recv() => {},
            }
        });
        self.killer = Some(killer);
    }

    /// Adds a folder to the watched folders (using the send channel)
    /// Parameters:
    /// - folder: the **ABSOLUE** folder path
    /// - file_indexer: an Arc with the File Indexer element
    ///
    /// Returns:
    /// - Result<(), SendError<DaemonEvent>>: The result if there's any errors
    pub async fn add_folder(&mut self, folder: String) -> Result<(), SendError<DaemonEvent>> {
        return self.sender.send(DaemonEvent::Add(folder)).await;
    }

    /// Unwatches a folder
    /// Parameters:
    /// - folder: the **ABSOLUE** path to the folder that needs to be unwatched
    ///
    /// Returns:
    /// - Result<(), SendError<DaemonEvent>>: The result if there's any errors
    pub async fn remove_folder(&mut self, folder: String) -> Result<(), SendError<DaemonEvent>> {
        return self.sender.send(DaemonEvent::Remove(folder)).await;
    }

    /// Stops the daemon, using the kill switch mpsc channel
    pub async fn stop_daemon(&mut self) -> Result<(), ()> {
        if let Some(killer) = self.killer.as_ref() {
            if killer.send(()).await.is_ok() {
                return Ok(());
            } else {
                return Err(());
            }
        } else {
            return Err(());
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{fs::remove_file, path::MAIN_SEPARATOR, time::Duration};

    use tempdir::TempDir;
    use tokio::{fs, io::AsyncWriteExt, time::sleep};

    use crate::controller::controller::Controller;

    use super::*;

    #[tokio::test]
    async fn test_worker_single_folder() {
        let indexer_temp_dir = TempDir::new("ffsearch_test")
            .expect("Tempdir failed")
            .path()
            .to_string_lossy()
            .to_string();

        let mut controller =
            Controller::new(format!("{}{}config.ini", indexer_temp_dir, MAIN_SEPARATOR))
                .expect("Failed to create controller");

        let indexer_id = controller
            .add_file_indexer(indexer_temp_dir.clone())
            .expect("Failed to create indexer");

        let arc = Arc::new(RwLock::new(controller));

        let mut manager = DaemonManager::new(arc.clone()).expect("Failed to create manager");

        manager.add_folder(indexer_temp_dir.clone()).await;

        manager.start();

        sleep(Duration::from_millis(1000)).await; // TODO: Maybe find a better way to do it?

        let mut file = fs::File::create(format!("{}{}file1.txt", indexer_temp_dir, MAIN_SEPARATOR))
            .await
            .expect("Failed to open file");
        file.write(b"Hello World~").await.expect("Write failed");

        let mut tries = 0;
        loop {
            let file1 = arc
                .read()
                .await
                .get_controller_for(indexer_id)
                .expect("Failed to get controller")
                .get_content(String::from("file1.txt"))
                .await;
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
        let file1 = arc
            .read()
            .await
            .get_controller_for(indexer_id)
            .expect("Failed to get controller")
            .get_content(String::from("file1.txt"))
            .await;

        assert!(file1.is_err());

        manager.remove_folder(indexer_temp_dir.clone()).await;

        let mut file = fs::File::create(format!("{}{}file1.txt", indexer_temp_dir, MAIN_SEPARATOR))
            .await
            .expect("Failed to open file");
        file.write(b"Hello World~").await.expect("Write failed");

        sleep(Duration::from_millis(1000)).await; // TODO: Maybe find a better way to do it?

        let file1 = arc
            .read()
            .await
            .get_controller_for(indexer_id)
            .expect("Failed to get controller")
            .get_content(String::from("file1.txt"))
            .await;
        assert!(file1.is_err());

        sleep(Duration::from_millis(1000)).await; // TODO: Maybe find a better way to do it?

        manager.stop_daemon().await;

        sleep(Duration::from_millis(1000)).await; // TODO: Maybe find a better way to do it?

        let mut file = fs::File::create(format!("{}{}file2.txt", indexer_temp_dir, MAIN_SEPARATOR))
            .await
            .expect("Failed to open file");
        file.write(b"Hello World~").await.expect("Write failed");

        sleep(Duration::from_millis(1000)).await; // TODO: Maybe find a better way to do it?

        let file2 = arc
            .read()
            .await
            .get_controller_for(indexer_id)
            .expect("Failed to get controller")
            .get_content(String::from("file2.txt"))
            .await;
        assert!(file2.is_err());
    }
}
