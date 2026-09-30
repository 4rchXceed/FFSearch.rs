use crate::searcher::config::DatabaseConfig;

/// The configuration for the file indexer.
/// These are the config properties for the file indexer:
/// - max_file_size: the maximum size of a single file in bytes
#[derive(Clone)]
pub struct FileIndexerConfig {
    /// The number of threads to use for indexing files
    pub number_of_threads: usize,
    /// The maximum size of a single file in bytes
    pub max_file_size: usize,
    /// The database's config
    pub db_config: DatabaseConfig,
}

impl FileIndexerConfig {
    /// Creates a new file indexer config with default values.
    pub fn new() -> Self {
        Self {
            number_of_threads: 6,
            max_file_size: 1024 * 1024 * 20, // 1MB, large files are VERY slow to index
            db_config: DatabaseConfig::new(),
        }
    }
}
