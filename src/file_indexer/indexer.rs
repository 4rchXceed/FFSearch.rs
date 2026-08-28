use std::{fs::File, path::PathBuf, time::UNIX_EPOCH};

use tantivy::DateTime;
use walkdir::{DirEntry, WalkDir};

use crate::{
    file_indexer::{config::FileIndexerConfig, text_extractor::TextExtractor},
    not_err_check,
    searcher::database::{Database, DatabaseError, Document},
    utils::{same_path, trim_custom},
};

/// Represents all possible errors that can occur during file indexing.
#[derive(Debug)]
pub enum FileIndexerError {
    /// Failed to get file metadata.
    GetFileMetaError(std::io::Error),
    /// Failed to open file to write modification date.
    OpenFileToWriteModDateError(std::io::Error),
    /// Failed to update document in the database.
    CouldNotUpdateDocument(DatabaseError),
    /// Failed to add document to the database.
    CouldNotAddDocument(DatabaseError),
    /// Invalid file path.
    InvalidFilePath(String),
    /// Failed to remove document from the database.
    FailedToRemoveDocument(DatabaseError),
    /// The database failed to init
    DatabaseInitError(DatabaseError),
}

/// This is the file indexer, responsible for indexing files in a given folder to the database.
pub struct FileIndexer {
    /// The folder to index. (absolute path is REALLY recommended)
    folder: String,
    /// The database to store the index in.
    db: Database,
    /// The text extractor to use for extracting file content.
    extractor: TextExtractor,
    /// The configuration for the file indexer.
    config: FileIndexerConfig,
}

impl FileIndexer {
    /// Creates a new file indexer with the given folder, database, and configuration.
    /// Parameters:
    /// - folder: the folder to index. (absolute path is REALLY recommended)
    /// - config: the configuration for the file indexer.
    /// Returns: a new FileIndexer instance.
    pub fn new(folder: String, config: FileIndexerConfig) -> Result<Self, FileIndexerError> {
        let database = Database::new(folder.clone(), config.clone().db_config)
            .map_err(|e| FileIndexerError::DatabaseInitError(e))?;

        return Ok(Self {
            folder: folder,
            db: database,
            extractor: TextExtractor::new(
                config.one_file_max_length,
                config.enable_ocr,
                &config.ocr_lang,
            ),
            config: config,
        });
    }

    /// Checks if the given file is a valid filename for indexing.
    /// Parameters:
    /// - file: the file to check.
    /// Returns: bool - if the file should be indexed.
    fn is_correct_filename(&self, file: &DirEntry) -> bool {
        let db_folder = self.db.get_config().database_folder_prefix;
        if file.file_name().to_str().is_none()
            || file.file_name().to_str().unwrap().starts_with(".")
            || file.path().parent().is_none()
            || file.path().parent().unwrap().to_str().is_none()
            || file
                .path()
                .parent()
                .unwrap()
                .to_str()
                .unwrap()
                .ends_with(db_folder.as_str())
        {
            return false;
        } else {
            return true;
        }
    }

    /// Syncs the files in the folder to the database.
    /// Returns: a vector of any errors that occurred during syncing.
    pub fn sync(&mut self) -> Vec<FileIndexerError> {
        let all = self.db.get_all();

        let files: Vec<PathBuf> = WalkDir::new(self.folder.clone())
            .into_iter()
            .filter_map(|e| e.ok())
            // Filter out sys files and db folder
            .filter(|e| self.is_correct_filename(e))
            // Filter out the folder itself
            .filter(|e| {
                !same_path(
                    &e.path().to_string_lossy().to_string(),
                    &self.folder.as_str(),
                )
            })
            // Filter out directories
            .filter(|e| !e.path().is_dir())
            .map(|x| x.path().to_owned())
            .collect();

        let mut all_errors: Vec<FileIndexerError> = Vec::new();
        for file in &files {
            let result = self.index_one_file(file, &all);
            if result.is_err() {
                all_errors.push(result.err().unwrap());
            }
        }

        let local_files: Vec<String> = files
            .iter()
            .map(|f| self.to_local_path(f.to_string_lossy().to_string()).unwrap())
            .collect();

        for db_file in all {
            if !local_files.contains(&db_file.path) {
                let result = self.db.remove_document(db_file.path);
                if result.is_err() {
                    all_errors.push(FileIndexerError::FailedToRemoveDocument(
                        result.err().unwrap(),
                    ));
                }
            }
        }

        return all_errors;
    }

    /// Converts a file path (absolute/relative) to a local file path (relative to the folder being indexed).
    /// Parameters:
    /// - path: the file path to convert.
    /// Returns: the local file path as a string.
    fn to_local_path(&self, path: String) -> Result<String, FileIndexerError> {
        let local_file_path = path.strip_prefix(self.folder.as_str());
        if local_file_path.is_none() {
            return Err(FileIndexerError::InvalidFilePath(path));
        }
        let mut local_file_path = trim_custom(local_file_path.unwrap().to_string(), "/");
        local_file_path = trim_custom(local_file_path, "\\");
        return Ok(local_file_path);
    }

    /// Indexes a single file to the database.
    /// Parameters:
    /// - file: the file to index (as PathBuf)
    /// - db_files: the list of files already in the database.
    /// Returns: an error if the file could not be indexed, otherwise None.
    pub fn index_one_file(
        &mut self,
        file: &PathBuf,
        db_files: &Vec<Document>,
    ) -> Result<(), FileIndexerError> {
        let timestamp = self.extract_mod_date(&file);
        if timestamp.is_ok() {
            let timestamp = timestamp.unwrap();
            let file_path = file.to_string_lossy().to_string();
            let local_file_path = self.to_local_path(file_path.clone())?;

            let same = db_files
                .iter()
                .find(|h| same_path((**h).path.as_str(), local_file_path.as_str()));
            // Case: the file is not in the database, add it
            if same.is_none() {
                let content = self.extractor.extract(file_path.clone());
                if content.is_ok() {
                    let content = content.unwrap();

                    let res = self
                        .db
                        .add_document(local_file_path.clone(), content, timestamp);
                    if res.is_err() {
                        return Err(FileIndexerError::CouldNotAddDocument(res.err().unwrap()));
                    }
                }
            // Case: the file is in the database but the modification date has changed, update it
            } else if same.unwrap().modification_date != DateTime::from_timestamp_secs(timestamp) {
                let content = self.extractor.extract(file_path.clone());
                if content.is_ok() {
                    let content = content.unwrap();

                    let res = self.db.update_document(local_file_path, content, timestamp);
                    if res.is_err() {
                        return Err(FileIndexerError::CouldNotUpdateDocument(res.err().unwrap()));
                    }
                }
            }
            return Ok(());
        } else {
            return Err(timestamp.err().unwrap());
        }
    }

    /// Extracts the modification date of a file.
    /// Parameters:
    /// - file: the file to extract the modification date from.
    /// Returns: the modification date as a unix timestamp, or an error if the file could not be read.
    fn extract_mod_date(&self, file: &PathBuf) -> Result<i64, FileIndexerError> {
        let meta = std::fs::metadata(file);
        if meta.is_ok() {
            let timestamp: i64;
            let meta = meta.unwrap();
            let mod_date = meta.modified();
            if mod_date.is_err() {
                timestamp = chrono::Local::now().timestamp();
                self.write_mod_date(file, timestamp)?;
            } else {
                let timestamp_op = mod_date.unwrap().duration_since(UNIX_EPOCH);
                if timestamp_op.is_ok() {
                    timestamp = timestamp_op.unwrap().as_secs() as i64;
                } else {
                    // Welp, time went backwards
                    timestamp = chrono::Local::now().timestamp();
                    self.write_mod_date(file, timestamp)?;
                }
            }
            return Ok(timestamp);
        } else {
            return Err(FileIndexerError::GetFileMetaError(meta.err().unwrap()));
        }
    }

    /// Writes the modification date of a file to the database.
    /// Parameters:
    /// - file: the file to write the modification date to.
    /// - timestamp: the modification date as a unix timestamp.
    /// Returns: an error if the file could not be written, otherwise None.
    fn write_mod_date(&self, file: &PathBuf, timestamp: i64) -> Result<(), FileIndexerError> {
        let file = File::open(file);
        if file.is_ok() {
            let file = file.unwrap();
            file.set_modified(std::time::SystemTime::from(
                std::time::UNIX_EPOCH + std::time::Duration::from_secs(timestamp as u64),
            ))
            .map_err(|e| FileIndexerError::OpenFileToWriteModDateError(e))?;
            return Ok(());
        } else {
            return Err(FileIndexerError::OpenFileToWriteModDateError(
                file.err().unwrap(),
            ));
        }
    }

    /// [Tests only] Gets the database.
    /// Returns: a mutable reference to the database.
    pub fn get_db(&mut self) -> &mut Database {
        return &mut self.db;
    }
}

#[cfg(test)]
mod tests {
    use std::{
        ffi::OsString,
        fs::{File, create_dir, remove_dir_all, remove_file},
        io::Write,
        path::Path,
        str::FromStr,
        thread::sleep,
        time::Duration,
    };

    use tempdir::TempDir;

    use crate::{
        searcher::{config::DatabaseConfig, database::Database},
        utils::join_path,
    };

    use super::*;

    static FIRST_FILE_CONTENT: &str =
        "This is some t-text, yeah I guess I wiww wwite some mowe text";
    static SECOND_FILE_CONTENT: &str = "Love: Rust tackwes scale and c-c-concuwwency. Love: Rust is a modewn wanguage. Love: You can teww the *boops your nose* Rust compiler what to do. Love: Rust has gweat backwawd compatibiwity. Love: Pwogwamming without abstwactions. Love: A bettew w-way to do gawbage cowwection";
    static THIRD_FILE_CONTENT: &str = "Wowaka rest in peace <3";

    fn write_to_file(path: OsString, content: &str) {
        let file = File::create(path);
        assert!(file.is_ok());
        let res = file.unwrap().write_all(content.as_bytes());
        assert!(res.is_ok())
    }

    fn generate_test_datas(path: &str) {
        let file1_path = OsString::from_str(join_path(path, "file1.txt").as_str());
        let dir1_path = OsString::from_str(join_path(path, "dir1").as_str());
        let file2_path = OsString::from_str(join_path(path, "file2.txt").as_str());
        assert!(file1_path.is_ok());
        assert!(dir1_path.is_ok());
        assert!(file2_path.is_ok());
        write_to_file(file1_path.unwrap(), FIRST_FILE_CONTENT);
        let res = create_dir(dir1_path.unwrap());
        assert!(res.is_ok());
        write_to_file(file2_path.unwrap(), SECOND_FILE_CONTENT);
    }

    fn setup() -> (FileIndexer, TempDir, String) {
        let temp_dir = TempDir::new("ffsearch_unittest_");
        assert!(temp_dir.is_ok());
        let temp_dir = temp_dir.unwrap();
        let temp_dir_path = temp_dir.path().to_str();
        assert!(temp_dir_path.is_some());

        let folder = String::from(temp_dir_path.unwrap());

        let file_indexer = FileIndexer::new(folder.clone(), FileIndexerConfig::new());

        assert!(file_indexer.is_ok());

        return (file_indexer.unwrap(), temp_dir, folder);
    }

    #[test]
    fn test_sync_one() {
        let (mut indexer, tempdir, folder_path) = setup();
        generate_test_datas(folder_path.as_str());
        let errors = indexer.sync();
        assert!(errors.is_empty());

        let db = indexer.get_db();

        let file1 = db.get(String::from("file1.txt"));
        let file2 = db.get(String::from("file2.txt"));
        let file3 = db.get(String::from("dir1/file2.txt"));

        assert!(file1.is_ok());

        let mut first_content = String::from(FIRST_FILE_CONTENT);
        first_content.push('\n');
        assert_eq!(file1.unwrap().content.unwrap(), first_content);

        let mut second_content = String::from(SECOND_FILE_CONTENT);
        second_content.push('\n');
        assert_eq!(file2.unwrap().content.unwrap(), second_content);

        assert!(match file3.err().unwrap() {
            DatabaseError::NoResult => true,
            _ => false,
        });

        let _ = tempdir.close();
    }

    #[test]
    fn test_sync_two() {
        let (mut indexer, tempdir, folder_path) = setup();
        generate_test_datas(folder_path.as_str());
        let errors = indexer.sync();
        assert!(errors.is_empty());

        sleep(Duration::from_secs(2)); // If the files are updated too quickly, the sync may not detect the change

        let path = OsString::from_str(join_path(folder_path.as_str(), "file2.txt").as_str());
        assert!(path.is_ok());
        write_to_file(path.unwrap(), THIRD_FILE_CONTENT);

        let errors = indexer.sync();
        assert!(errors.is_empty());

        let db = indexer.get_db();

        let file1 = db.get(String::from("file1.txt"));
        assert!(file1.is_ok());
        let mut first_content = String::from(FIRST_FILE_CONTENT);
        first_content.push('\n');
        assert_eq!(file1.unwrap().content.unwrap(), first_content);

        let file3 = db.get(String::from("file2.txt"));
        assert!(file3.is_ok());
        let mut third_content = String::from(THIRD_FILE_CONTENT);
        third_content.push('\n');
        assert_eq!(file3.unwrap().content.unwrap(), third_content);

        let _ = tempdir.close();
    }

    #[test]
    fn test_relative_path() {
        let folder = String::from("/tmp/ffsearch-test");
        if !Path::new(&folder).exists() {
            let _ = create_dir(folder.clone());
        } else {
            let _ = remove_dir_all(&folder);
            let _ = create_dir(folder.clone());
        }

        let mut file_indexer = FileIndexer::new(folder.clone(), FileIndexerConfig::new()).unwrap();
        assert!(file_indexer.sync().is_empty());
        generate_test_datas(folder.as_str());
        assert!(file_indexer.sync().is_empty());
    }

    #[test]
    fn test_sync_three() {
        let (mut indexer, tempdir, folder_path) = setup();
        generate_test_datas(folder_path.as_str());
        indexer.sync();

        let path = OsString::from_str(join_path(folder_path.as_str(), "file1.txt").as_str());
        assert!(path.is_ok());
        let _ = remove_file(path.unwrap());

        indexer.sync();

        let db = indexer.get_db();

        let file1 = db.get(String::from("file1.txt"));
        assert!(match file1.clone().err().unwrap() {
            DatabaseError::NoResult => true,
            _ => false,
        });

        let _ = tempdir.close();
    }
}
