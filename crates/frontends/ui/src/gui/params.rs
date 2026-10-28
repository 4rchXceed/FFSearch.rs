use application::indexer::traits::registry::IndexerId;
use domain::searcher::document::Document;

#[derive(Clone)]
pub enum WindowType {
    HomeWindow(bool, Vec<String>),
    ErrorWindow(String),
    SettingsWindow,
    StartupWindow,
    ResultsWindow(Vec<Document>, String, IndexerId),
    DetailsWindow(String, IndexerId),
}

#[derive(Clone)]
pub struct WindowParams {
    pub wintype: WindowType,
}

impl WindowParams {
    pub fn error(error: String) -> Self {
        return Self {
            wintype: WindowType::ErrorWindow(error),
            ..Default::default()
        };
    }

    pub fn home(errors: Vec<String>, first_opening: bool) -> Self {
        return Self {
            wintype: WindowType::HomeWindow(first_opening, errors),
            ..Default::default()
        };
    }

    pub fn settings() -> Self {
        return Self {
            wintype: WindowType::SettingsWindow,
            ..Default::default()
        };
    }

    pub fn startup() -> Self {
        return Self {
            wintype: WindowType::StartupWindow,
            ..Default::default()
        };
    }

    pub fn results(results: Vec<Document>, query: String, indexer: IndexerId) -> Self {
        return Self {
            wintype: WindowType::ResultsWindow(results, query, indexer),
            ..Default::default()
        };
    }

    pub fn details(path: String, indexer: IndexerId) -> Self {
        return Self {
            wintype: WindowType::DetailsWindow(path, indexer),
            ..Default::default()
        };
    }
}

impl Default for WindowParams {
    fn default() -> Self {
        return Self {
            wintype: WindowType::HomeWindow(false, Vec::new()),
        };
    }
}
