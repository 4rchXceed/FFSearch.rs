use std::sync::Arc;

use application::indexer::traits::registry::IndexerId;
use domain::indexer::config::FileIndexerConfig;
use domain::searcher::document::Document;
use iced_xml::app_manager::state_wrapper::App;
use iced_xml::dom::api::Dom;
use iced_xml::dom::query_builder::QueryBuilder;
use iced_xml::dom::types::{CustomElementEvent, EventListenerTypes};
#[cfg(debug_assertions)]
use iced_xml::utils::watch_css::watch_css_file;
#[cfg(debug_assertions)]
use iced_xml::window_wrapper::CssRx;
#[cfg(debug_assertions)]
use iced_xml::window_wrapper::CssWatcher;
use iced_xml::window_wrapper::{Objects, ObjectsReadOnly, WindowTemplate};
use iced_xml::xml_engine::XmlEngine;
use infrastructure::indexer::stdfile::indexer::StdFileIndexerProvider;
use infrastructure::searcher::tantivy::database::TantivyDatabase;
use log::info;
use rfd::FileDialog;
use wait::Waitable;

#[cfg(debug_assertions)]
use crate::errors::{AppError, show_error};
use crate::gui::gui_main::{DEFAULT_ENGINE_SETTINGS, Window};
use crate::gui::params::WindowParams;
use crate::gui::state::GuiState;
use crate::manager::Task;

use super::errors::WindowCreationError;

#[cfg(feature = "daemon")]
enum DaemonUpdateType {
    Add,
    Remove,
}

enum SearchType {
    Normal,
    #[cfg(feature = "ai")]
    AI,
}

#[cfg(feature = "daemon")]
enum DaemonState {
    Started,
    Stopped,
}

pub struct HomeWindow {
    qb: QueryBuilder<Self, GuiState>,
    engine: XmlEngine,
    #[cfg(debug_assertions)]
    css_rx: Option<CssRx>,
    #[cfg(debug_assertions)]
    css_watcher: Option<CssWatcher>,
    errors: Vec<String>,
    #[cfg(feature = "daemon")]
    daemon_state: DaemonState,
}

impl WindowTemplate<Self, GuiState> for HomeWindow {
    fn get_objects(&mut self) -> Objects<'_, Self, GuiState> {
        return Objects {
            engine: &mut self.engine,
            qb: &mut self.qb,
            #[cfg(debug_assertions)]
            css_path: "styles/mainstyle.css",
            #[cfg(debug_assertions)]
            css_watcher_rx: self.css_rx.as_mut(),
        };
    }

    fn get_objects_read_only(&self) -> ObjectsReadOnly<'_, Self, GuiState> {
        return ObjectsReadOnly {
            engine: &self.engine,
            qb: &self.qb,
        };
    }

    fn get_self(&mut self) -> &mut Self {
        return self;
    }

    fn into_any(self: Box<Self>) -> Box<dyn std::any::Any> {
        return self;
    }

    #[cfg(debug_assertions)]
    fn set_css_watcher_rx(&mut self, rx: CssRx) {
        self.css_rx = Some(rx);
    }
}

impl HomeWindow {
    pub fn new(errors: Vec<String>) -> Result<Self, WindowCreationError> {
        return Ok(Self {
            qb: QueryBuilder::new(),
            engine: XmlEngine::try_new(
                include_str!("home.xml").to_string(),
                DEFAULT_ENGINE_SETTINGS.clone(),
            )
            .map_err(|e| WindowCreationError::EngineCreationError(e))?,
            #[cfg(debug_assertions)]
            css_rx: None,
            #[cfg(debug_assertions)]
            css_watcher: None,
            errors: errors,
            #[cfg(feature = "daemon")]
            daemon_state: DaemonState::Stopped,
        });
    }

    pub fn get_u32(&mut self, id: &str, field: &str) -> Option<u32> {
        self.qb.b(Dom::get_element_by_id(id).get_property(field));

        let last = self.process();

        if let Some(string) = last.data_str {
            return string.parse().ok();
        } else {
            return None;
        }
    }

    pub fn get_str(&mut self, id: &str, field: &str) -> Option<String> {
        self.qb.b(Dom::get_element_by_id(id).get_property(field));

        let last = self.process();

        return last.data_str;
    }

    pub fn set_search_status(&mut self, status: &str) {
        self.qb
            .b(Dom::get_element_by_id("search-status").set_property("text", status));

        self.process();
    }

    pub fn set_add_folder_status(&mut self, status: &str) {
        self.qb
            .b(Dom::get_element_by_id("add-folder-status").set_property("text", status));

        self.process();
    }

    pub fn set_actions_status(&mut self, status: &str) {
        self.qb
            .b(Dom::get_element_by_id("actions-status").set_property("text", status));

        self.process();
    }

    #[cfg(feature = "daemon")]
    pub fn set_daemon_status(&mut self, status: &str) {
        self.qb
            .b(Dom::get_element_by_id("actions-status").set_property("text", status));

        self.process();
    }

    #[cfg(feature = "mcp")]
    pub fn set_mcp_status(&mut self, status: &str) {
        self.qb
            .b(Dom::get_element_by_id("mcp-status").set_property("text", status));

        self.process();
    }

    pub fn reload_select(&mut self, app: &mut App<GuiState>) {
        self.qb.b(Dom::get_elements_by_class("folders-list")
            .fire_event(CustomElementEvent::ClearSelectOptions()));

        let indexers = app.state.registry.list_indexers().wait();

        for (id, path) in indexers {
            self.qb.b(Dom::get_elements_by_class("folders-list")
                .fire_event(CustomElementEvent::AddSelectOption(id.to_string(), path)));
        }

        self.process();
    }

    pub fn add_folder(&mut self, app: &mut App<GuiState>) {
        let dialog = FileDialog::new();

        if let Some(folder) = dialog.pick_folder()
            && let Ok(mut settings) = app.state.settings.lock()
        {
            let config = FileIndexerConfig::new();

            let path = folder.to_string_lossy().to_string();

            let database_res = match TantivyDatabase::new(path.clone(), config.db_config.clone()) {
                Ok(db) => db,
                Err(e) => {
                    self.set_add_folder_status(&format!("Failed to add the folder: {e}"));
                    return;
                }
            };

            let database = Arc::new(database_res);
            let indexer = Arc::new(StdFileIndexerProvider::new(config.clone()));

            app.state
                .registry
                .add(indexer, database, path.clone())
                .wait();

            settings.file_indexers.insert(path, config);

            if let Err(e) = settings.save() {
                self.set_add_folder_status(&format!("Failed to save the new settings: {e}"));
            } else {
                self.set_add_folder_status("Folder added successfully");
            }
        }

        self.reload_select(app);
    }

    pub fn finish_search(
        &mut self,
        search_result: Vec<Document>,
        query: String,
        app: &mut App<GuiState>,
        indexer: IndexerId,
    ) {
        info!("Search results: {search_result:?}");

        self.qb.open_window(
            Box::new(WindowParams::results(search_result, query, indexer)),
            app,
        );

        self.process();

        self.set_search_status("");
    }

    fn search(&mut self, app: &mut App<GuiState>, search_type: SearchType) {
        if let Some(id) = self.get_u32("folder-select", "value") {
            if let Some(limit) = self.get_u32("search-limit", "value") {
                if let Some(query) = self.get_str("search-query", "value") {
                    match search_type {
                        SearchType::Normal => {
                            self.normal_search(app, query, limit, id);
                        }
                        #[cfg(feature = "ai")]
                        SearchType::AI => {
                            self.ai_search(app, query, id);
                        }
                    }
                } else {
                    self.set_search_status("Invalid text input");
                }
            } else {
                self.set_search_status("Invalid limit, please enter a positivve number");
            }
        } else {
            self.set_search_status("Please select a folder");
        }
    }

    #[cfg(feature = "ai")]
    fn ai_search(&mut self, app: &mut App<GuiState>, query: String, id: u32) {
        let agent_result = app.state.registry.get_agent(id).wait();

        if let Some(agent_op) = agent_result
            && let Some(_) = agent_op
        {
            self.qb
                .open_window(Box::new(WindowParams::ai_chat(query, id)), app);

            self.process();

            self.set_search_status("");
        } else {
            self.set_search_status("Please first click 'Enable AI' for this folder");
        }
    }

    fn normal_search(&mut self, app: &mut App<GuiState>, query: String, limit: u32, id: u32) {
        // We can use wait here, since it doesn't take more than a few ms
        let result = app
            .state
            .searcher
            .search(id, query.clone(), limit as usize)
            .wait();

        match result {
            Ok(res) => self.finish_search(res, query, app, id),
            Err(e) => {
                self.set_search_status(&format!("Error while searching: {}", e.to_string()));
            }
        }
    }

    #[cfg(feature = "daemon")]
    fn update_from_daemon(&mut self, app: &mut App<GuiState>, update_type: DaemonUpdateType) {
        if let Some(id) = self.get_u32("actions-select", "value") {
            if let Ok(mut daemon_manager) = app.state.daemon_mgr.lock() {
                if let Some(folder) = app.state.registry.get_folder(id).wait() {
                    let result = match update_type {
                        DaemonUpdateType::Add => daemon_manager.add_folder(folder).wait(),
                        DaemonUpdateType::Remove => daemon_manager.remove_folder(folder).wait(),
                    };

                    match result {
                        Ok(_) => {
                            self.set_actions_status("Daemon modified successfully!");
                        }
                        Err(e) => {
                            self.set_actions_status(&format!("Could not update the daemon: {e}"));
                        }
                    }
                }
            }
        } else {
            self.set_actions_status("Please select a folder first");
        }
    }

    #[cfg(feature = "daemon")]
    pub fn toggle_daemon_state(&mut self, app: &mut App<GuiState>) {
        if let Ok(mut daemon_mgr) = app.state.daemon_mgr.lock() {
            match self.daemon_state {
                DaemonState::Started => {
                    let result = daemon_mgr.stop_daemon().wait();

                    match result {
                        Ok(_) => {
                            self.daemon_state = DaemonState::Stopped;

                            self.qb.b(Dom::get_element_by_id("sync-daemon-btn")
                                .set_property("text", "Start Daemon"));

                            self.process();
                        }
                        Err(_) => {
                            self.set_daemon_status("Error stopping daemon!");
                        }
                    }
                }
                DaemonState::Stopped => {
                    daemon_mgr.start();

                    self.daemon_state = DaemonState::Started;

                    self.qb.b(Dom::get_element_by_id("sync-daemon-btn")
                        .set_property("text", "Stop Daemon"));

                    self.process();
                }
            }
        }
    }

    pub fn post_create(
        &mut self,
        app: &mut App<GuiState>,
        first_opening: bool,
    ) -> Result<(), WindowCreationError> {
        if first_opening {
            self.qb.open_window(Box::new(WindowParams::startup()), app);
        }

        #[cfg(debug_assertions)]
        match watch_css_file(self, 1000) {
            Err(e) => {
                show_error(AppError::LazyError(e.to_string()));
            }
            Ok(w) => self.css_watcher = Some(w),
        };

        #[cfg(not(debug_assertions))]
        self.qb.import_css(crate::gui::css::CSS.to_string(), false);

        self.qb
            .b(Dom::get_element_by_id("search-btn").add_event_listener(EventListenerTypes::Click))
            .with_callback(|this, _, app| {
                this.search(app, SearchType::Normal);
            });

        #[cfg(not(feature = "ai"))]
        {
            self.qb.b(Dom::get_element_by_id("prompt-btn").remove());
            self.process();
        }
        #[cfg(feature = "ai")]
        {
            self.qb
                .b(Dom::get_element_by_id("prompt-btn")
                    .add_event_listener(EventListenerTypes::Click))
                .with_callback(|this, _, app| {
                    this.search(app, SearchType::AI);
                });
            self.process();
        }

        self.qb
            .b(Dom::get_element_by_id("add-folder-btn")
                .add_event_listener(EventListenerTypes::Click))
            .with_callback(|this, _, app| {
                this.add_folder(app);
            });

        self.qb
            .b(Dom::get_element_by_id("sync-btn").add_event_listener(EventListenerTypes::Click))
            .with_callback(|this, _, app| {
                if let Some(id) = this.get_u32("actions-select", "value") {
                    if let Err(e) = app.state.task_sender.send(Task::SyncFolder(id)) {
                        this.set_actions_status(&format!(
                            "Error while sending task to background process: {}",
                            e
                        ));
                    }
                } else {
                    this.set_actions_status("Please select a folder first");
                }
            });

        #[cfg(feature = "ai")]
        self.qb
            .b(Dom::get_element_by_id("enable-ai-btn")
                .add_event_listener(EventListenerTypes::Click))
            .with_callback(|this, _, app| {
                if let Some(id) = this.get_u32("actions-select", "value") {
                    if let Err(e) = app.state.task_sender.send(Task::AssignAI(id)) {
                        this.set_actions_status(&format!(
                            "Error while sending task to background process: {}",
                            e
                        ));
                    }
                } else {
                    this.set_actions_status("Please select a folder first");
                }
            });

        #[cfg(not(feature = "ai"))]
        {
            self.qb.b(Dom::get_element_by_id("enable-ai-btn").remove());
            self.process();
        }

        #[cfg(not(feature = "daemon"))]
        {
            self.qb
                .b(Dom::get_element_by_id("daemon-controls").remove());

            self.qb.b(Dom::get_element_by_id("daemon-sync").remove());

            self.process();
        }

        #[cfg(not(feature = "mcp"))]
        {
            self.qb.b(Dom::get_element_by_id("mcp-container").remove());

            self.process();
        }

        #[cfg(feature = "mcp")]
        {
            self.qb
                .b(Dom::get_element_by_id("start-mcp")
                    .add_event_listener(EventListenerTypes::Click))
                .with_callback(|this, _, app| {
                    if let Err(_) = app.state.task_sender.send(Task::StartMCP) {
                        this.set_mcp_status("Connection to Task Manager broken");
                    } else {
                        this.set_mcp_status("MCP Server started, a task should be created");
                    }
                });
        }

        #[cfg(feature = "daemon")]
        {
            self.qb
                .b(Dom::get_element_by_id("add-to-daemon")
                    .add_event_listener(EventListenerTypes::Click))
                .with_callback(|this, _, app| {
                    this.update_from_daemon(app, DaemonUpdateType::Add);
                });

            self.qb
                .b(Dom::get_element_by_id("remove-from-daemon")
                    .add_event_listener(EventListenerTypes::Click))
                .with_callback(|this, _, app| {
                    this.update_from_daemon(app, DaemonUpdateType::Remove);
                });

            self.qb
                .b(Dom::get_element_by_id("sync-daemon-btn")
                    .add_event_listener(EventListenerTypes::Click))
                .with_callback(|this, _, app| {
                    this.toggle_daemon_state(app);
                });

            self.process();
        }

        self.qb
            .b(Dom::get_element_by_id("delete-btn").add_event_listener(EventListenerTypes::Click))
            .with_callback(|this, _, app| {
                if let Some(id) = this.get_u32("actions-select", "value") {
                    if let Ok(mut settings) = app.state.settings.lock()
                        && let Some(folder) = &app.state.registry.get_folder(id).wait()
                        && settings.file_indexers.contains_key(folder)
                    {
                        settings.file_indexers.remove(folder);
                        if let Err(e) = settings.save() {
                            this.set_actions_status(&format!(
                                "Failed to save the new settings: {e}"
                            ));
                        }
                    }

                    if let Err(_) = app.state.registry.remove(id).wait() {
                        this.set_actions_status("Folder doesn't exists");
                    }

                    this.reload_select(app);
                } else {
                    this.set_actions_status("Please select a folder first");
                }
            });

        let errors_text = if self.errors.is_empty() {
            "App startup complete! No error reported."
        } else {
            &format!(
                "Some erros were found while starting the app:\n{}",
                self.errors.join("\n")
            )
        };

        self.qb
            .b(Dom::get_element_by_id("startup-errors").set_property("text", errors_text));

        self.reload_select(app);

        let error = self.add_component(
            Dom::get_element_by_id("tasks"),
            Box::new(Window::component_task(app)),
        );

        if let Err(e) = error {
            self.qb
                .b(Dom::get_element_by_id("tasks").set_property("text", &format!("Error: {e}")));
        }

        self.qb
            .b(Dom::get_element_by_id("open-settings")
                .add_event_listener(EventListenerTypes::Click))
            .with_callback(|this, _, app| {
                this.qb.open_window(Box::new(WindowParams::settings()), app);
                this.process();
            });

        self.process();

        return Ok(());
    }
}
