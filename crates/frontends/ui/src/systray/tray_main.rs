use tokio::sync::broadcast::Sender;
use tray_item::TrayItem;

#[cfg(target_os = "windows")]
use super::icon::load_icon_windows;

use crate::errors::{AppError, show_error};

#[cfg(not(target_os = "windows"))]
use super::icon::load_icon_linux;

#[derive(Clone)]
pub enum MenuAction {
    Exit,
    // AbsoluteVoid, // ✋😐🤚
    OpenUI,
}

pub struct FFSearchTray {
    _tray: TrayItem,
    _sender: Sender<MenuAction>,
}

impl FFSearchTray {
    pub fn new(sender: Sender<MenuAction>) -> Result<Self, AppError> {
        #[cfg(target_os = "windows")]
        let icon = load_icon_windows().map_err(|e| AppError::TrayIconLoadError(e))?;
        #[cfg(not(target_os = "windows"))]
        let icon = load_icon_linux().map_err(|e| AppError::TrayIconLoadError(e))?;

        let mut tray = TrayItem::new("FFSearch.rs", icon)
            .map_err(|e| AppError::FailedToBuildTrayIcon(e.to_string()))?;

        tray.add_menu_item("FFSearch.rs", || {})
            .map_err(|e| AppError::FailedToBuildTrayIcon(e.to_string()))?;

        let sender_clone = sender.clone();

        tray.add_menu_item("Open UI", move || {
            match sender_clone.send(MenuAction::OpenUI) {
                Ok(_) => {}
                Err(e) => show_error(AppError::BroadcastFailed(e.to_string())),
            }
        })
        .map_err(|e| AppError::FailedToBuildTrayIcon(e.to_string()))?;

        let sender_clone = sender.clone();

        tray.add_menu_item("Exit", move || match sender_clone.send(MenuAction::Exit) {
            Ok(_) => {}
            Err(e) => show_error(AppError::BroadcastFailed(e.to_string())),
        })
        .map_err(|e| AppError::FailedToBuildTrayIcon(e.to_string()))?;

        return Ok(Self {
            _tray: tray,
            _sender: sender,
        });
    }

    //pub fn exit(&mut self) {
    // Absolute empty function ✋😐🤚
    // YOU SHALL NOT CLOSE MY APP
    //}
}
