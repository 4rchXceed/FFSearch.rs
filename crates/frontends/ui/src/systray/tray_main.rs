use std::sync::mpsc::Sender;

use tray_item::TrayItem;

#[cfg(target_os = "windows")]
use super::icon::load_icon_windows;

use crate::errors::AppError;

#[cfg(not(target_os = "windows"))]
use super::icon::load_icon_linux;

pub enum MenuAction {
    Exit,
    AbsoluteVoid, // ✋😐🤚
    OpenUI,
}

pub struct FFSearchTray {
    tray: TrayItem,
    sender: Sender<MenuAction>,
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
            sender_clone.send(MenuAction::OpenUI).unwrap();
        })
        .map_err(|e| AppError::FailedToBuildTrayIcon(e.to_string()))?;

        let sender_clone = sender.clone();

        tray.add_menu_item("exit", move || {
            sender_clone.send(MenuAction::Exit).unwrap();
        })
        .map_err(|e| AppError::FailedToBuildTrayIcon(e.to_string()))?;

        return Ok(Self { tray, sender });
    }

    pub fn exit(&mut self) {
        // Absolute empty function ✋😐🤚
        // YOU SHALL NOT CLOSE MY APP
    }
}
