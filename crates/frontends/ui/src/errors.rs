use rustydialogs::{MessageBox, MessageButtons, MessageIcon};
use thiserror::Error;

use crate::systray::errors::IconCreationError;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Failed to load the tray icon: {0}. Application cannot start.")]
    TrayIconLoadError(IconCreationError),
    #[error("Failed to build the system tray: {0}. Application cannot start.")]
    FailedToBuildTrayIcon(tray::Error),
    #[error(
        "Communication with/from system icon tray is closed / has an error, application needs to be closed"
    )]
    AppEventFailure(String),
    #[error("Failed to open the settings")]
    FailedToLoadSettings(String),
    #[error("Error while opening main window")]
    OpenUIError(String),
    #[error("Lazy error: this errors shouldn't exist / are only for dev")]
    LazyError(String),
    #[error("Failed to init the logger: {0}")]
    FailedToInitLogger(String),
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
