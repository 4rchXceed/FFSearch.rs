use std::{
    fs::{File, canonicalize},
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};

use application::indexer::traits::file_indexer::FileIndexerProvider;
use domain::indexer::{config::FileIndexerConfig, error::FileIndexerError};
use utils::same_path;
use walkdir::{DirEntry, WalkDir};

pub struct StdFileIndexerProvider {
    config: FileIndexerConfig,
}

impl StdFileIndexerProvider {
    pub fn new(config: FileIndexerConfig) -> Self {
        return Self { config };
    }

    /// Checks if the given file is a valid filename for indexing.
    /// Parameters:
    /// - file: the file to check.
    /// Returns: bool - if the file should be indexed.
    fn is_correct_filename(&self, file: &DirEntry, folder: String) -> bool {
        let file = canonicalize(file.path());
        let db_folder = canonicalize(
            Path::new(&folder).join(self.config.db_config.database_folder_prefix.clone()),
        );

        if let Ok(file) = file
            && let Ok(db_folder) = db_folder
        {
            let file_string = file.to_string_lossy().to_string();

            if file_string.starts_with(".") || file.starts_with(db_folder) {
                return false;
            } else {
                return true;
            }
        } else {
            return false;
        }
    }

    /// Writes the modification date of a file to the database.
    /// Parameters:
    /// - file: the file to write the modification date to.
    /// - timestamp: the modification date as a unix timestamp.
    /// Returns: an error if the file could not be written, otherwise None.
    fn write_mod_date(&self, file: &PathBuf, timestamp: i64) -> Result<(), FileIndexerError> {
        let file = File::open(file)
            .map_err(|e| FileIndexerError::OpenFileToWriteModDateError(e.to_string()))?;

        file.set_modified(std::time::SystemTime::from(
            std::time::UNIX_EPOCH + std::time::Duration::from_secs(timestamp as u64),
        ))
        .map_err(|e| FileIndexerError::OpenFileToWriteModDateError(e.to_string()))?;

        return Ok(());
    }
}

impl FileIndexerProvider for StdFileIndexerProvider {
    fn get_all_files(&self, folder: String) -> Vec<PathBuf> {
        return WalkDir::new(folder.clone())
            .into_iter()
            .filter_map(|e| e.ok())
            // Filter out sys files and db folder
            .filter(|e| self.is_correct_filename(e, folder.clone()))
            // Filter out the folder itself
            .filter(|e| !same_path(&e.path().to_string_lossy().to_string(), &folder.as_str()))
            // Filter out directories
            .filter(|e| !e.path().is_dir())
            .map(|x| x.path().to_owned())
            .collect();
    }

    fn extract_modified_time(&self, path: &PathBuf) -> Result<i64, FileIndexerError> {
        let meta = std::fs::metadata(path)
            .map_err(|e| FileIndexerError::GetFileMetaError(e.to_string()))?;
        let timestamp: i64;
        let mod_date = meta.modified();

        if let Ok(mod_date) = mod_date {
            let timestamp_op = mod_date.duration_since(UNIX_EPOCH);

            if let Ok(timestamp_op) = timestamp_op {
                timestamp = timestamp_op.as_secs() as i64;
            } else {
                // Welp, time went backwards
                timestamp = chrono::Local::now().timestamp();
                self.write_mod_date(path, timestamp)?;
            }
        } else {
            timestamp = chrono::Local::now().timestamp();
            self.write_mod_date(path, timestamp)?;
        }

        return Ok(timestamp);
    }

    fn get_config(&self) -> &FileIndexerConfig {
        return &self.config;
    }
}
