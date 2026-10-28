use std::sync::Arc;

use application::indexer::traits::registry::IndexerId;
use domain::indexer::config::FileIndexerConfig;
use domain::searcher::document::Document;
use iced_xml::app_manager::state_wrapper::App;
use iced_xml::dom::api::Dom;
use iced_xml::dom::events::EventListenerTypes;
use iced_xml::dom::query_builder::{CustomElementEvent, QueryBuilder};
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

pub struct HomeWindow {
    qb: QueryBuilder<Self, GuiState>,
    engine: XmlEngine,
    #[cfg(debug_assertions)]
    css_rx: Option<CssRx>,
    #[cfg(debug_assertions)]
    css_watcher: Option<CssWatcher>,
    errors: Vec<String>,
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
    }

    pub fn search(&mut self, app: &mut App<GuiState>) {
        if let Some(id) = self.get_u32("folder-select", "value") {
            if let Some(limit) = self.get_u32("search-limit", "value") {
                if let Some(query) = self.get_str("search-query", "value") {
                    // We can use wait here, since it doesn't take more than a few ms
                    let result = app
                        .state
                        .searcher
                        .search(id, query.clone(), limit as usize)
                        .wait();

                    match result {
                        Ok(res) => self.finish_search(res, query, app, id),
                        Err(e) => {
                            self.set_search_status(&format!(
                                "Error while searching: {}",
                                e.to_string()
                            ));
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

        self.qb
            .b(Dom::get_element_by_id("search-btn").add_event_listener(EventListenerTypes::Click))
            .with_callback(|this, _, app| {
                this.search(app);
            });

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
            ""
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
