use std::fs::File;
use std::path::Path;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use application::indexer::actions::sync::FileIndexerSync;
use application::indexer::traits::registry::FileIndexerRegistry;
use domain::indexer::config::FileIndexerConfig;
use indexmap::IndexMap;
#[cfg(feature = "ai")]
use infrastructure::ai::rig::agent::RigAgent;
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
    #[cfg(feature = "ai")]
    AssignAI(u32),
}

pub struct TaskResult {
    pub success: bool,
    pub error: Option<String>,
}

pub struct RunningTaskData {
    pub name: String,
    pub result: Option<TaskResult>,
}

pub type RunningTask = Option<RunningTaskData>;

pub struct AppManager {
    settings: Arc<Mutex<Settings>>,
    folders_registry: Arc<dyn FileIndexerRegistry>,
    current_tasks: Arc<Mutex<IndexMap<u32, RunningTask>>>,
    sender: Arc<Sender<Task>>,
    startup_errors: Vec<String>,
    _th: JoinHandle<()>,
    first_opening: bool,
}

impl AppManager {
    pub async fn new(config_file: String) -> Result<Self, AppError> {
        let file = Path::new(&config_file);
        let mut first_opening = false;

        if !file.exists() {
            File::create(&config_file).map_err(|e| {
                AppError::FailedToLoadSettings(format!("Failed to create new config file: {}", e))
            })?;

            first_opening = true;
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

        let settings = Arc::new(Mutex::new(settings));
        let tasks = Arc::new(Mutex::new(IndexMap::new()));

        let th_registry = folders_registry.clone();
        let th_tasks = tasks.clone();

        let th_settings = settings.clone();
        let th = tokio::spawn(async move {
            Self::worker(recv, syncer, th_registry, th_tasks, th_settings).await;
        });

        return Ok(Self {
            startup_errors: errors,
            settings: settings,
            folders_registry: folders_registry,
            current_tasks: tasks,
            sender: Arc::new(sender),
            _th: th,
            first_opening: first_opening,
        });
    }

    #[allow(unused)]
    pub async fn worker(
        rx: Receiver<Task>,
        syncer: Arc<FileIndexerSync>,
        registry: Arc<dyn FileIndexerRegistry>,
        current_tasks: Arc<Mutex<IndexMap<u32, RunningTask>>>,
        settings: Arc<Mutex<Settings>>,
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
                    #[cfg(feature = "ai")]
                    Task::AssignAI(folder) => {
                        let folder_path = registry
                            .get_folder(folder)
                            .await
                            .unwrap_or(String::from("Unknown"));

                        let settings_clone = settings.clone();
                        let registry_clone = registry.clone();

                        let th = Self::run_task(
                            current_tasks.clone(),
                            format!("Creating agent and building RAG for: {folder_path}"),
                            async move {
                                let config = if let Ok(s) = settings_clone.lock() {
                                    s.ai.clone()
                                } else {
                                    return Err(format!("Could not get AI settings"));
                                };

                                let db = registry_clone
                                    .get_database(folder)
                                    .await
                                    .ok_or(format!("Could not get database"))?;

                                let ai_provider = RigAgent::new_auto_provider(config, db)
                                    .await
                                    .map_err(|e| format!("Failed to create the AI Agent: {e}"))?;

                                let agent = Arc::new(ai_provider);

                                registry_clone
                                    .assign_agent(folder, agent)
                                    .await
                                    .map_err(|_| {
                                        String::from("Failed to add the agent to the indexer :(")
                                    })?;

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
        current_tasks: Arc<Mutex<IndexMap<u32, RunningTask>>>,
        name: String,
        fun: F,
    ) -> Option<JoinHandle<()>>
    where
        F: Future<Output = Result<(), String>> + Send + 'static,
    {
        let id = {
            let tasks_res = current_tasks.lock();

            if let Ok(mut tasks) = tasks_res {
                let len = tasks.len();
                tasks.insert(
                    len as u32,
                    Some(RunningTaskData {
                        name: name.to_string(),
                        result: None,
                    }),
                );
                len // Return the last index
            } else {
                return None;
            }
        };

        let handle = tokio::spawn(async move {
            let result = fun.await;
            if let Ok(mut tasks) = current_tasks.lock() {
                if let Some(task) = tasks.get_mut(&(id as u32)) {
                    if let Some(task_data) = task {
                        task_data.result = Some(TaskResult {
                            success: result.is_ok(),
                            error: result.err(),
                        });
                    }
                } else {
                    error!("Could not find the task {id}??");
                }
            }

            tokio::time::sleep(Duration::from_secs(10)).await;

            if let Ok(mut tasks) = current_tasks.lock() {
                if id < tasks.len() {
                    tasks.insert(id as u32, None);
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
            self.first_opening,
        ) {
            Ok(_) => {}
            Err(e) => show_error(AppError::OpenUIError(e.to_string())),
        }
    }
}
