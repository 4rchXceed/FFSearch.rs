use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};

#[cfg(feature = "mcp")]
use crate::manager::{RunningTask, Task};
use application::indexer::traits::registry::FileIndexerRegistry;
use application::searcher::actions::get::DatabaseGet;
use application::searcher::actions::search::SearchDocument;
#[cfg(feature = "daemon")]
use daemon::manager::DaemonManager;
use indexmap::IndexMap;
#[cfg(feature = "mcp")]
use settings::settings::Settings;

pub struct GuiState {
    pub searcher: SearchDocument,
    pub get_db: DatabaseGet,
    pub settings: Arc<Mutex<Settings>>,
    pub registry: Arc<dyn FileIndexerRegistry>,
    pub task_sender: Arc<Sender<Task>>,
    pub tasks: Arc<Mutex<IndexMap<u32, RunningTask>>>,
    #[cfg(feature = "daemon")]
    pub daemon_mgr: Arc<Mutex<DaemonManager>>,
}
