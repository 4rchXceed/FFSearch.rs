use thiserror::Error;

#[derive(Debug, Error)]
pub enum IconCreationError {
    #[cfg(not(windows))]
    #[error(
        "Could not decode the icon: {0}. Please ask the build provider to rebuild the program with the correct icon format"
    )]
    CouldNotDecodeIcon(String),
    #[cfg(not(windows))]
    #[error(
        "Invalid tray icon: {0}. Please ask the build provider to rebuild the program with the correct icon format"
    )]
    InvalidIcon(String),
    // #[cfg(not(windows))]
    // #[error(
    //     "Could not detect icon format: {0}. Please ask the build provider to rebuild the program with the correct icon format"
    // )]
    // CouldNotDetectIconFormat(String),
}
