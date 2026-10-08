use std::sync::mpsc::Sender;

use tray::{
    MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent, dpi::PhysicalPosition,
};
use tray_menu::{PopupMenu, TextEntry};

use crate::{errors::AppError, systray::icon::load_icon};

pub enum MenuAction {
    Exit,
    AbsoluteVoid, // ✋😐🤚
    OpenUI,
}

pub struct FFSearchTray {
    tray: TrayIcon,
    sender: Sender<MenuAction>,
}

impl FFSearchTray {
    pub fn new(sender: Sender<MenuAction>) -> Result<Self, AppError> {
        let icon = load_icon().map_err(|e| AppError::TrayIconLoadError(e))?;

        let tray = TrayIconBuilder::new()
            .with_icon(icon)
            .with_tooltip("FFSearch.rs")
            .build()
            .map_err(|e| AppError::FailedToBuildTrayIcon(e))?;

        return Ok(Self { tray, sender });
    }

    pub fn run(&mut self) -> Result<(), AppError> {
        loop {
            #[cfg(target_os = "windows")]
            self.tray.recv_messages();

            std::thread::sleep(std::time::Duration::from_millis(50));

            match TrayIconEvent::receiver().try_recv() {
                Ok(event) => match event {
                    TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } => {
                        self.sender
                            .send(MenuAction::OpenUI)
                            .map_err(|e| AppError::AppEventFailure(e.to_string()))?;
                    }
                    TrayIconEvent::Click {
                        button: MouseButton::Right,
                        button_state: MouseButtonState::Up,
                        position,
                        ..
                    } => {
                        let menu_action = self.show_menu(position);

                        match menu_action {
                            MenuAction::Exit => {
                                break Ok(());
                            }
                            _ => {}
                        }

                        self.sender
                            .send(menu_action)
                            .map_err(|e| AppError::AppEventFailure(e.to_string()))?;
                    }
                    _ => {}
                },
                Err(_) => {}
            }
        }
    }

    pub fn show_menu(&mut self, position: PhysicalPosition<f64>) -> MenuAction {
        let mut menu = PopupMenu::new();

        menu.add(&TextEntry::of("exit", "Exit"));

        #[cfg(not(target_os = "windows"))]
        let id_op = menu.popup(position);
        #[cfg(target_os = "windows")]
        let id_op = menu.popup(position, self.tray.get_hwnd());

        if let Some(id) = id_op {
            if id.0 == "exit" {
                self.exit();

                return MenuAction::Exit;
            }
        };

        return MenuAction::AbsoluteVoid;
    }

    pub fn exit(&mut self) {
        // Absolute empty function ✋😐🤚
        // YOU SHALL NOT CLOSE MY APP
    }
}
