use std::path::{Path, PathBuf};

use application::indexer::traits::registry::IndexerId;
use chrono::DateTime;
use domain::searcher::document::Document;
use iced_xml::app_manager::state_wrapper::App;
use iced_xml::dom::api::Dom;
use iced_xml::dom::query_builder::QueryBuilder;
use iced_xml::dom::types::EventListenerTypes;
#[cfg(debug_assertions)]
use iced_xml::utils::watch_css::watch_css_file;
#[cfg(debug_assertions)]
use iced_xml::window_wrapper::{CssRx, CssWatcher};
use iced_xml::window_wrapper::{Objects, ObjectsReadOnly, WindowTemplate};
use iced_xml::xml_engine::XmlEngine;
use iced_xml::xml_struct::xml_element::XmlElement;
use log::{error, info};
use wait::Waitable;

use crate::gui::gui_main::{DEFAULT_ENGINE_SETTINGS, text};
use crate::gui::params::WindowParams;
use crate::gui::state::GuiState;

use super::errors::WindowCreationError;

pub struct ResultsWindow {
    qb: QueryBuilder<Self, GuiState>,
    engine: XmlEngine,
    #[cfg(debug_assertions)]
    css_rx: Option<CssRx>,
    #[cfg(debug_assertions)]
    css_watcher: Option<CssWatcher>,
    current_indexer: IndexerId,
}

impl WindowTemplate<Self, GuiState> for ResultsWindow {
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

impl ResultsWindow {
    pub fn new(current_indexer: IndexerId) -> Result<Self, WindowCreationError> {
        return Ok(Self {
            qb: QueryBuilder::new(),
            engine: XmlEngine::try_new(
                include_str!("results.xml").to_string(),
                DEFAULT_ENGINE_SETTINGS.clone(),
            )
            .map_err(|e| WindowCreationError::EngineCreationError(e))?,
            #[cfg(debug_assertions)]
            css_rx: None,
            #[cfg(debug_assertions)]
            css_watcher: None,
            current_indexer: current_indexer,
        });
    }

    pub fn build_result(&mut self, document: &Document, dir: &PathBuf) -> XmlElement {
        let mut parent = XmlElement::void();
        parent.set_tag("Row");

        let time = DateTime::from_timestamp(document.modification_date, 0);

        let date_str = if let Some(date) = time {
            &date.format("%Y-%m-%d %H:%M").to_string()
        } else {
            "Invalid date"
        };

        parent.append_child(text(&format!(
            "{}: last updated: {}",
            document.path, date_str
        )));

        let real_path = dir.join(Path::new(&document.path));

        let mut open_folder_button = XmlElement::void();
        open_folder_button.set_tag("Button");
        open_folder_button.add_data("realpath", &real_path.to_string_lossy().to_string());
        open_folder_button.add_class("openfolder-btn");
        open_folder_button.set_text("Open folder");

        parent.append_child(open_folder_button);

        let mut open_button = XmlElement::void();
        open_button.set_tag("Button");
        open_button.add_data("realpath", &real_path.to_string_lossy().to_string());
        open_button.add_class("open-btn");
        open_button.set_text("Open");

        parent.append_child(open_button);

        let mut button = XmlElement::void();
        button.set_tag("Button");
        button.add_data("path", &document.path);
        button.add_class("result-btn");
        button.set_text("Details");

        parent.append_child(button);

        return parent;
    }

    pub fn update_results(
        &mut self,
        new_results: Vec<Document>,
        query: String,
        app: &mut App<GuiState>,
    ) {
        let path_str = app
            .state
            .registry
            .get_folder(self.current_indexer)
            .wait()
            .unwrap_or_default();

        let path = Path::new(&path_str).to_path_buf();

        let mut results_xml_list = XmlElement::void();
        results_xml_list.set_tag("Col");
        results_xml_list.set_id("results");

        for result in &new_results {
            results_xml_list.append_child(self.build_result(result, &path));
        }

        if new_results.is_empty() {
            results_xml_list = text(&format!("No results for: {}", query));
        }

        self.qb
            .b(Dom::get_element_by_id("results").replace(Box::new(results_xml_list)));

        self.qb
            .b(Dom::get_elements_by_class("result-btn")
                .add_event_listener(EventListenerTypes::Click))
            .with_callback(|this, er, app| {
                if let Some(query) = er.target {
                    this.qb.b(Dom::from(query).get_data("path"));

                    let result = this.process();

                    if let Some(path) = result.data_str {
                        info!("Opening details for: {path}");
                        this.qb.open_window(
                            Box::new(WindowParams::details(path, this.current_indexer)),
                            app,
                        );
                    } else {
                        error!("GetData for element failed!");
                    }
                } else {
                    error!("EventResponse should have a target, but doesn't have one");
                }
            });

        self.qb
            .b(
                Dom::get_elements_by_class("open-btn")
                    .add_event_listener(EventListenerTypes::Click),
            )
            .with_callback(|this, er, _| {
                if let Some(query) = er.target {
                    this.qb.b(Dom::from(query).get_data("realpath"));

                    let result = this.process();

                    if let Some(path) = result.data_str {
                        info!("Opening: {path}");

                        match open::that(path) {
                            Ok(_) => {}
                            Err(e) => error!("Error opening file: {e}"),
                        }
                    } else {
                        error!("GetData for element failed!");
                    }
                } else {
                    error!("EventResponse should have a target, but doesn't have one");
                }
            });

        self.qb
            .b(Dom::get_elements_by_class("openfolder-btn")
                .add_event_listener(EventListenerTypes::Click))
            .with_callback(|this, er, _| {
                if let Some(query) = er.target {
                    this.qb.b(Dom::from(query).get_data("realpath"));

                    let result = this.process();

                    if let Some(path) = result.data_str {
                        let path = Path::new(&path)
                            .parent()
                            .unwrap_or(Path::new(&path))
                            .to_string_lossy()
                            .to_string();

                        info!("Opening: {path}");

                        match open::that(path) {
                            Ok(_) => {}
                            Err(e) => error!("Error opening file: {e}"),
                        }
                    } else {
                        error!("GetData for element failed!");
                    }
                } else {
                    error!("EventResponse should have a target, but doesn't have one");
                }
            });

        self.process();
    }

    pub fn post_create(&mut self, results: Vec<Document>, query: String, app: &mut App<GuiState>) {
        self.qb.b(Dom::get_element_by_id("results-label")
            .set_property("text", &format!("Results for: {query}")));

        self.process();

        self.update_results(results, query, app);

        #[cfg(debug_assertions)]
        match watch_css_file(self, 1000) {
            Err(e) => {
                crate::errors::show_error(crate::errors::AppError::LazyError(e.to_string()));
            }
            Ok(w) => self.css_watcher = Some(w),
        };

        #[cfg(not(debug_assertions))]
        self.qb.import_css(crate::gui::css::CSS.to_string(), false);

        self.process();
    }
}
