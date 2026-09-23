use std::{collections::HashMap, sync::Arc};

use futures::future::join_all;
use thiserror::Error;
use tokio::sync::mpsc::Receiver;
use tokio::sync::{RwLock, mpsc::error::SendError};

#[cfg(feature = "ai")]
use crate::ai::agent::AIError;
#[cfg(feature = "ai")]
use crate::ai::agent::Agent;
#[cfg(feature = "ai")]
use crate::ai::config::AgentProvider;
use crate::file_indexer::indexer::FileOperationNeeded;
#[cfg(feature = "ai")]
use rig::client::EmbeddingsClient;

#[cfg(feature = "ai")]
use crate::daemon::{manager::DaemonManager, worker::DaemonEvent};

use crate::{
    controller::settings::Settings,
    file_indexer::indexer::{FileIndexer, FileIndexerError},
    searcher::database::{DatabaseError, Document},
};

pub type LockedController = Arc<RwLock<Controller>>;

#[derive(Debug, Error)]
pub enum ControllerInitError {
    #[error("Settings load failed! {0}")]
    SettingsLoadFailed(String),
    #[cfg(feature = "daemon")]
    #[error("Daemon manager creation failed! {0}")]
    DaemonManagerFailedToStart(String),
}

#[derive(Debug, Error)]
pub enum ControllerError {
    #[error("Folder with id: {0} not found!")]
    FolderWithIdNotFound(u32),
    #[error("Error while searching: {0}")]
    SearchError(DatabaseError),
    #[cfg(feature = "daemon")]
    #[error("Error while adding the folder to the daemon: {0} (Maybe the daemon has crashed?)")]
    DaemonAddFolderError(SendError<DaemonEvent>),
    #[cfg(feature = "ai")]
    #[error("An agent has already been created at for folder: {0}")]
    AgentAlreadyExists(u32),
    #[cfg(feature = "ai")]
    #[error("Error while connecting with LLM api: {0}")]
    ApiConnectionError(rig::http_client::Error),
    #[cfg(feature = "ai")]
    #[error("Failed to create agent: {0}")]
    FailedToCreateAgent(AIError),
    #[cfg(feature = "ai")]
    #[error("Agent not found on folder with id: {0}, but tried to do something with it")]
    AgentNotFoundOn(u32),
    #[cfg(feature = "daemon")]
    #[error("Daemon not enabled")]
    DaemonNotEnabled,
    #[cfg(feature = "daemon")]
    #[error("Faild to stop the daemon :(")]
    FailedToStopTheDaemon,
}

pub struct FileIndexerObjects {
    pub indexer_ref: Arc<RwLock<FileIndexer>>,
    #[cfg(feature = "ai")]
    pub agent: Option<Agent>,
}

pub struct Controller {
    file_indexers: HashMap<u32, FileIndexerObjects>,
    current_uid: u32,
    config: Settings,
    #[cfg(feature = "daemon")]
    daemon_manager: Option<DaemonManager>,
}

impl Controller {
    pub fn new(config_file: String) -> Result<Self, ControllerInitError> {
        let settings =
            Settings::load(config_file).map_err(|e| ControllerInitError::SettingsLoadFailed(e))?;

        return Ok(Self {
            file_indexers: HashMap::new(),
            current_uid: 0,
            config: settings,
            #[cfg(feature = "daemon")]
            daemon_manager: None,
        });
    }

    #[cfg(feature = "daemon")]
    pub fn enable_daemon(&mut self, me: LockedController) -> Result<(), ControllerInitError> {
        let daemon = DaemonManager::new(me)
            .map_err(|e| ControllerInitError::DaemonManagerFailedToStart(e))?;

        self.daemon_manager = Some(daemon);

        return Ok(());
    }

    pub fn get_settings(&self) -> &Settings {
        return &self.config;
    }

    fn generate_uid(&mut self) -> u32 {
        self.current_uid += 1;

        return self.current_uid;
    }

    pub fn add_file_indexer(&mut self, folder: String) -> Result<u32, FileIndexerError> {
        let indexer = FileIndexer::new(folder.clone(), self.config.file_indexer.clone())?;

        let id = self.generate_uid();

        self.file_indexers.insert(
            id,
            FileIndexerObjects {
                indexer_ref: Arc::new(RwLock::new(indexer)),
                #[cfg(feature = "ai")]
                agent: None,
            },
        );

        return Ok(id);
    }

    pub async fn get_file_indexers(&self) -> HashMap<u32, String> {
        return join_all(self.file_indexers.iter().map(|(id, indexer)| async move {
            (*id, indexer.indexer_ref.read().await.get_folder())
        }))
        .await
        .into_iter()
        .collect();
    }

    #[cfg(feature = "daemon")]
    pub fn start_daemon(&mut self) -> Result<(), ControllerError> {
        if let Some(daemon) = &mut self.daemon_manager {
            daemon.start();

            return Ok(());
        } else {
            return Err(ControllerError::DaemonNotEnabled);
        }
    }

    #[cfg(feature = "daemon")]
    pub async fn stop_daemon(&mut self) -> Result<(), ControllerError> {
        if let Some(daemon) = &mut self.daemon_manager {
            return daemon
                .stop_daemon()
                .await
                .map_err(|_| ControllerError::FailedToStopTheDaemon);
        } else {
            return Err(ControllerError::DaemonNotEnabled);
        }
    }

    #[cfg(feature = "daemon")]
    pub async fn assign_to_daemon(&mut self, folder_id: u32) -> Result<(), ControllerError> {
        let folder = self.get_controller_for(folder_id)?.get_path().await;

        if let Some(daemon) = &mut self.daemon_manager {
            daemon
                .add_folder(folder)
                .await
                .map_err(|e| ControllerError::DaemonAddFolderError(e))?;

            return Ok(());
        } else {
            return Err(ControllerError::DaemonNotEnabled);
        }
    }

    #[cfg(feature = "daemon")]
    pub async fn remove_from_daemon(&mut self, folder_id: u32) -> Result<(), ControllerError> {
        let folder = self.get_controller_for(folder_id)?.get_path().await;

        if let Some(daemon) = &mut self.daemon_manager {
            daemon
                .remove_folder(folder)
                .await
                .map_err(|e| ControllerError::DaemonAddFolderError(e))?;

            return Ok(());
        } else {
            return Err(ControllerError::DaemonNotEnabled);
        }
    }

    pub fn get_controller_for(&self, folder_id: u32) -> Result<FolderController, ControllerError> {
        let folder = self
            .file_indexers
            .get(&folder_id)
            .ok_or(ControllerError::FolderWithIdNotFound(folder_id))?;

        return Ok(FolderController::new(folder.indexer_ref.clone(), folder_id));
    }

    #[cfg(feature = "ai")]
    pub async fn create_agent(&mut self, folder_id: u32) -> Result<(), ControllerError> {
        let folder = self
            .file_indexers
            .get_mut(&folder_id)
            .ok_or(ControllerError::FolderWithIdNotFound(folder_id))?;

        if folder.agent.is_some() {
            return Err(ControllerError::AgentAlreadyExists(folder_id));
        }

        let agent = match self.config.ai.provider {
            AgentProvider::OpenAI => {
                let client = rig::providers::openai::Client::new(self.config.ai.api_key.clone())
                    .map_err(|e| ControllerError::ApiConnectionError(e))?;

                let embeddings =
                    client.embedding_model(self.config.ai.rag_config.embedding_model.clone());

                Agent::new_auto(
                    client,
                    self.config.ai.clone(),
                    folder.indexer_ref.clone(),
                    embeddings,
                )
                .await
            }
            AgentProvider::Ollama => {
                let client = rig::providers::ollama::Client::new(self.config.ai.api_key.clone())
                    .map_err(|e| ControllerError::ApiConnectionError(e))?;

                let embeddings =
                    client.embedding_model(self.config.ai.rag_config.embedding_model.clone());

                Agent::new_auto(
                    client,
                    self.config.ai.clone(),
                    folder.indexer_ref.clone(),
                    embeddings,
                )
                .await
            }
        };

        let agent = agent.map_err(|e| ControllerError::FailedToCreateAgent(e))?;

        folder.agent = Some(agent);

        return Ok(());
    }

    #[cfg(feature = "ai")]
    pub fn ask_ai(
        &mut self,
        folder_id: u32,
        message: String,
    ) -> Result<(Receiver<String>, impl Future<Output = Result<(), AIError>>), ControllerError>
    {
        let objects = self
            .file_indexers
            .get_mut(&folder_id)
            .ok_or(ControllerError::FolderWithIdNotFound(folder_id))?;

        if let Some(agent) = &mut objects.agent {
            let (tx, rx) = tokio::sync::mpsc::channel::<String>(100);

            return Ok((rx, agent.send(message, tx)));
        } else {
            return Err(ControllerError::AgentNotFoundOn(folder_id));
        }
    }

    pub async fn remove(&mut self, folder_id: u32) -> Result<(), ControllerError> {
        self.file_indexers
            .remove(&folder_id)
            .ok_or(ControllerError::FolderWithIdNotFound(folder_id))?;

        return Ok(());
    }
}

pub struct FolderController {
    indexer: Arc<RwLock<FileIndexer>>,
    uid: u32,
}

impl FolderController {
    pub fn new(indexer: Arc<RwLock<FileIndexer>>, uid: u32) -> FolderController {
        return FolderController {
            indexer: indexer,
            uid: uid,
        };
    }

    pub async fn search(
        &self,
        query: String,
        limit: usize,
    ) -> Result<Vec<Document>, DatabaseError> {
        return self.indexer.read().await.get_db_ro().search(query, limit);
    }

    pub async fn get_content(&self, path: String) -> Result<Document, DatabaseError> {
        return self.indexer.read().await.get_db_ro().get(path);
    }

    pub async fn get_db_path(&self) -> String {
        return self.indexer.read().await.get_db_ro().get_index_path();
    }

    pub async fn get_path(&self) -> String {
        return self.indexer.read().await.get_folder();
    }

    pub async fn index_file_change(
        &self,
        real_path: String,
        operation: FileOperationNeeded,
    ) -> Option<String> {
        return self.indexer.write().await.index(real_path, operation).await;
    }

    pub async fn remove_document(&self, db_path: String) -> Result<(), DatabaseError> {
        let remove_result = self.indexer.write().await.get_db().remove_document(db_path);
        return match remove_result {
            Ok(_) => self.indexer.write().await.get_db().commit(),
            Err(e) => Err(e),
        };
    }
}
