use std::sync::Arc;

use application::{
    indexer::actions::sync::FileIndexerSync,
    searcher::actions::{get::DatabaseGet, modification::UpdateDatabase},
};
use tokio::sync::{
    RwLock,
    mpsc::{Sender, error::SendError},
};

use crate::worker::{DaemonEvent, DaemonWorker};

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
    pub fn new(
        get: Arc<DatabaseGet>,
        update: Arc<UpdateDatabase>,
        sync: Arc<FileIndexerSync>,
    ) -> Result<Self, String> {
        let (worker, sender) = DaemonWorker::new(get, update, sync)?;

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
