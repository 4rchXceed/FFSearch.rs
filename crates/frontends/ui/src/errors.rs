use rustydialogs::{MessageBox, MessageButtons, MessageIcon};
use thiserror::Error;

use crate::systray::errors::IconCreationError;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Failed to load the tray icon: {0}. Application cannot start.")]
    TrayIconLoadError(IconCreationError),
    #[error("Failed to build the system tray: {0}. Application cannot start.")]
    FailedToBuildTrayIcon(String),
    #[error("Failed to open the settings: {0}")]
    FailedToLoadSettings(String),
    #[error("Error while opening main window: {0}")]
    OpenUIError(String),
    #[cfg(debug_assertions)]
    #[error("Lazy error: this errors shouldn't exist / is only for dev: {0}")]
    LazyError(String),
    #[error("Failed to init the logger: {0}")]
    FailedToInitLogger(String),
    #[error("Could not send message between system tray and application: {0}")]
    BroadcastFailed(String),
    #[cfg(feature = "daemon")]
    #[error("Could not create the daemon: {0}")]
    CouldNotCreateDaemon(String),
    #[error("Failed to open ui: {0}")]
    FailedToOpenUI(String),
}

pub fn show_error(error: AppError) {
    let icon = MessageIcon::Error;

    let buttons = MessageButtons::Ok;

    let dialog = MessageBox {
        title: "Oh nooooooooo :(",
        message: &format!(
            "It appears that FFSearch.rs crashed :( here's some details:\nError message: {}\n\nSorry...\nSee you next time~\n(if you don't uninstall after this incident ^.^)",
            error.to_string()
        ),
        icon: icon,
        buttons: buttons,
        owner: None,
    };
    dialog.show();
}
