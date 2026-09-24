use std::{collections::HashMap, sync::Arc};

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
    #[error("The file indexer has been deleted")]
    FileIndexerDeleted,
}

pub struct FileIndexerObjects {
    pub indexer: FileIndexer,
    #[cfg(feature = "ai")]
    pub agent: Option<Agent>,
}

impl FileIndexerObjects {
    pub fn indexer(&self) -> &FileIndexer {
        return &self.indexer;
    }

    pub fn indexer_mut(&mut self) -> &mut FileIndexer {
        return &mut self.indexer;
    }
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
                indexer: indexer,
                #[cfg(feature = "ai")]
                agent: None,
            },
        );

        return Ok(id);
    }

    pub async fn get_file_indexers(&self) -> HashMap<u32, String> {
        return self
            .file_indexers
            .iter()
            .map(|(id, i)| (*id, i.indexer.get_folder()))
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
        let folder = self
            .file_indexers
            .get(&folder_id)
            .ok_or(ControllerError::FolderWithIdNotFound(folder_id))?
            .indexer
            .get_folder();

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
        let folder = self
            .file_indexers
            .get(&folder_id)
            .ok_or(ControllerError::FolderWithIdNotFound(folder_id))?
            .indexer
            .get_folder();

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

    pub fn get_controller_for(
        &self,
        folder_id: u32,
        me: LockedController,
    ) -> Result<FolderController, ControllerError> {
        if !self.file_indexers.contains_key(&folder_id) {
            return Err(ControllerError::FolderWithIdNotFound(folder_id));
        }

        return Ok(FolderController::new(me, folder_id));
    }

    #[cfg(feature = "ai")]
    pub async fn create_agent(me: LockedController, folder_id: u32) -> Result<(), ControllerError> {
        let agent_exists = {
            let me_read = me.read().await;

            me_read
                .file_indexers
                .get(&folder_id)
                .ok_or(ControllerError::FolderWithIdNotFound(folder_id))?
                .agent
                .is_some()
        };

        if agent_exists {
            return Err(ControllerError::AgentAlreadyExists(folder_id));
        };

        let config = me.read().await.config.ai.clone();

        let controller = me.read().await.get_controller_for(folder_id, me.clone())?;

        let agent = match config.provider {
            AgentProvider::OpenAI => {
                let client = rig::providers::openai::Client::new(config.api_key.clone())
                    .map_err(|e| ControllerError::ApiConnectionError(e))?;

                let embeddings = client.embedding_model(config.rag_config.embedding_model.clone());

                Agent::new_auto(client, config, controller, embeddings).await
            }
            AgentProvider::Ollama => {
                let client = rig::providers::ollama::Client::new(config.api_key.clone())
                    .map_err(|e| ControllerError::ApiConnectionError(e))?;

                let embeddings = client.embedding_model(config.rag_config.embedding_model.clone());

                Agent::new_auto(client, config.clone(), controller, embeddings).await
            }
        };

        let agent = agent.map_err(|e| ControllerError::FailedToCreateAgent(e))?;

        me.write()
            .await
            .file_indexers
            .get_mut(&folder_id)
            .ok_or(ControllerError::FolderWithIdNotFound(folder_id))?
            .agent = Some(agent);

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
    controller: Arc<RwLock<Controller>>,
    uid: u32,
}

impl FolderController {
    pub fn new(controller: Arc<RwLock<Controller>>, uid: u32) -> FolderController {
        return FolderController {
            controller,
            uid: uid,
        };
    }

    pub async fn search(
        &self,
        query: String,
        limit: usize,
    ) -> Option<Result<Vec<Document>, DatabaseError>> {
        let controller = self.controller.read().await;
        let indexer = controller.file_indexers.get(&self.uid)?.indexer();

        let search_result = indexer.get_db_ro().search(query, limit);

        return Some(search_result);
    }

    pub async fn get_content(&self, path: String) -> Option<Result<Document, DatabaseError>> {
        let controller = self.controller.read().await;
        let indexer = controller.file_indexers.get(&self.uid)?.indexer();

        let content_result = indexer.get_db_ro().get(path);

        return Some(content_result);
    }

    pub async fn get_everything(
        &self,
        optimized: bool,
    ) -> Option<Result<Vec<Document>, DatabaseError>> {
        let controller = self.controller.read().await;
        let indexer = controller.file_indexers.get(&self.uid)?.indexer();

        let content_result = indexer.get_db_ro().get_all(optimized);

        return Some(content_result);
    }

    pub async fn get_db_path(&self) -> Option<String> {
        let controller = self.controller.read().await;
        let indexer = controller.file_indexers.get(&self.uid)?.indexer();

        let db_path = indexer.get_db_ro().get_index_path();

        return Some(db_path);
    }

    pub async fn get_path(&self) -> Option<String> {
        let controller = self.controller.read().await;
        let indexer = controller.file_indexers.get(&self.uid)?.indexer();

        let path = indexer.get_folder();

        return Some(path);
    }

    pub async fn index_file_change(
        &self,
        real_path: String,
        operation: FileOperationNeeded,
    ) -> Option<Option<String>> {
        let mut controller = self.controller.write().await;
        let indexer = controller.file_indexers.get_mut(&self.uid)?.indexer_mut();

        let result = indexer.index(real_path, operation).await;

        return Some(result);
    }

    pub async fn remove_document(&self, db_path: String) -> Option<Result<(), DatabaseError>> {
        let mut controller = self.controller.write().await;
        let indexer = controller.file_indexers.get_mut(&self.uid)?.indexer_mut();

        let remove_result = indexer.get_db().remove_document(db_path);

        let post_result = match remove_result {
            Ok(_) => indexer.get_db().commit(),
            Err(e) => Err(e),
        };

        return Some(post_result);
    }

    pub async fn sync_files(&self) -> Option<Result<Vec<FileIndexerError>, FileIndexerError>> {
        let mut controller = self.controller.write().await;
        let indexer = controller.file_indexers.get_mut(&self.uid)?.indexer_mut();

        let sync_results = indexer.sync();

        return Some(sync_results);
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs::{File, create_dir},
        io::Write,
        path::PathBuf,
    };

    use tempdir::TempDir;

    use super::*;

    const DEFAULT_CONFIG: &str = "
    [FILE_INDEXER]
    THREADS=1

    [DATABASE]
    DB_FOLDER_PREFIX=.db
    ";

    fn create_temp_dir() -> TempDir {
        let dir = TempDir::new(".ffsearch-test").expect("Failed to create test directory");

        return dir;
    }

    fn write_file(to: PathBuf, content: &str) {
        let mut file = File::create(to).expect("Failed to create config file");

        file.write(content.as_bytes())
            .expect("Failed to write config file");
    }

    /// Tests the FolderController class
    /// This test is testing:
    /// - Controller creation
    /// - get_settings()
    #[test]
    fn test_controller_settings() {
        let dir = create_temp_dir();
        let path = dir.path();

        let config_path = path.join("config.ini");

        write_file(config_path.clone(), DEFAULT_CONFIG);

        let controller = Controller::new(config_path.to_string_lossy().to_string())
            .expect("Failed to create controller");

        assert_eq!(controller.get_settings().file_indexer.number_of_threads, 1);
        assert_eq!(
            controller
                .get_settings()
                .file_indexer
                .db_config
                .database_folder_prefix,
            ".db"
        );
    }

    /// Tests the FolderController class
    /// This test is testing:
    /// - Controller creation
    /// - File indexer creation
    /// - Folder-specific controller creation
    /// - remove_document()
    /// - get_content()
    /// - index_file_change()
    #[tokio::test]
    async fn test_folder_controller_file_operations() {
        // ---------------------------------------
        // Tear up
        // ---------------------------------------
        // Create the directories and files
        let dir = create_temp_dir();
        let path = dir.path();

        let config_path = path.join("config.ini");
        let sync_path = path.join("sync");
        let file1_path = sync_path.join("file1.txt");

        create_dir(sync_path.clone()).expect("Failed to create sync dir");

        write_file(config_path.clone(), DEFAULT_CONFIG);
        write_file(file1_path.clone(), "Hatsune Miku 2 ^.^");

        // Create the controller
        let controller = Arc::new(RwLock::new(
            Controller::new(config_path.to_string_lossy().to_string())
                .expect("Failed to create controller"),
        ));

        // Create the indexer
        let indexer_id = controller
            .write()
            .await
            .add_file_indexer(sync_path.to_string_lossy().to_string())
            .expect("Failed to create file indexer");

        // Get the controller for the file indexer
        let indexer_controller = controller
            .read()
            .await
            .get_controller_for(indexer_id, controller.clone())
            .expect("Failed to create indexer controller");

        // ---------------------------------------
        // Test 1: Force the index of file1.txt
        // ---------------------------------------
        // Action
        indexer_controller
            .index_file_change(String::from("file1.txt"), FileOperationNeeded::Add)
            .await
            .expect("Failed to index one file");

        // Check
        let content = indexer_controller
            .get_content(String::from("file1.txt"))
            .await
            .expect("File indexer deleted")
            .expect("Failed to get file content");

        assert_eq!(
            content.content.expect("Content not bundled"),
            "Hatsune Miku 2 ^.^"
        );

        // ---------------------------------------
        // Test 1: Force the remove of file1.txt
        // ---------------------------------------
        // Action
        indexer_controller
            .remove_document(String::from("file1.txt"))
            .await
            .expect("File indexer deleted")
            .expect("Failed to remove the file");

        // Check
        indexer_controller
            .get_content(String::from("file1.txt"))
            .await
            .expect("File indexer deleted")
            .expect_err("File is still here");
    }

    /// Tests the FolderController class
    /// This test is testing:
    /// - Controller creation
    /// - File indexer creation
    /// - Folder-specific controller creation
    /// - sync_files()
    /// - get_everything()
    /// - get_db_path()
    /// - get_path()
    #[tokio::test]
    async fn test_folder_controller() {
        // ---------------------------------------
        // Tear up
        // ---------------------------------------
        // Create the directories and files
        let dir = create_temp_dir();
        let path = dir.path();

        let config_path = path.join("config.ini");
        let sync_path = path.join("sync");
        let file1_path = sync_path.join("file1.txt");

        create_dir(sync_path.clone()).expect("Failed to create sync dir");

        write_file(config_path.clone(), DEFAULT_CONFIG);
        write_file(file1_path.clone(), "Hatsune Miku 2 ^.^");

        // Create the controller
        let controller = Arc::new(RwLock::new(
            Controller::new(config_path.to_string_lossy().to_string())
                .expect("Failed to create controller"),
        ));

        // Create the indexer
        let indexer_id = controller
            .write()
            .await
            .add_file_indexer(sync_path.to_string_lossy().to_string())
            .expect("Failed to create file indexer");

        // Get the controller for the indexer
        let indexer_controller = controller
            .read()
            .await
            .get_controller_for(indexer_id, controller.clone())
            .expect("Failed to create indexer controller");

        // ---------------------------------------
        // Test the functions: sync_files + get_everything
        // ---------------------------------------
        // Action
        indexer_controller
            .sync_files()
            .await
            .expect("File indexer deleted")
            .expect("Sync failed");

        // Action 2
        let everything = indexer_controller
            .get_everything(false)
            .await
            .expect("File indexer deleted")
            .expect("Failed to get everything");

        // Checks
        assert_eq!(everything.len(), 1);
        assert_eq!(everything[0].path, "file1.txt");
        assert_eq!(
            everything[0].content,
            Some(String::from("Hatsune Miku 2 ^.^"))
        );

        // ---------------------------------------
        // Test get_db_path() and get_path()
        // ---------------------------------------
        // get_db_path test
        let db_path = indexer_controller
            .get_db_path()
            .await
            .expect("File indexer deleted");

        // Check
        assert_eq!(db_path, sync_path.join(".db").to_string_lossy().to_string());

        // get_path test
        let folder_path = indexer_controller
            .get_path()
            .await
            .expect("File indexer deleted");

        // Check
        assert_eq!(folder_path, sync_path.to_string_lossy().to_string());
    }

    /// Basic search test
    /// This test is testing:
    /// - Controller creation
    /// - File indexer creation
    /// - Folder-specific controller creation
    /// - get_file_indexers()
    /// - sync_files()
    /// - get_content()
    ///
    #[tokio::test]
    async fn test_basic_search() {
        // ---------------------------------------
        // Tear up
        // ---------------------------------------
        // Create the directories and files
        let dir = create_temp_dir();
        let path = dir.path();

        let config_path = path.join("config.ini");
        let sync_path = path.join("sync");
        let file1_path = sync_path.join("file1.txt");

        create_dir(sync_path.clone()).expect("Failed to create sync dir");

        write_file(config_path.clone(), DEFAULT_CONFIG);
        write_file(file1_path.clone(), "Hatsune Miku ^.^");

        // Create the controller
        let controller = Arc::new(RwLock::new(
            Controller::new(config_path.to_string_lossy().to_string())
                .expect("Failed to create controller"),
        ));

        // Create the file indexer
        let indexer_id = controller
            .write()
            .await
            .add_file_indexer(sync_path.to_string_lossy().to_string())
            .expect("Failed to create file indexer");

        // ---------------------------------------
        // Tests get_file_indexers()
        // ---------------------------------------
        assert_eq!(
            controller.read().await.get_file_indexers().await,
            HashMap::from([(1, sync_path.to_string_lossy().to_string())])
        );

        // Gets the controller for the file indexer
        let indexer_controller = controller
            .read()
            .await
            .get_controller_for(indexer_id, controller.clone())
            .expect("Failed to create indexer controller");

        // ---------------------------------------
        // Tests sync_files() and get_content()
        // ---------------------------------------
        // sync_files action
        indexer_controller
            .sync_files()
            .await
            .expect("File indexer deleted")
            .expect("Sync failed");

        // get_content action
        let content = indexer_controller
            .get_content(file1_path.to_string_lossy().to_string())
            .await
            .expect("File indexer deleted")
            .expect("Failed to get file content");

        // Check
        assert_eq!(
            content.content.expect("Content not bundled"),
            "Hatsune Miku ^.^"
        );
    }
}
