use application::indexer::actions::sync::FileIndexerSync;
use application::indexer::traits::registry::IndexerId;
use application::searcher::actions::get::DatabaseGet;
use application::searcher::actions::modification::UpdateDatabase;
use application::searcher::error::DatabaseError;
use domain::indexer::file_operation::FileOperationNeeded;
use domain::searcher::error::DatabaseDomainError;
use log::error;
use log::trace;
use std::path::MAIN_SEPARATOR_STR;
use std::path::Path;
use std::sync::Arc;
use utils::trim_custom;

use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
use tokio::sync::mpsc::Receiver;
use tokio::sync::mpsc::Sender;

/// This represents an event that the DaemonManager sends to the DaemonWorker
pub enum DaemonEvent {
    /// Adds a folder, from it's path and a ref to the FileIndexer
    Add(String),
    /// Removes a folder, from it's path
    Remove(String),
}

/// The daemon worker:
///
/// This struct represents a worker (background job) that will listen to file events in multiple folders, and write the changes into the FileIndexer if needed.
pub struct DaemonWorker {
    /// The db's get use case
    db_get: Arc<DatabaseGet>,
    /// The db's update use case
    db_update: Arc<UpdateDatabase>,
    /// The sync's use case
    db_sync: Arc<FileIndexerSync>,
    /// The FS Event receiver
    rx: Receiver<notify::Result<notify::Event>>,
    /// The Daemon Event receiver (see Daemon Event's description)
    rx_event: Receiver<DaemonEvent>,
    /// The file watcher (that will trigger the FS Events)
    watcher: RecommendedWatcher,
}

impl DaemonWorker {
    /// Creates a daemon worker.
    ///
    /// First this will create an mpsc channel for the notify (FS Events) results, and another one for the DaemonEvents
    ///
    /// Then this'll create a tokio-friendly FS Event watcher
    ///
    /// Returns:
    /// - Ok((Self {...}, Sender<DaemonEvent>)): A DaemonWorker instance and a DaemonEvent mpsc event
    /// - Err(String): The error
    pub fn new(
        db_get: Arc<DatabaseGet>,
        db_update: Arc<UpdateDatabase>,
        db_sync: Arc<FileIndexerSync>,
    ) -> Result<(Self, Sender<DaemonEvent>), String> {
        let (tx, rx) = tokio::sync::mpsc::channel::<notify::Result<notify::Event>>(100);
        let (tx_event, rx_event) = tokio::sync::mpsc::channel::<DaemonEvent>(100);

        let watcher = notify::RecommendedWatcher::new(
            move |res| {
                let _ = tx.blocking_send(res);
            },
            Config::default(),
        )
        .map_err(|e| e.to_string())?;

        return Ok((
            Self {
                db_get,
                db_sync,
                db_update,
                rx,
                rx_event,
                watcher,
            },
            tx_event,
        ));
    }

    /// Gets the file indexer and the "clean" relative path from an absolute path
    /// Clean relative path is relative to the file indexer's path
    ///
    /// Parameters:
    /// - path: the absolute path of the file
    ///
    /// Returns:
    /// - String: the clean relative path
    /// - &Arc<RwLock<FileIndexer>>: the Arc reference to the FileIndexer
    async fn get_from_path(&self, path: &String) -> Option<(IndexerId, String)> {
        return self
            .db_get
            .list_folders()
            .await
            .iter()
            .find(|(_, f)| path.starts_with(f.as_str()))
            .map(|(i, f)| (i, path.strip_prefix(f).unwrap_or(path.as_str()).to_string()))
            .map(|(i, f)| (*i, trim_custom(f, MAIN_SEPARATOR_STR))); // Avoid having something like \file1.txt or /file1.txt
    }

    /// Adds a folder to the watched folders
    /// Inserts it into the folders collection
    /// Also adds it to the watcher
    ///
    /// Parameters:
    /// - folder: the folder path: MUST be absolute
    /// - indexer: Arc ref to he file indexer
    ///
    /// Returns:
    /// - Ok(()): ok
    /// - Err(String): error  with the string of
    pub fn add_folder(&mut self, folder: String) -> Result<(), String> {
        if !Path::new(&folder).is_absolute() {
            return Err(String::from("Path MUST be absolute"));
        }

        self.watcher
            .watch(&Path::new(&folder), RecursiveMode::Recursive)
            .map_err(|e| e.to_string())?;

        return Ok(());
    }

    /// Removes a folder from the watcher
    ///
    /// Parameters:
    /// - folder: the folder's path
    ///
    /// Returns:
    /// - Ok(()):
    pub fn remove_folder(&mut self, folder: String) -> Result<(), String> {
        self.watcher
            .unwatch(&Path::new(&folder))
            .map_err(|e| e.to_string())?;

        return Ok(());
    }

    /// Starts a worker with a threaded-function
    ///
    /// Waits for either receiving an FileSystem even, or a DaemonEven
    pub async fn start_worker(&mut self) {
        loop {
            tokio::select! {
                event = self.rx.recv() => {
                    self.handle_fs_event(event).await;
                }
                event = self.rx_event.recv() => {
                    self.handle_manager_event(event);
                }
            }
        }
    }

    /// Handles a DaemonEvent, and redirects it correctly
    ///
    /// Parameters:
    /// - event: the DaemonEvent
    fn handle_manager_event(&mut self, event: Option<DaemonEvent>) {
        let error = match event {
            Some(event) => match event {
                DaemonEvent::Add(folder) => self.add_folder(folder),
                DaemonEvent::Remove(folder) => self.remove_folder(folder),
            },
            None => Ok(()),
        };

        if let Err(err) = error {
            error!("Error while handling event: {err}");
        }
    }

    /// This will handle an event by doing the right actions
    ///
    /// Parameters:
    /// - res: the FS Notify event
    async fn handle_fs_event(&self, res: Option<notify::Result<notify::Event>>) {
        match res {
            Some(event_op) => match event_op {
                Ok(event) => {
                    for path in &event.paths {
                        let path = path.to_string_lossy().to_string();
                        let folder = self.get_from_path(&path).await;

                        if let Some((indexer_uid, folder)) = folder {
                            self.process_event(&event, &path, folder, indexer_uid).await;
                        } else {
                            error!("File {path} watched, but not in the watch folders");
                        }
                    }
                }
                Err(e) => {
                    let err = e.to_string();
                    error!("Watcher error: {err}");
                }
            },
            None => {}
        }
    }

    /// This processes an FS event by adding it to the FileIndexer
    ///
    /// Parameters:
    /// - event: the FS event
    /// - path: the absolute path to the file
    /// - db_path: the path of the file RELATIVE to the FileIndexer
    /// - indexer: the file indexer
    async fn process_event(
        &self,
        event: &notify::Event,
        path: &String,
        db_path: String,
        indexer_uid: IndexerId,
    ) {
        if let Some(db_path_prefix) = self.db_get.get_db_folder(indexer_uid).await {
            // Check if it's not the system folder
            if path.starts_with(&db_path_prefix) {
                return;
            }

            trace!("Received event for file: {path}...");

            if !event.kind.is_modify() && !event.kind.is_create() {
                if event.kind.is_remove() {
                    trace!("Remove event for file: {path}. Removing from database...");
                    let operation_result = self.db_update.remove(indexer_uid, db_path).await;

                    match operation_result {
                        Ok(_) => trace!("File removed!"),
                        Err(e) => error!("[DAEMON]: Error while removing file: {e}"),
                    }

                    if let Err(error) = self.db_update.apply_changes(indexer_uid).await {
                        error!("[DAEMON]: Error while removing file: {error}");
                    }
                }

                return;
            }

            let content_result = self.db_get.get_one(indexer_uid, db_path.clone()).await;

            trace!("Add/update event for file: {path}. Updating the database...");

            let result = match content_result {
                Ok(_) => self
                    .db_sync
                    .index_one_file(indexer_uid, path.clone(), FileOperationNeeded::Update)
                    .await
                    .err(),
                Err(DatabaseError::DomainError(DatabaseDomainError::NoResult)) => self
                    .db_sync
                    .index_one_file(indexer_uid, path.clone(), FileOperationNeeded::Add)
                    .await
                    .err(),
                Err(error) => Some(format!(
                    "Error while querying database for {db_path}: {error}"
                )),
            };

            match result {
                Some(e) => error!("[DAEMON]: Error while adding/updating file: {e}"),
                None => {
                    if let Err(error) = self.db_update.apply_changes(indexer_uid).await {
                        error!("[DAEMON]: Error while adding/updating file: {error}");
                    }
                }
            }
        } else {
            error!("Folder with id: {indexer_uid} not found!");
        }
    }
}
