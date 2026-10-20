#[derive(Clone)]
pub enum WindowType {
    HomeWindow,
    ErrorWindow(String),
    TasksWindow,
    SettingsWindow,
}

#[derive(Clone)]
pub struct WindowParams {
    pub wintype: WindowType,
    pub startup_errors: Vec<String>,
}

impl WindowParams {
    pub fn error(error: String) -> Self {
        return Self {
            wintype: WindowType::ErrorWindow(error),
            startup_errors: Vec::new(),
        };
    }

    pub fn home(errors: Vec<String>) -> Self {
        return Self {
            wintype: WindowType::HomeWindow,
            startup_errors: errors,
        };
    }

    pub fn settings() -> Self {
        return Self {
            wintype: WindowType::SettingsWindow,
            startup_errors: Vec::new(),
        };
    }
}
