use application::ai::actions::ai_chat::AiChat;
use application::indexer::traits::registry::IndexerId;
use domain::ai::message::AIMessage;
use iced_xml::app_manager::state_wrapper::App;
use iced_xml::dom::api::Dom;
use iced_xml::dom::query_builder::QueryBuilder;
use iced_xml::dom::types::{EventListenerTypes, EventResponse};
#[cfg(debug_assertions)]
use iced_xml::utils::watch_css::watch_css_file;
#[cfg(debug_assertions)]
use iced_xml::window_wrapper::{CssRx, CssWatcher};
use iced_xml::window_wrapper::{Objects, ObjectsReadOnly, WindowTemplate};
use iced_xml::xml_engine::XmlEngine;
use iced_xml::xml_struct::xml_element::XmlElement;
use log::{error, info};
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::task::JoinHandle;
use wait::Waitable;

use crate::gui::gui_main::DEFAULT_ENGINE_SETTINGS;
use crate::gui::state::GuiState;

use super::errors::WindowCreationError;

pub struct AiChatWindow {
    qb: QueryBuilder<Self, GuiState>,
    engine: XmlEngine,
    #[cfg(debug_assertions)]
    css_rx: Option<CssRx>,
    #[cfg(debug_assertions)]
    css_watcher: Option<CssWatcher>,
    folder: IndexerId,
    chat: AiChat,
    last_message: Option<String>,
    ask_thread: Option<JoinHandle<AIMessage>>,
    recv: Receiver<String>,
    sender: Sender<String>,
    messages: Vec<AIMessage>,
}

impl WindowTemplate<Self, GuiState> for AiChatWindow {
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

impl AiChatWindow {
    pub fn new(folder: IndexerId, app: &mut App<GuiState>) -> Result<Self, WindowCreationError> {
        let (tx, rx) = tokio::sync::mpsc::channel::<String>(100);

        return Ok(Self {
            qb: QueryBuilder::new(),
            engine: XmlEngine::try_new(
                include_str!("aichat.xml").to_string(),
                DEFAULT_ENGINE_SETTINGS.clone(),
            )
            .map_err(|e| WindowCreationError::EngineCreationError(e))?,
            #[cfg(debug_assertions)]
            css_rx: None,
            #[cfg(debug_assertions)]
            css_watcher: None,
            folder: folder,
            chat: AiChat::new(app.state.registry.clone()),
            last_message: None,
            ask_thread: None,
            recv: rx,
            sender: tx,
            messages: Vec::new(),
        });
    }

    pub fn update_messages(&mut self) {
        let mut container = XmlElement::void();
        container.set_tag("Col");
        container.set_id("messages");

        let mut messages = self.messages.clone();

        if let Some(message) = &self.last_message {
            messages.push(AIMessage::assistant(message.clone()));
        }

        for message in messages {
            let mut message_elem = XmlElement::void();
            message_elem.set_tag("Label");
            message_elem.set_text(&format!(
                "{}:\n{}",
                message.sender.to_uppercase(),
                message.content
            ));
            message_elem.apply_css("fg", "white");

            container.append_child(message_elem);
        }

        self.qb
            .b(Dom::get_element_by_id("messages").replace(Box::new(container)));

        self.process();
    }

    pub fn ask_ai(&mut self, query: String) -> Result<(), String> {
        if self.ask_thread.is_none() {
            let folder = self.folder;
            let sender = self.sender.clone();
            let chat = self.chat.clone();

            self.messages.push(AIMessage::user(query.clone()));

            let th = tokio::spawn(async move {
                return Self::ask_ai_threaded(chat, folder, query, sender).await;
            });

            self.ask_thread = Some(th);

            self.last_message = Some(String::from(""));
            self.set_status("");

            return Ok(());
        } else {
            return Err(String::from("Cannot ask two questions at a time"));
        }
    }

    pub async fn ask_ai_threaded(
        chat: AiChat,
        folder: IndexerId,
        query: String,
        sender: Sender<String>,
    ) -> AIMessage {
        let result = chat.ask(folder, query, sender.clone()).await;

        let last_message = match result {
            Ok(r) => r,
            Err(error) => AIMessage::assistant(error),
        };

        return last_message;
    }

    pub fn refresh_message(&mut self, _: EventResponse, _: &mut App<GuiState>) {
        while let Ok(msg) = self.recv.try_recv() {
            if let Some(last_message) = &mut self.last_message {
                info!("Recv: {msg}");
                last_message.push_str(&msg);
            } else {
                error!("Dropped: {msg}");
            }
        }

        if let Some(handle) = &mut self.ask_thread
            && let Some(last_message) = &self.last_message
        {
            if handle.is_finished() {
                self.messages
                    .push(AIMessage::assistant(last_message.clone()));
                self.ask_thread = None;
                self.last_message = None;
            }
        }

        self.update_messages();
    }

    pub fn set_status(&mut self, status: &str) {
        self.qb
            .b(Dom::get_element_by_id("status").set_property("text", status));

        self.process();
    }

    pub fn post_create(&mut self, prompt: String, app: &mut App<GuiState>) {
        #[cfg(not(debug_assertions))]
        self.qb.import_css(crate::gui::css::CSS.to_string(), false);

        let folder_path = app
            .state
            .registry
            .get_folder(self.folder)
            .wait()
            .unwrap_or(String::from("Unknown folder"));

        self.qb.b(Dom::get_element_by_id("askabout")
            .set_property("text", &format!("Chat about the content of: {folder_path}")));

        #[cfg(debug_assertions)]
        match watch_css_file(self, 1000) {
            Err(e) => {
                crate::errors::show_error(crate::errors::AppError::LazyError(e.to_string()));
            }
            Ok(w) => self.css_watcher = Some(w),
        };

        match self.ask_ai(prompt) {
            Ok(_) => {}
            Err(error) => self.set_status(&error),
        };

        self.refresh_message(EventResponse::default(), app);

        self.qb
            .set_interval(500)
            .with_callback(Self::refresh_message);

        self.qb
            .b(Dom::get_element_by_id("prompt-btn").add_event_listener(EventListenerTypes::Click))
            .with_callback(|this, _, _| {
                this.qb
                    .b(Dom::get_element_by_id("prompt-input").get_property("value"))
                    .then(|this, qr| {
                        if let Some(prompt) = qr.data_str
                            && !prompt.is_empty()
                        {
                            match this.ask_ai(prompt) {
                                Ok(_) => {}
                                Err(error) => this.set_status(&error),
                            };
                            this.qb
                                .b(Dom::get_element_by_id("prompt-input")
                                    .set_property("value", ""));

                            this.process();
                        }
                    });

                this.process();
            });

        self.process();
    }
}
