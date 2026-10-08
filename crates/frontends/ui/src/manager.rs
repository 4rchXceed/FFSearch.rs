use std::fs::File;
use std::path::Path;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use application::indexer::actions::sync::FileIndexerSync;
use application::indexer::traits::registry::FileIndexerRegistry;
use domain::indexer::config::FileIndexerConfig;
use infrastructure::indexer::stdfile::indexer::StdFileIndexerProvider;
use infrastructure::indexer::stdfile::registry::InMemoryRegistry;
use infrastructure::indexer::xberg::extractor::XbergFileExtractor;
use infrastructure::searcher::tantivy::database::TantivyDatabase;
use log::error;
use settings::settings::Settings;
use tokio::task::JoinHandle;

use crate::errors::{AppError, show_error};
use crate::gui::gui_main::run_gui;

pub enum Task {
    SyncFolder(u32),
}

pub struct TaskResult {
    pub success: bool,
    pub error: Option<String>,
}

pub struct RunningTask {
    pub name: String,
    pub result: Option<TaskResult>,
}

pub struct AppManager {
    settings: Arc<Mutex<Settings>>,
    folders_registry: Arc<dyn FileIndexerRegistry>,
    current_tasks: Arc<Mutex<Vec<RunningTask>>>,
    sender: Arc<Sender<Task>>,
    startup_errors: Vec<String>,
    _th: JoinHandle<()>,
}

impl AppManager {
    pub async fn new(config_file: String) -> Result<Self, AppError> {
        let file = Path::new(&config_file);

        if !file.exists() {
            File::create(&config_file).map_err(|e| {
                AppError::FailedToLoadSettings(format!("Failed to create new config file: {}", e))
            })?;
        }

        let settings =
            Settings::load(config_file).map_err(|e| AppError::FailedToLoadSettings(e))?;

        let folders_registry = Arc::new(InMemoryRegistry::new());
        // TODO: Make it global settings and configurable
        let text_extractor = Arc::new(XbergFileExtractor::new(&FileIndexerConfig::new()));

        let syncer = Arc::new(FileIndexerSync::new(
            folders_registry.clone(),
            text_extractor,
        ));

        let mut errors = Vec::new();

        for (path, config) in &settings.file_indexers {
            let file_indexer = Arc::new(StdFileIndexerProvider::new(config.clone()));

            let database_res = TantivyDatabase::new(path.clone(), config.db_config.clone());

            match database_res {
                Ok(db) => {
                    folders_registry
                        .add(file_indexer, Arc::new(db), path.clone())
                        .await;
                }
                Err(e) => {
                    error!("Startup error for {path}: {e}");
                    errors.push(format!("Error while starting manager for {path}: {e}"));
                }
            }
        }

        let (sender, recv) = std::sync::mpsc::channel();

        let tasks = Arc::new(Mutex::new(Vec::new()));

        let th_registry = folders_registry.clone();
        let th_tasks = tasks.clone();

        let th = tokio::spawn(async move {
            Self::worker(recv, syncer, th_registry, th_tasks).await;
        });

        let settings = Arc::new(Mutex::new(settings));

        return Ok(Self {
            startup_errors: errors,
            settings: settings,
            folders_registry: folders_registry,
            current_tasks: tasks,
            sender: Arc::new(sender),
            _th: th,
        });
    }

    pub async fn worker(
        rx: Receiver<Task>,
        syncer: Arc<FileIndexerSync>,
        registry: Arc<dyn FileIndexerRegistry>,
        current_tasks: Arc<Mutex<Vec<RunningTask>>>,
    ) {
        let mut pool = Vec::new();

        loop {
            if let Ok(task) = rx.recv() {
                match task {
                    Task::SyncFolder(id) => {
                        let folder_path = registry
                            .get_folder(id)
                            .await
                            .unwrap_or(String::from("Unknown"));
                        let syncer = syncer.clone();

                        let th = Self::run_task(
                            current_tasks.clone(),
                            format!("Syncing files in {folder_path}"),
                            async move {
                                let errors = syncer.sync(id).await.map_err(|e| e.to_string())?;
                                for error in &errors {
                                    println!("Error while syncing document: {error}");
                                }

                                if !errors.is_empty() {
                                    return Err(format!(
                                        "{} errors occurred during syncing. This is not very important, but you can check the logs",
                                        errors.len()
                                    ));
                                }
                                return Ok(());
                            },
                        );

                        pool.push(th);

                        tokio::task::yield_now().await;
                    }
                }
            }
        }
    }

    pub fn run_task<F>(
        current_tasks: Arc<Mutex<Vec<RunningTask>>>,
        name: String,
        fun: F,
    ) -> Option<JoinHandle<()>>
    where
        F: Future<Output = Result<(), String>> + Send + 'static,
    {
        let id = {
            let tasks_res = current_tasks.lock();

            if let Ok(mut tasks) = tasks_res {
                tasks.push(RunningTask {
                    name: name.to_string(),
                    result: None,
                });
                tasks.len() - 1 // Return the last index
            } else {
                return None;
            }
        };

        let handle = tokio::spawn(async move {
            let result = fun.await;
            if let Ok(mut tasks) = current_tasks.lock() {
                if let Some(task) = tasks.get_mut(id) {
                    task.result = Some(TaskResult {
                        success: result.is_ok(),
                        error: result.err(),
                    });
                } else {
                    error!("Could not find the task {id}??");
                }
            }

            tokio::time::sleep(Duration::from_secs(4)).await;

            if let Ok(mut tasks) = current_tasks.lock() {
                if id < tasks.len() {
                    tasks.remove(id);
                }
            }
        });

        return Some(handle);
    }

    pub fn open_ui(&mut self) {
        match run_gui(
            self.folders_registry.clone(),
            self.settings.clone(),
            self.sender.clone(),
            self.current_tasks.clone(),
            self.startup_errors.clone(),
        ) {
            Ok(_) => {}
            Err(e) => show_error(AppError::OpenUIError(e.to_string())),
        }
    }
}
