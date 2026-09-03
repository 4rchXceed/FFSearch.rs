/// The default database size
/// Used by DatabaseConfig::new()
static DEFAULT_DATABASE_SIZE: usize = 100_000_000;
/// The default save folder prefix
/// Used by DatabaseConfig::new()
static DEFAULT_SAVE_FOLDER_PREFIX: &str = ".ffsearch";
/// The database config stores the config for the Searcher/DB Component
/// It is made of these elements:
/// - max_db_size: usize -> the maximum size of a database in bytes
/// - save_folder_prefix: String -> the prefix for the database folder (default: .ffsearch)
#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    /// [public] Maximum database size [bytes]
    pub max_db_size: usize,
    /// [public] The prefix for the save folder
    pub database_folder_prefix: String,
}

impl DatabaseConfig {
    /// Creates a database config
    /// Returns a new DatabaseConfig with:
    /// max_db_size being DEFAULT_DATABASE_SIZE
    /// save_folder_prefix being ".ffsearch"
    pub fn new() -> Self {
        return DatabaseConfig {
            max_db_size: DEFAULT_DATABASE_SIZE,
            database_folder_prefix: String::from(DEFAULT_SAVE_FOLDER_PREFIX),
        };
    }
}
