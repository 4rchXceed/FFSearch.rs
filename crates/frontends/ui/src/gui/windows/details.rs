use application::indexer::traits::registry::IndexerId;
use iced_xml::app_manager::state_wrapper::App;
use iced_xml::dom::api::Dom;
use iced_xml::dom::query_builder::QueryBuilder;
#[cfg(debug_assertions)]
use iced_xml::utils::watch_css::watch_css_file;
#[cfg(debug_assertions)]
use iced_xml::window_wrapper::{CssRx, CssWatcher};
use iced_xml::window_wrapper::{Objects, ObjectsReadOnly, WindowTemplate};
use iced_xml::xml_engine::XmlEngine;
use wait::Waitable;

use crate::gui::gui_main::DEFAULT_ENGINE_SETTINGS;
use crate::gui::state::GuiState;

use super::errors::WindowCreationError;

pub struct DetailsWindow {
    qb: QueryBuilder<Self, GuiState>,
    engine: XmlEngine,
    #[cfg(debug_assertions)]
    css_rx: Option<CssRx>,
    #[cfg(debug_assertions)]
    css_watcher: Option<CssWatcher>,
}

impl WindowTemplate<Self, GuiState> for DetailsWindow {
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

impl DetailsWindow {
    pub fn new() -> Result<Self, WindowCreationError> {
        return Ok(Self {
            qb: QueryBuilder::new(),
            engine: XmlEngine::try_new(
                include_str!("details.xml").to_string(),
                DEFAULT_ENGINE_SETTINGS.clone(),
            )
            .map_err(|e| WindowCreationError::EngineCreationError(e))?,
            #[cfg(debug_assertions)]
            css_rx: None,
            #[cfg(debug_assertions)]
            css_watcher: None,
        });
    }

    pub fn post_create(&mut self, path: String, database: IndexerId, app: &mut App<GuiState>) {
        self.qb
            .b(Dom::get_element_by_id("filename").set_property("text", &path));

        let content = match app.state.get_db.get_one(database, path).wait() {
            Ok(c) => c.content.unwrap_or(String::from("No content")),
            Err(e) => format!("Error getting file content: {e}"),
        };

        self.qb
            .b(Dom::get_element_by_id("content").set_property("text", &content));

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
