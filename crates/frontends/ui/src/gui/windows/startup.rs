use iced_xml::app_manager::utils::WindowId;
use iced_xml::dom::api::Dom;
use iced_xml::dom::query_builder::QueryBuilder;
use iced_xml::dom::types::EventListenerTypes;
#[cfg(debug_assertions)]
use iced_xml::window_wrapper::CssRx;
use iced_xml::window_wrapper::{Objects, ObjectsReadOnly, WindowTemplate};
use iced_xml::xml_engine::XmlEngine;
use iced_xml::xml_struct::parser::try_xml;

use crate::gui::css::CSS;
use crate::gui::gui_main::{DEFAULT_ENGINE_SETTINGS, text};
use crate::gui::state::GuiState;

use super::errors::WindowCreationError;

pub enum Panels {
    Welcome,
    HowItWorks,
    #[cfg(feature = "ai")]
    Ai,
    #[cfg(feature = "daemon_worker")]
    Daemon,
    #[cfg(feature = "mcp_server")]
    Mcp,
}

pub struct StartupWindow {
    qb: QueryBuilder<Self, GuiState>,
    engine: XmlEngine,
    #[cfg(debug_assertions)]
    css_rx: Option<CssRx>,
    current_panel: Panels,
    me_winid: WindowId,
}

impl WindowTemplate<Self, GuiState> for StartupWindow {
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

impl StartupWindow {
    pub fn new(winid: WindowId) -> Result<Self, WindowCreationError> {
        return Ok(Self {
            qb: QueryBuilder::new(),
            engine: XmlEngine::try_new(
                include_str!("startup.xml").to_string(),
                DEFAULT_ENGINE_SETTINGS.clone(),
            )
            .map_err(|e| WindowCreationError::EngineCreationError(e))?,
            #[cfg(debug_assertions)]
            css_rx: None,
            current_panel: Panels::Welcome,
            me_winid: winid,
        });
    }

    pub fn render_panel(&mut self) {
        let code = match self.current_panel {
            Panels::Welcome => include_str!("startup_panels/welcome.xml"),
            Panels::HowItWorks => include_str!("startup_panels/howitworks.xml"),
            #[cfg(feature = "ai")]
            Panels::Ai => include_str!("startup_panels/ai.xml"),
            #[cfg(feature = "mcp_server")]
            Panels::Mcp => include_str!("startup_panels/mcp.xml"),
            #[cfg(feature = "daemon_worker")]
            Panels::Daemon => include_str!("startup_panels/daemon.xml"),
        };

        let mut element = match try_xml(code) {
            Ok(e) => e,
            Err(e) => text(&e),
        };

        element.set_id("panel");

        self.qb
            .b(Dom::get_element_by_id("panel").replace(Box::new(element)));

        self.qb.import_css(CSS.to_string(), false);

        self.process();
    }

    pub fn post_create(&mut self) {
        self.render_panel();

        self.qb
            .b(Dom::get_element_by_id("next").add_event_listener(EventListenerTypes::Click))
            .with_callback(|this, _, app| {
                match this.current_panel {
                    Panels::Welcome => {
                        this.current_panel = Panels::HowItWorks;
                    }
                    Panels::HowItWorks => {
                        #[cfg(not(feature = "ai"))]
                        #[cfg(not(feature = "mcp_server"))]
                        #[cfg(not(feature = "daemon_worker"))]
                        {
                            this.qb.close_window(this.me_winid, app);

                            this.process();

                            return;
                        }

                        #[cfg(feature = "ai")]
                        {
                            this.current_panel = Panels::Ai;
                        }

                        #[cfg(not(feature = "ai"))]
                        #[cfg(feature = "mcp_server")]
                        {
                            this.current_panel = Panels::Mcp;
                        }

                        #[cfg(not(feature = "ai"))]
                        #[cfg(not(feature = "mcp_server"))]
                        #[cfg(feature = "daemon_worker")]
                        {
                            this.current_panel = Panels::Daemon;
                        }
                    }
                    #[cfg(feature = "ai")]
                    Panels::Ai => {
                        #[cfg(not(feature = "mcp_server"))]
                        #[cfg(not(feature = "daemon_worker"))]
                        {
                            this.qb.close_window(this.me_winid, app);

                            this.process();

                            return;
                        }

                        #[cfg(feature = "mcp_server")]
                        {
                            this.current_panel = Panels::Mcp;
                        }

                        #[cfg(not(feature = "mcp_server"))]
                        #[cfg(feature = "daemon_worker")]
                        {
                            this.current_panel = Panels::Daemon;
                        }
                    }
                    #[cfg(feature = "mcp")]
                    Panels::Mcp => {
                        #[cfg(not(feature = "daemon_worker"))]
                        {
                            this.qb.close_window(this.me_winid, app);

                            this.process();

                            return;
                        }

                        #[cfg(feature = "daemon_worker")]
                        {
                            this.current_panel = Panels::Daemon;
                        }
                    }
                    #[cfg(feature = "daemon_worker")]
                    Panels::Daemon => {
                        this.qb.close_window(this.me_winid, app);

                        this.process();

                        return;
                    }
                }

                this.render_panel();
            });

        self.process();
    }
}
