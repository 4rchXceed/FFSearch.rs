use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex};

use application::indexer::traits::registry::FileIndexerRegistry;
use application::searcher::actions::get::DatabaseGet;
use application::searcher::actions::search::SearchDocument;
#[cfg(feature = "daemon")]
use daemon::manager::DaemonManager;
use iced_xml::app_manager::app_manager::WindowManager;
use iced_xml::app_manager::state_wrapper::App;
use iced_xml::app_manager::utils::{CloseMode, WindowId};
use iced_xml::window_manager;
use iced_xml::window_wrapper::AppResult;
use iced_xml::xml_struct::xml_element::XmlElement;
use indexmap::IndexMap;
use settings::settings::Settings;

#[cfg(feature = "ai")]
use crate::gui::windows::aichat::AiChatWindow;
use crate::manager::{RunningTask, Task};

use super::params::{WindowParams, WindowType};
use super::state::GuiState;
use super::windows::details::DetailsWindow;
use super::windows::errors::WindowCreationError;
use super::windows::home::HomeWindow;
use super::windows::results::ResultsWindow;
use super::windows::settings::SettingsWindow;
use super::windows::startup::StartupWindow;
use super::windows::tasks::TasksWindow;
use super::windows::winerror::ErrorWindow;

fn create_window_safe(
    params: WindowParams,
    app: &mut App<GuiState>,
    win_id: WindowId,
) -> Result<Window, WindowCreationError> {
    let window = match params.wintype {
        WindowType::HomeWindow(first_opening, startup_errors) => {
            let mut win = HomeWindow::new(startup_errors)?;

            win.post_create(app, first_opening)?;

            Window::HomeWindow(win)
        }
        WindowType::ErrorWindow(error) => {
            let mut win = ErrorWindow::new()?;

            win.post_create(error);

            Window::ErrorWindow(win)
        }
        WindowType::SettingsWindow => {
            let mut win = SettingsWindow::new()?;

            win.post_create(app);

            Window::SettingsWindow(win)
        }
        WindowType::StartupWindow => {
            let mut win = StartupWindow::new(win_id)?;

            win.post_create();

            Window::StartupWindow(win)
        }
        WindowType::ResultsWindow(results, query, indexer_id) => {
            let mut win = ResultsWindow::new(indexer_id)?;

            win.post_create(results, query, app);

            Window::ResultsWindow(win)
        }
        WindowType::DetailsWindow(path, indexer_id) => {
            let mut win = DetailsWindow::new()?;

            win.post_create(path, indexer_id, app);

            Window::DetailsWindow(win)
        }
        #[cfg(feature = "ai")]
        WindowType::AiChatWindow(prompt, indexer_id) => {
            let mut win = AiChatWindow::new(indexer_id, app)?;

            win.post_create(prompt, app);

            Window::AiChatWindow(win)
        }
        #[cfg(not(feature = "ai"))]
        #[allow(unused)]
        _ => {
            let mut win = ErrorWindow::new()?;

            win.post_create(format!("Window not supported as standalone Window :("));

            Window::ErrorWindow(win)
        }
    };

    return Ok(window);
}

pub fn text(text: &str) -> XmlElement {
    let mut label = XmlElement::void();

    label.set_tag("Label");
    label.set_text(text);

    return label;
}

fn create_window(params: WindowParams, state: &mut App<GuiState>, win_id: WindowId) -> Window {
    match create_window_safe(params, state, win_id) {
        Ok(win) => win,
        Err(error) => create_window_safe(WindowParams::error(error.to_string()), state, win_id)
            .map_err(|e| e.to_string())
            .expect("Failed to even create the error window: FATAL ERROR"),
    }
}

#[cfg(not(feature = "ai"))]
type AiChatWindow = ErrorWindow;

window_manager!(Window {
    HomeWindow,
    ErrorWindow,
    TasksWindow,
    SettingsWindow,
    StartupWindow,
    ResultsWindow,
    DetailsWindow,
    AiChatWindow
}; GuiState; WindowParams);

impl Window {
    pub fn component_task(app: &mut App<GuiState>) -> Window {
        match Self::component_task_safe(app) {
            Ok(win) => win,
            Err(error) => create_window_safe(
                WindowParams::error(error.to_string()),
                app,
                WindowId::unique(),
            )
            .map_err(|e| e.to_string())
            .expect("Failed to even create the error window: FATAL ERROR"),
        }
    }

    pub fn component_task_safe(app: &mut App<GuiState>) -> Result<Window, WindowCreationError> {
        let mut win = TasksWindow::new()?;

        win.post_create(app);

        return Ok(Window::TasksWindow(win));
    }
}

pub fn run_gui(
    folders_registry: Arc<dyn FileIndexerRegistry>,
    settings: Arc<Mutex<Settings>>,
    sender: Arc<Sender<Task>>,
    tasks: Arc<Mutex<IndexMap<u32, RunningTask>>>,
    startup_errors: Vec<String>,
    first_time: bool,
    #[cfg(feature = "daemon")] daemon_mgr: Arc<Mutex<DaemonManager>>,
    winopen_recv: Receiver<WindowParams>,
) -> AppResult {
    return WindowManager::run_app(
        WindowParams::home(startup_errors, first_time),
        move || GuiState {
            searcher: SearchDocument::new(folders_registry.clone()),
            registry: folders_registry.clone(),
            settings: settings.clone(),
            task_sender: sender.clone(),
            tasks: tasks.clone(),
            get_db: DatabaseGet::new(folders_registry.clone()),
            #[cfg(feature = "daemon")]
            daemon_mgr: daemon_mgr.clone(),
        },
        DEFAULT_ENGINE_SETTINGS.clone(),
        create_window,
        CloseMode::Minimize,
        winopen_recv,
    );
}
