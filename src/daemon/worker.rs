use std::path::MAIN_SEPARATOR_STR;
use std::{collections::HashMap, path::Path};

use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
use std::sync::Arc;
use tokio::sync::mpsc::Receiver;
use tokio::sync::{RwLock, mpsc::Sender};

use crate::file_indexer::indexer::{FileIndexer, FileOperationNeeded};
use crate::utils::trim_custom;

pub enum DaemonEvent {
    Add(String, Arc<RwLock<FileIndexer>>),
    Remove(String),
}

pub struct DaemonWorker {
    folders: HashMap<String, Arc<RwLock<FileIndexer>>>,
    rx: Receiver<notify::Result<notify::Event>>,
    rx_event: Receiver<DaemonEvent>,
    watcher: RecommendedWatcher,
}

impl DaemonWorker {
    pub fn new() -> Result<(Self, Sender<DaemonEvent>), String> {
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
                folders: HashMap::new(),
                rx: rx,
                rx_event: rx_event,
                watcher: watcher,
            },
            tx_event,
        ));
    }

    fn get_from_path(&self, path: &String) -> Option<(String, &Arc<RwLock<FileIndexer>>)> {
        return self
            .folders
            .iter()
            .find(|(f, _)| path.starts_with(f.as_str()))
            .map(|(f, i)| (path.strip_prefix(f).unwrap_or(path.as_str()).to_string(), i))
            .map(|(f, i)| (trim_custom(f, MAIN_SEPARATOR_STR), i)); // Avoid having something like \file1.txt or /file1.txt
    }

    pub fn add_folder(
        &mut self,
        folder: String,
        indexer: Arc<RwLock<FileIndexer>>,
    ) -> Result<(), String> {
        if !Path::new(&folder).is_absolute() {
            return Err(String::from("Path MUST be absolute"));
        }

        self.folders.insert(folder.clone(), indexer);

        self.watcher
            .watch(&Path::new(&folder), RecursiveMode::Recursive)
            .map_err(|e| e.to_string())?;

        return Ok(());
    }

    pub fn remove_folder(&mut self, folder: String) -> Result<(), String> {
        self.watcher
            .unwatch(&Path::new(&folder))
            .map_err(|e| e.to_string())?;

        self.folders.remove(&folder);

        return Ok(());
    }

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

    fn handle_manager_event(&mut self, event: Option<DaemonEvent>) {
        let error = match event {
            Some(event) => match event {
                DaemonEvent::Add(folder, indexer) => self.add_folder(folder, indexer),
                DaemonEvent::Remove(folder) => self.remove_folder(folder),
            },
            None => Ok(()),
        };

        if let Err(err) = error {
            // TODO: logs
            eprintln!("Error while handling event: {}", err);
        }
    }

    async fn handle_fs_event(&self, res: Option<notify::Result<notify::Event>>) {
        match res {
            Some(event_op) => {
                match event_op {
                    Ok(event) => {
                        for path in &event.paths {
                            let path = path.to_string_lossy().to_string();
                            let folder = self.get_from_path(&path);
                            if let Some((db_path, folder)) = folder {
                                process_event(&event, &path, db_path, folder).await;
                            } else {
                                // TODO: Log
                                eprintln!("File {} watched, but not in the watch folders", path);
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Watcher error: {}", e.to_string());
                    }
                }
            }
            None => {}
        }
    }
}

async fn process_event(
    event: &notify::Event,
    path: &String,
    db_path: String,
    folder: &Arc<RwLock<FileIndexer>>,
) {
    let mut folder_write = folder.write().await;
    // Check if it's not the system folder
    let error = if !path.starts_with(&folder_write.get_db_ro().get_index_path()) {
        if event.kind.is_modify() || event.kind.is_create() {
            // Already exists
            if folder_write.get_db_ro().get(db_path).is_ok() {
                folder_write
                    .index(path.clone(), FileOperationNeeded::Update)
                    .await
            // Doesn't exists
            } else {
                folder_write
                    .index(path.clone(), FileOperationNeeded::Add)
                    .await
            }
        } else if event.kind.is_remove() {
            let mut error = folder_write
                .get_db()
                .remove_document(db_path)
                .map_err(|e| e.to_string())
                .err();
            if error.is_none() {
                error = folder_write
                    .get_db()
                    .commit()
                    .map_err(|e| e.to_string())
                    .err();
            }
            error
        } else {
            None
        }
    } else {
        None
    };
    if let Some(error) = error {
        // TODO: Log
        eprintln!("Error while indexing {}: {}", path, error);
    }
}

#[cfg(test)]
mod tests {
    use std::{path::MAIN_SEPARATOR, time::Duration};

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
        let arc_new = arc.clone();
        let indexer_temp_dir_new = indexer_temp_dir.clone();
        let th = tokio::spawn(async move {
            let (mut worker, event_sender) = DaemonWorker::new().expect("Failed to create Daemon");

            event_sender
                .send(DaemonEvent::Add(indexer_temp_dir.clone(), arc.clone()))
                .await
                .expect("Failed to send add folder");

            worker.start_worker().await;
        });
        sleep(Duration::from_millis(1000)).await; // TODO: Maybe find a better way to do it?

        let mut file = fs::File::create(format!(
            "{}{}file1.txt",
            indexer_temp_dir_new, MAIN_SEPARATOR
        ))
        .await
        .expect("Failed to open file");
        file.write(b"Hello World~").await.expect("Write failed");

        let mut tries = 0;
        loop {
            let file1 = arc_new
                .read()
                .await
                .get_db_ro()
                .get(String::from("file1.txt"));
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
}
