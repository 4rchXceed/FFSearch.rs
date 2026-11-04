use domain::indexer::config::FileIndexerConfig;
use iced_xml::app_manager::state_wrapper::App;
use iced_xml::dom::api::Dom;
use iced_xml::dom::query::DomQuery;
use iced_xml::dom::query_builder::QueryBuilder;
use iced_xml::dom::types::{EventListenerTypes, EventResponse};
#[cfg(debug_assertions)]
use iced_xml::utils::watch_css::watch_css_file;
#[cfg(debug_assertions)]
use iced_xml::window_wrapper::{CssRx, CssWatcher};
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
    #[cfg(debug_assertions)]
    css_watcher: Option<CssWatcher>,
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
            #[cfg(debug_assertions)]
            css_watcher: None,
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

        container.append_child(input);

        return container;
    }

    pub fn single_folder_settings(path: String, cfg: FileIndexerConfig) -> XmlElement {
        let mut container = XmlElement::void();
        container.set_tag("Col");
        container.add_class("single-settings");
        container.append_child(text(&format!("Settings for {path}")));

        let mut indexer_settings = XmlElement::void();
        indexer_settings.set_tag("Col");
        indexer_settings.add_class("subsettings");

        indexer_settings.append_child(text("Indexer settings"));
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

    #[allow(unused)]
    pub fn get_value_from_id(&mut self, id: &str) -> Option<String> {
        self.qb
            .b(&mut Dom::get_element_by_id(id).get_property("value"));

        let datas = self.process();

        return datas.data_str;
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

        #[cfg(feature = "mcp")]
        {
            // ---- MCP Settings ----
            // Listen addr
            if let Some(listen_addr) = self.get_value_from_id("mcp-listen-addr-settings") {
                if let Ok(mut settings) = app.state.settings.lock() {
                    settings.mcp.listen_addr = listen_addr;
                }
            }
        }

        #[cfg(feature = "ai")]
        {
            // ---- AI Settings ----
            // LLM Provider
            if let Some(llm_provider) = self.get_value_from_id("llm-provider-settings") {
                if let Ok(mut settings) = app.state.settings.lock() {
                    settings.ai.provider = llm_provider;
                }
            }

            // API Key
            if let Some(api_key) = self.get_value_from_id("api-key-settings") {
                if let Ok(mut settings) = app.state.settings.lock() {
                    settings.ai.api_key = api_key;
                }
            }

            // Chat Model
            if let Some(ai_model) = self.get_value_from_id("ai-model-settings") {
                if let Ok(mut settings) = app.state.settings.lock() {
                    settings.ai.model = ai_model;
                }
            }

            // Allow thinking checkbox
            self.qb
                .b(Dom::get_element_by_id("thinking-settings").get_property("checked"));

            let result = self.process();

            if let Some(thinking_enabled) = result.data_bool {
                if let Ok(mut settings) = app.state.settings.lock() {
                    settings.ai.think = thinking_enabled;
                }
            }

            // System prompt
            if let Some(sys_prompt) = self.get_value_from_id("system-prompt-settings") {
                if let Ok(mut settings) = app.state.settings.lock() {
                    settings.ai.preamble = sys_prompt.replace("\n", " ");
                }
            }

            // ---- RAG Settings ----
            // Embedding Model
            if let Some(embedding_model) = self.get_value_from_id("rag-model-settings") {
                if let Ok(mut settings) = app.state.settings.lock() {
                    settings.ai.rag_config.embedding_model = embedding_model;
                }
            }

            // Top results
            if let Some(top_results_str) = self.get_value_from_id("rag-top-results-settings") {
                if let Ok(top_results) = top_results_str.parse::<usize>() {
                    if let Ok(mut settings) = app.state.settings.lock() {
                        settings.ai.rag_config.top_results = top_results;
                    }
                } else {
                    self.add_status(&format!("Not a number (top results = '{top_results_str}')"));
                }
            }

            // Max batch size
            if let Some(batch_size_str) = self.get_value_from_id("batch-size-settings") {
                if let Ok(batch_size) = batch_size_str.parse::<usize>() {
                    if let Ok(mut settings) = app.state.settings.lock() {
                        settings.ai.rag_config.max_batch_size = batch_size;
                    }
                } else {
                    self.add_status(&format!("Not a number (batch size = '{batch_size_str}')"));
                }
            }
        }

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

    #[allow(unused)]
    pub fn set_val(&mut self, id: &str, val: String) {
        self.qb
            .b(Dom::get_element_by_id(id).set_property("value", &val));

        self.process();
    }

    pub fn post_create(&mut self, app: &mut App<GuiState>) {
        let mut main = XmlElement::void();
        main.set_tag("Col");

        if let Ok(settings) = app.state.settings.lock() {
            for (path, indexer_cfg) in &settings.file_indexers {
                main.append_child(Self::single_folder_settings(
                    path.clone(),
                    indexer_cfg.clone(),
                ));
            }

            #[cfg(feature = "ai")]
            {
                self.set_val("llm-provider-settings", settings.ai.provider.clone());
                self.set_val("api-key-settings", settings.ai.api_key.clone());
                self.set_val("ai-model-settings", settings.ai.model.clone());
                self.set_val(
                    "thinking-settings",
                    if settings.ai.think {
                        String::from("true")
                    } else {
                        String::from("false")
                    },
                );
                self.set_val("system-prompt-settings", settings.ai.preamble.clone());
                self.set_val(
                    "rag-model-settings",
                    settings.ai.rag_config.embedding_model.clone(),
                );
                self.set_val(
                    "rag-top-results-settings",
                    settings.ai.rag_config.top_results.to_string(),
                );
                self.set_val(
                    "batch-size-settings",
                    settings.ai.rag_config.max_batch_size.to_string(),
                );
            }

            #[cfg(not(feature = "ai"))]
            {
                self.qb.b(Dom::get_element_by_id("ai-settings").remove());
                self.process();
            }

            #[cfg(not(feature = "mcp"))]
            {
                self.qb.b(Dom::get_element_by_id("mcp-settings").remove());

                self.process();
            }

            #[cfg(feature = "mcp")]
            {
                self.set_val("mcp-listen-addr-settings", settings.mcp.listen_addr.clone());
            }
        }

        self.qb
            .b(Dom::get_element_by_id("folders-settings").replace(Box::new(main)));

        self.qb
            .b(Dom::get_element_by_id("save").add_event_listener(EventListenerTypes::Click))
            .with_callback(Self::save);

        self.process();

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
