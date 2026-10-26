use log::Level;

use crate::{
    errors::{AppError, show_error},
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

    let (sender, recv) = std::sync::mpsc::channel::<MenuAction>();

    // MacOS support dropped for now: https://github.com/olback/tray-item-rs/ -> * MacOS does not allow running applications in threads other than main, meaning that it is not possible to listen for events in a new thread. See the macos.rs example for a how-to.
    let mut tray = match FFSearchTray::new(sender) {
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

    let mut manager = AppManager::new(config_path.clone()).await?;

    manager.open_ui();

    loop {
        match recv.recv() {
            Ok(msg) => match msg {
                MenuAction::OpenUI => {
                    manager.open_ui();
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
