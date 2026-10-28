use domain::indexer::config::FileIndexerConfig;
use iced_xml::app_manager::state_wrapper::App;
use iced_xml::dom::api::Dom;
use iced_xml::dom::events::EventListenerTypes;
use iced_xml::dom::query::DomQuery;
use iced_xml::dom::query_builder::{EventResponse, QueryBuilder};
#[cfg(debug_assertions)]
use iced_xml::window_wrapper::CssRx;
use iced_xml::window_wrapper::{Objects, ObjectsReadOnly, WindowTemplate};
use iced_xml::xml_engine::XmlEngine;
use iced_xml::xml_struct::xml_element::XmlElement;
use log::info;

use crate::gui::gui_main::{DEFAULT_ENGINE_SETTINGS, text};
use crate::gui::state::GuiState;

use super::errors::WindowCreationError;

pub struct SettingsWindow {
    qb: QueryBuilder<Self, GuiState>,
    engine: XmlEngine,
    #[cfg(debug_assertions)]
    css_rx: Option<CssRx>,
}

impl WindowTemplate<Self, GuiState> for SettingsWindow {
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

impl SettingsWindow {
    pub fn new() -> Result<Self, WindowCreationError> {
        return Ok(Self {
            qb: QueryBuilder::new(),
            engine: XmlEngine::try_new(
                include_str!("settings.xml").to_string(),
                DEFAULT_ENGINE_SETTINGS.clone(),
            )
            .map_err(|e| WindowCreationError::EngineCreationError(e))?,
            #[cfg(debug_assertions)]
            css_rx: None,
        });
    }

    pub fn input(
        placeholder: &str,
        value: String,
        class: &str,
        path: String,
        input_id: &str,
        type_class: &str,
    ) -> XmlElement {
        let mut container = XmlElement::void();
        container.set_tag("Row");

        container.append_child(text(placeholder));

        let mut input = XmlElement::void();
        input.set_tag("Input");
        input
            .attributes
            .insert(String::from("placeholder"), placeholder.to_string());
        input.set_text(&value);
        input.add_class(class);
        input.add_data("folder-id", &path);
        input.add_data("for", input_id);
        input.add_class(type_class);
        println!("{input:?}");

        container.append_child(input);

        return container;
    }

    pub fn single_folder_settings(path: String, cfg: FileIndexerConfig) -> XmlElement {
        let mut container = XmlElement::void();
        container.set_tag("Col");
        container.append_child(text(&format!("Settings for {path}")));

        let mut indexer_settings = XmlElement::void();
        indexer_settings.set_tag("Col");

        indexer_settings.append_child(text("Indexer settngs"));
        indexer_settings.append_child(Self::input(
            "Number of concurrent processes",
            cfg.number_of_threads.to_string(),
            "input-threads",
            path.clone(),
            "threads",
            "dbinput-number",
        ));
        // TODO: Single file size is global, and not yet implemented
        container.append_child(indexer_settings);

        let mut database_settings = XmlElement::void();
        database_settings.set_tag("Col");

        database_settings.append_child(text("Database settings"));
        database_settings.append_child(Self::input(
            "Maximum size of the database (bytes)",
            cfg.db_config.max_db_size.to_string(),
            "input-maxdb-size",
            path.clone(),
            "max-db-size",
            "dbinput-number",
        ));
        database_settings.append_child(Self::input(
            "Database's index folder prefix",
            cfg.db_config.database_folder_prefix,
            "input-db-prefix",
            path.clone(),
            "db-prefix",
            "dbinput-text",
        ));
        container.append_child(database_settings);

        return container;
    }

    pub fn get_folder_id(&mut self, selector: Option<DomQuery>) -> Option<String> {
        if let Some(sel) = selector {
            self.qb.b(&mut Dom::from(sel).get_data("folder-id"));

            let datas = self.process();

            return datas.data_str;
        }

        return None;
    }

    pub fn get_input_id(&mut self, selector: Option<DomQuery>) -> Option<String> {
        if let Some(sel) = selector {
            self.qb.b(&mut Dom::from(sel).get_data("for"));

            let datas = self.process();

            return datas.data_str;
        }

        return None;
    }

    pub fn get_value(&mut self, selector: Option<DomQuery>) -> Option<String> {
        if let Some(sel) = selector {
            self.qb.b(&mut Dom::from(sel).get_property("value"));

            let datas = self.process();

            return datas.data_str;
        }

        return None;
    }

    pub fn add_status(&mut self, status: &str) {
        info!("New status: {status}");

        self.qb
            .b(Dom::get_element_by_id("status").get_property("text"));

        let datas = self.process();

        let current_status = datas.data_str.unwrap_or_default();

        let new_status = format!("{current_status}\n{status}");

        self.qb
            .b(Dom::get_element_by_id("status").set_property("text", &new_status));

        self.process();
    }

    pub fn save(&mut self, _: EventResponse, app: &mut App<GuiState>) {
        self.qb
            .b(Dom::get_element_by_id("status").set_property("text", ""));

        self.process();

        // .then: Small hack for now, else the process_context doesn't work
        self.qb
            .b(Dom::get_elements_by_class("dbinput-number").get_property("value"))
            .then(|_, _| ());

        self.process_context(
            |this, qr, context| {
                if let Some(folder_id) = this.get_folder_id(qr.data_selector.clone())
                    && let Some(value) = this.get_value(qr.data_selector.clone())
                    && let Some(input_id) = this.get_input_id(qr.data_selector)
                {
                    if let Ok(mut settings) = context.state.settings.lock() {
                        if let Ok(threads) = value.parse::<usize>() {
                            if let Some(indexer_cfg) = settings.file_indexers.get_mut(&folder_id) {
                                match input_id.as_str() {
                                    "threads" => {
                                        indexer_cfg.number_of_threads = threads;
                                    }
                                    "max-db-size" => {
                                        indexer_cfg.db_config.max_db_size = threads;
                                    }
                                    _ => {}
                                }
                            } else {
                                this.add_status(&format!(
                                    "Folder not found in settings: {folder_id}"
                                ));
                            }
                        } else {
                            this.add_status(&format!("Not a number (threads = '{value}')"));
                        }
                    } else {
                        this.add_status("Failed to lock settings");
                    }
                } else {
                    this.add_status("Failed to get folder id or value");
                }
            },
            app,
        );

        self.qb
            .b(Dom::get_elements_by_class("dbinput-text").get_property("value"));

        self.process_context(
            |this, qr, context| {
                if let Some(folder_id) = this.get_folder_id(qr.data_selector.clone())
                    && let Some(value) = this.get_value(qr.data_selector.clone())
                    && let Some(input_id) = this.get_input_id(qr.data_selector)
                {
                    if let Ok(mut settings) = context.state.settings.lock() {
                        if let Some(indexer_cfg) = settings.file_indexers.get_mut(&folder_id) {
                            match input_id.as_str() {
                                "db-prefix" => {
                                    indexer_cfg.db_config.database_folder_prefix = value.clone();
                                }
                                _ => {}
                            }
                        } else {
                            this.add_status(&format!(
                                "Folder id not found in settings: {folder_id}"
                            ));
                        }
                    } else {
                        this.add_status("Failed to lock settings");
                    }
                } else {
                    this.add_status("Failed to get folder id or value");
                }
            },
            app,
        );

        if let Ok(settings) = app.state.settings.lock() {
            if let Err(e) = settings.save() {
                self.add_status(&format!("Failed to save settings: {e}"));
            } else {
                info!("Settings saved successfully");
                self.add_status("Settings saved. Restart the app for changes to take effect.");
            }
        } else {
            self.add_status("Failed to lock settings");
        }
    }

    pub fn post_create(&mut self, app: &mut App<GuiState>) {
        let mut main = XmlElement::void();
        main.set_tag("Col");
        main.append_child(text("All these settings require restarting the app!"));

        if let Ok(settings) = app.state.settings.lock() {
            for (path, indexer_cfg) in &settings.file_indexers {
                main.append_child(Self::single_folder_settings(
                    path.clone(),
                    indexer_cfg.clone(),
                ));
            }
        }

        self.qb
            .b(Dom::get_element_by_id("folders-settings").replace(Box::new(main)));

        self.qb
            .b(Dom::get_element_by_id("save").add_event_listener(EventListenerTypes::Click))
            .with_callback(Self::save);

        self.process();
    }
}
