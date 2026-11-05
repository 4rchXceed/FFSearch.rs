pub enum SearchContent {
    Path,
    ContentAndPath,
}

pub struct SearchQuery {
    pub query: String,
    pub limit: usize,
    pub search_for: SearchContent,
}
