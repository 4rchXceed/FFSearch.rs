use log::Level;

use crate::{
    errors::{AppError, show_error},
    gui::params::WindowParams,
    systray::tray_main::FFSearchTray,
};

use self::{manager::AppManager, systray::tray_main::MenuAction};

mod errors;
mod gui;
mod manager;
mod systray;

async fn main_safe() -> Result<(), AppError> {
    simple_logger::init_with_level(Level::Info)
        .map_err(|e| AppError::FailedToInitLogger(e.to_string()))?;

    let (tx, mut rx1) = tokio::sync::broadcast::channel::<MenuAction>(100);
    let mut rx2 = tx.subscribe();

    // MacOS support dropped for now: https://github.com/olback/tray-item-rs/ -> * MacOS does not allow running applications in threads other than main, meaning that it is not possible to listen for events in a new thread. See the macos.rs example for a how-to.
    let _tray = match FFSearchTray::new(tx) {
        Ok(t) => t,
        Err(e) => {
            return Err(e);
        }
    };

    let args: Vec<String> = std::env::args().collect();

    let config_path = if let Some(first_arg) = args.get(1) {
        first_arg
    } else {
        &String::from("config.ini")
    };

    let (tx, rx) = std::sync::mpsc::channel::<WindowParams>();

    let mut manager = AppManager::new(config_path.clone()).await?;

    // Other handlers
    let tx_clone = tx.clone();
    let _th = tokio::spawn(async move {
        loop {
            match rx2.recv().await {
                Ok(msg) => match msg {
                    MenuAction::Exit => {
                        std::process::exit(0);
                    }
                    MenuAction::OpenUI => AppManager::reopen_ui(tx_clone.clone()),
                },
                // Tray is closed
                Err(_) => {
                    return;
                }
            }
        }
    });

    manager.open_ui(rx);

    loop {
        match rx1.recv().await {
            Ok(msg) => match msg {
                MenuAction::OpenUI => {
                    AppManager::reopen_ui(tx.clone());
                }
                _ => {}
            },
            // Tray is closed
            Err(_) => {
                return Ok(());
            }
        }
    }
}

#[tokio::main]
async fn main() {
    let maybe_error = main_safe().await;

    if let Err(error) = maybe_error {
        show_error(error);
    }
}
