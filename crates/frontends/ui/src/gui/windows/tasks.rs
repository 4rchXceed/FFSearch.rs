use iced_xml::app_manager::state_wrapper::App;
use iced_xml::dom::api::Dom;
use iced_xml::dom::query_builder::QueryBuilder;
#[cfg(debug_assertions)]
use iced_xml::utils::watch_css::watch_css_file;
#[cfg(debug_assertions)]
use iced_xml::window_wrapper::{CssRx, CssWatcher};
use iced_xml::window_wrapper::{Objects, ObjectsReadOnly, WindowTemplate};
use iced_xml::xml_engine::XmlEngine;
use iced_xml::xml_struct::xml_element::XmlElement;

use crate::gui::gui_main::DEFAULT_ENGINE_SETTINGS;
use crate::gui::state::GuiState;

use super::errors::WindowCreationError;

pub struct TasksWindow {
    qb: QueryBuilder<Self, GuiState>,
    engine: XmlEngine,
    #[cfg(debug_assertions)]
    css_rx: Option<CssRx>,
    #[cfg(debug_assertions)]
    css_watcher: Option<CssWatcher>,
}

impl WindowTemplate<Self, GuiState> for TasksWindow {
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

impl TasksWindow {
    pub fn new() -> Result<Self, WindowCreationError> {
        return Ok(Self {
            qb: QueryBuilder::new(),
            engine: XmlEngine::try_new(
                include_str!("tasks.xml").to_string(),
                DEFAULT_ENGINE_SETTINGS.clone(),
            )
            .map_err(|e| WindowCreationError::EngineCreationError(e))?,
            #[cfg(debug_assertions)]
            css_rx: None,
            #[cfg(debug_assertions)]
            css_watcher: None,
        });
    }

    pub fn reload_tasks(&mut self, app: &mut App<GuiState>) {
        if let Ok(tasks) = app.state.tasks.lock() {
            let mut task_list = XmlElement::void();

            task_list.set_tag("Row");
            task_list.set_id("tasks-list");
            task_list.apply_css("border-width", "2");
            task_list.apply_css("border-color", "white");

            for task in tasks.iter().filter_map(|t| t.1.as_ref()) {
                let mut child = XmlElement::void();

                child.set_tag("Row");

                let mut status_label = XmlElement::void();

                status_label.set_tag("Label");
                status_label.add_class("row-status");
                status_label.set_text(if let Some(result) = &task.result {
                    if result.success { "✅" } else { "ℹ️" }
                } else {
                    "⏳"
                });
                // CSS
                status_label.apply_css("fg", "white");

                child.append_child(status_label);

                let mut text_label = XmlElement::void();

                text_label.set_tag("Label");
                text_label.set_text(
                    if let Some(result) = &task.result {
                        if let Some(error) = &result.error {
                            format!("Task \"{}\" failed: {}", task.name, error)
                        } else {
                            format!("Task \"{}\" ran successfully!", task.name)
                        }
                    } else {
                        format!("Task \"{}\" running...", task.name)
                    }
                    .as_str(),
                );

                // CSS
                text_label.apply_css("fg", "white");

                child.append_child(text_label);

                task_list.append_child(child);
            }

            if tasks.is_empty() {
                let mut empty_label = XmlElement::void();

                empty_label.set_tag("Label");
                empty_label.set_text("No tasks running ^.^");
                empty_label.apply_css("fg", "white");

                task_list.append_child(empty_label);
            }

            self.qb
                .b(Dom::get_element_by_id("tasks-list").replace(Box::new(task_list)));

            self.process();
        }
    }

    pub fn post_create(&mut self, app: &mut App<GuiState>) {
        #[cfg(debug_assertions)]
        match watch_css_file(self, 1000) {
            Err(e) => {
                crate::errors::show_error(crate::errors::AppError::LazyError(e.to_string()));
            }
            Ok(w) => self.css_watcher = Some(w),
        };

        #[cfg(not(debug_assertions))]
        self.qb.import_css(crate::gui::css::CSS.to_string(), false);

        self.reload_tasks(app);

        self.qb.set_interval(500).with_callback(|this, _, app| {
            this.reload_tasks(app);
        });

        self.process();
    }
}
