use iced_xml::xml_engine::XmlEngineError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum WindowCreationError {
    #[error("Failed to parse the window's code: {0}")]
    EngineCreationError(XmlEngineError),
}
