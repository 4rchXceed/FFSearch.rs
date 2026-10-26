use std::any::Any;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};

use application::indexer::traits::registry::FileIndexerRegistry;
use application::searcher::actions::search::SearchDocument;
use iced_xml::app_manager::app_manager::WindowManager;
use iced_xml::app_manager::state_wrapper::App;
use iced_xml::app_manager::utils::WindowId;
use iced_xml::window_manager;
use iced_xml::window_wrapper::AppResult;
use iced_xml::xml_struct::xml_element::XmlElement;
use settings::settings::Settings;

use crate::manager::{RunningTask, Task};

use super::params::{WindowParams, WindowType};
use super::state::GuiState;
use super::windows::errors::WindowCreationError;
use super::windows::home::HomeWindow;
use super::windows::settings::SettingsWindow;
use super::windows::tasks::TasksWindow;
use super::windows::winerror::ErrorWindow;

fn create_window_safe(
    params: WindowParams,
    app: &mut App<GuiState>,
    _win_id: WindowId,
) -> Result<Window, WindowCreationError> {
    let window = match params.wintype {
        WindowType::HomeWindow => {
            let mut win = HomeWindow::new(params.startup_errors)?;

            win.post_create(app)?;

            Window::HomeWindow(win)
        }
        WindowType::ErrorWindow(error) => {
            let mut win = ErrorWindow::new()?;

            win.post_create(error);

            Window::ErrorWindow(win)
        }
        WindowType::TasksWindow => {
            let mut win = TasksWindow::new()?;

            win.post_create(app);

            Window::TasksWindow(win)
        }
        WindowType::SettingsWindow => {
            let mut win = SettingsWindow::new()?;

            win.post_create(app);

            Window::SettingsWindow(win)
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

window_manager!(Window {
    HomeWindow,
    ErrorWindow,
    TasksWindow,
    SettingsWindow
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
    tasks: Arc<Mutex<Vec<RunningTask>>>,
    startup_errors: Vec<String>,
) -> AppResult {
    return WindowManager::run_app(
        WindowParams::home(startup_errors),
        move || GuiState {
            searcher: SearchDocument::new(folders_registry.clone()),
            registry: folders_registry.clone(),
            settings: settings.clone(),
            task_sender: sender.clone(),
            tasks: tasks.clone(),
        },
        DEFAULT_ENGINE_SETTINGS.clone(),
        create_window,
    );
}
