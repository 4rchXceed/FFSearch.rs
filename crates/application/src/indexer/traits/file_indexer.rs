use std::path::PathBuf;

use async_trait::async_trait;
use domain::indexer::{config::FileIndexerConfig, error::FileIndexerError};

#[async_trait]
pub trait FileIndexerProvider {
    /// Gets all the files in the indexer's folder
    ///
    /// Returns:
    /// - Vec<PathBuf>: all the files
    fn get_all_files(&self, folder: String) -> Vec<PathBuf>;

    /// Extracts the modification date of a file.
    /// Parameters:
    /// - file: the file to extract the modification date from.
    /// Returns: the modification date as a unix timestamp, or an error if the file could not be read.
    fn extract_modified_time(&self, path: &PathBuf) -> Result<i64, FileIndexerError>;

    /// Returns the config (as ref)
    fn get_config(&self) -> &FileIndexerConfig;
}
