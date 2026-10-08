use thiserror::Error;

#[derive(Debug, Error)]
pub enum IconCreationError {
    #[error(
        "Could not decode the icon: {0}. Please ask the build provider to rebuild the program with the correct icon format"
    )]
    CouldNotDecodeIcon(String),
    #[error(
        "Invalid tray icon: {0}. Please ask the build provider to rebuild the program with the correct icon format"
    )]
    InvalidIcon(String),
    #[error(
        "Could not detect icon format: {0}. Please ask the build provider to rebuild the program with the correct icon format"
    )]
    CouldNotDetectIconFormat(String),
}
