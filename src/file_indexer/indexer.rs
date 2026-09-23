use std::{
    fs::{File, canonicalize},
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};

use tantivy::DateTime;
use thiserror::Error;
use tqdm::tqdm;
use wait::Waitable;
use walkdir::{DirEntry, WalkDir};

use crate::{
    file_indexer::{config::FileIndexerConfig, text_extractor::extract},
    searcher::database::{Database, DatabaseError, Document},
    utils::{same_path, trim_custom},
};

/// Represents all possible errors that can occur during file indexing.
#[derive(Debug, Error)]
pub enum FileIndexerError {
    #[error("Failed to get file metadata: {0}")]
    GetFileMetaError(std::io::Error),
    #[error("Failed to open file to write modification date: {0}")]
    OpenFileToWriteModDateError(std::io::Error),
    #[error("Failed to update document in the database: {0}")]
    CouldNotUpdateDocument(DatabaseError),
    #[error("Failed to add document to the database: {0}")]
    CouldNotAddDocument(DatabaseError),
    #[error("Invalid file path: {0}")]
    InvalidFilePath(String),
    #[error("Failed to remove document from the database: {0}")]
    FailedToRemoveDocument(DatabaseError),
    #[error("The database failed to init: {0}")]
    DatabaseInitError(DatabaseError),
    #[error("Failed to get all the documents from the DB: {0}")]
    GetAllDocumentsFailed(DatabaseError),
    #[error(
        "Thread panicked during file indexing, this is likely a bug in the code. You can check the logs."
    )]
    ThreadError,
    #[error("Failed to get file content: {0}")]
    GetFileContentError(String),
    #[error("Failed to commit changes to the database: {0}")]
    DatabaseCommitError(DatabaseError),
}

/// Represents the type of operation needed for a file during indexing.
#[derive(Clone)]
pub enum FileOperationNeeded {
    Add,
    Update,
    None,
}

/// This is the file indexer, responsible for indexing files in a given folder to the database.
pub struct FileIndexer {
    /// The folder to index. (absolute path is REALLY recommended)
    folder: String,
    /// The database to store the index in.
    db: Database,
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
            config: config,
        });
    }

    /// Checks if the given file is a valid filename for indexing.
    /// Parameters:
    /// - file: the file to check.
    /// Returns: bool - if the file should be indexed.
    fn is_correct_filename(&self, file: &DirEntry) -> bool {
        let file = canonicalize(file.path());
        let db_folder = canonicalize(self.db.get_index_path());

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

    /// Syncs the files in the folder to the database.
    /// Returns: a vector of any errors that occurred during syncing.
    pub fn sync(&mut self) -> Result<Vec<FileIndexerError>, FileIndexerError> {
        let all = self
            .db
            .get_all(true)
            .map_err(|e| FileIndexerError::GetAllDocumentsFailed(e))?;

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
        let mut all_files_to_process = Vec::new();

        let number_of_threads = self.config.number_of_threads;

        for file in tqdm(files.iter()).desc(Some(String::from("Checking every file"))) {
            let result = self.index_one_file(file, &all);

            if let Err(error) = result {
                all_errors.push(error);
            } else if let Ok(result) = result {
                match result {
                    FileOperationNeeded::None => {}
                    _ => all_files_to_process.push((file.clone(), result)),
                }
            }
        }

        let mut threads = Vec::new();

        let max_file_size = self.config.max_file_size;

        for th in 0..number_of_threads {
            let thread_process_files = all_files_to_process
                .iter()
                .enumerate()
                .filter(|(i, _)| i % number_of_threads == th)
                .map(|(_, d)| (d.0.clone(), d.1.clone()))
                .collect::<Vec<(PathBuf, FileOperationNeeded)>>()
                .clone();

            threads.push(tokio::spawn(async move {
                let mut results = Vec::new();
                for (file, operation) in
                    tqdm(thread_process_files).desc(Some(format!("Thread: {}", th)))
                {
                    results.push((
                        file.clone(),
                        operation,
                        extract(file.to_string_lossy().to_string(), max_file_size).await,
                    ));
                }
                return results;
            }));
        }

        for thread in threads {
            let results = thread.wait();
            if let Ok(results) = results {
                for (file, operation, result) in results {
                    if result.is_ok() {
                        self.process_result(&mut all_errors, file, operation, result);
                    } else if let Err(error) = result {
                        all_errors.push(FileIndexerError::GetFileContentError(error));
                    }
                }
            } else {
                all_errors.push(FileIndexerError::ThreadError);
            }
        }

        let local_files: Vec<String> = files
            .iter()
            .map(|f| self.to_local_path(f.to_string_lossy().to_string()))
            .filter(|p| p.is_ok())
            .map(|p| p.unwrap_or_default())
            .collect();

        for db_file in all {
            if !local_files.contains(&db_file.path) {
                let result = self.db.remove_document(db_file.path);
                if let Err(error) = result {
                    all_errors.push(FileIndexerError::FailedToRemoveDocument(error));
                }
            }
        }

        self.db
            .commit()
            .map_err(|e| FileIndexerError::DatabaseCommitError(e))?;

        return Ok(all_errors);
    }

    fn process_result(
        &mut self,
        all_errors: &mut Vec<FileIndexerError>,
        file: PathBuf,
        operation: FileOperationNeeded,
        file_content_op: Result<String, String>,
    ) {
        if let Ok(content) = file_content_op {
            let file_path = file.to_string_lossy().to_string();
            let local_file_path = self.to_local_path(file_path.clone());
            if let Ok(local_file_path) = local_file_path {
                let timestamp = self.extract_mod_date(&file);
                if let Ok(timestamp) = timestamp {
                    match operation {
                        FileOperationNeeded::Add => {
                            let result = self.db.add_document(local_file_path, content, timestamp);
                            if let Err(error) = result {
                                all_errors.push(FileIndexerError::CouldNotAddDocument(error));
                            }
                        }
                        FileOperationNeeded::Update => {
                            let result =
                                self.db.update_document(local_file_path, content, timestamp);
                            if let Err(error) = result {
                                all_errors.push(FileIndexerError::CouldNotUpdateDocument(error));
                            }
                        }
                        FileOperationNeeded::None => {}
                    };
                } else if let Err(error) = timestamp {
                    all_errors.push(error);
                }
            } else if let Err(error) = local_file_path {
                all_errors.push(error);
            }
        } else if let Err(error) = file_content_op {
            all_errors.push(FileIndexerError::GetFileContentError(error));
        }
    }

    /// Converts a file path (absolute/relative) to a local file path (relative to the folder being indexed).
    /// Parameters:
    /// - path: the file path to convert.
    /// Returns: the local file path as a string.
    fn to_local_path(&self, path: String) -> Result<String, FileIndexerError> {
        let local_file_path = path.strip_prefix(self.folder.as_str());

        if let Some(local_file_path) = local_file_path {
            let mut local_file_path = trim_custom(local_file_path.to_string(), "/");

            local_file_path = trim_custom(local_file_path, "\\");

            return Ok(local_file_path);
        } else {
            return Err(FileIndexerError::InvalidFilePath(path));
        }
    }

    /// Indexes one (new) file
    /// Parameter:
    /// - path: The *absolute* path to the file.
    /// - operation: the type of operation
    /// Returns:
    /// - Option<String>: if there's an error
    pub async fn index(&mut self, path: String, operation: FileOperationNeeded) -> Option<String> {
        let content = extract(path.clone(), self.config.max_file_size).await;

        let mut errors = Vec::new();

        self.process_result(
            &mut errors,
            Path::new(&path).to_path_buf(),
            operation,
            content,
        );

        if errors.first().is_none() {
            let commit_error = self
                .db
                .commit()
                .map_err(|e| FileIndexerError::DatabaseCommitError(e));

            if let Err(error) = commit_error {
                errors.push(error);
            }
        }

        return errors.first().map(|e| e.to_string());
    }

    /// Indexes a single file to the database.
    /// Parameters:
    /// - file: the file to index (as PathBuf)
    /// - db_files: the list of files already in the database.
    /// Returns: an error if the file could not be indexed, otherwise None.
    fn index_one_file(
        &mut self,
        file: &PathBuf,
        db_files: &Vec<Document>,
    ) -> Result<FileOperationNeeded, FileIndexerError> {
        let timestamp = self.extract_mod_date(&file)?;

        let file_path = file.to_string_lossy().to_string();

        let local_file_path = self.to_local_path(file_path.clone())?;

        let same = db_files
            .iter()
            .find(|h| same_path((**h).path.as_str(), local_file_path.as_str()));

        if let Some(same) = same
            && same.modification_date != DateTime::from_timestamp_secs(timestamp)
        {
            // Case: the file is in the database but the modification date has changed, update it
            return Ok(FileOperationNeeded::Update);
        } else if same.is_none() {
            // Case: the file is not in the database, add it
            return Ok(FileOperationNeeded::Add);
        }
        return Ok(FileOperationNeeded::None);
    }

    /// Extracts the modification date of a file.
    /// Parameters:
    /// - file: the file to extract the modification date from.
    /// Returns: the modification date as a unix timestamp, or an error if the file could not be read.
    fn extract_mod_date(&self, file: &PathBuf) -> Result<i64, FileIndexerError> {
        let meta = std::fs::metadata(file).map_err(|e| FileIndexerError::GetFileMetaError(e))?;
        let timestamp: i64;
        let mod_date = meta.modified();

        if let Ok(mod_date) = mod_date {
            let timestamp_op = mod_date.duration_since(UNIX_EPOCH);

            if let Ok(timestamp_op) = timestamp_op {
                timestamp = timestamp_op.as_secs() as i64;
            } else {
                // Welp, time went backwards
                timestamp = chrono::Local::now().timestamp();
                self.write_mod_date(file, timestamp)?;
            }
        } else {
            timestamp = chrono::Local::now().timestamp();
            self.write_mod_date(file, timestamp)?;
        }

        return Ok(timestamp);
    }

    /// Writes the modification date of a file to the database.
    /// Parameters:
    /// - file: the file to write the modification date to.
    /// - timestamp: the modification date as a unix timestamp.
    /// Returns: an error if the file could not be written, otherwise None.
    fn write_mod_date(&self, file: &PathBuf, timestamp: i64) -> Result<(), FileIndexerError> {
        let file =
            File::open(file).map_err(|e| FileIndexerError::OpenFileToWriteModDateError(e))?;

        file.set_modified(std::time::SystemTime::from(
            std::time::UNIX_EPOCH + std::time::Duration::from_secs(timestamp as u64),
        ))
        .map_err(|e| FileIndexerError::OpenFileToWriteModDateError(e))?;

        return Ok(());
    }

    /// Returns: a mutable reference to the database.
    pub fn get_db(&mut self) -> &mut Database {
        return &mut self.db;
    }

    /// Returns: a reference to the database.
    pub fn get_db_ro(&self) -> &Database {
        return &self.db;
    }

    /// Gets the sync folder
    ///
    /// Returns:
    /// - a clone of the folder path (as String)
    pub fn get_folder(&self) -> String {
        return self.folder.clone();
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

    use crate::utils::join_path;

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

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn test_sync_one() {
        let (mut indexer, tempdir, folder_path) = setup();
        generate_test_datas(folder_path.as_str());
        let errors = indexer.sync().unwrap();
        assert!(errors.is_empty());

        let db = indexer.get_db();

        let file1 = db.get(String::from("file1.txt"));
        let file2 = db.get(String::from("file2.txt"));
        let file3 = db.get(String::from("dir1/file2.txt"));

        assert!(file1.is_ok());

        let first_content = String::from(FIRST_FILE_CONTENT);
        assert_eq!(file1.unwrap().content.unwrap(), first_content);

        let second_content = String::from(SECOND_FILE_CONTENT);
        assert_eq!(file2.unwrap().content.unwrap(), second_content);

        assert!(match file3.err().unwrap() {
            DatabaseError::NoResult => true,
            _ => false,
        });

        let _ = tempdir.close();
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn test_sync_two() {
        let (mut indexer, tempdir, folder_path) = setup();
        generate_test_datas(folder_path.as_str());
        let errors = indexer.sync().unwrap();
        assert!(errors.is_empty());

        sleep(Duration::from_secs(2)); // If the files are updated too quickly, the sync may not detect the change

        let path = OsString::from_str(join_path(folder_path.as_str(), "file2.txt").as_str());
        assert!(path.is_ok());
        write_to_file(path.unwrap(), THIRD_FILE_CONTENT);

        let errors = indexer.sync().unwrap();
        println!("{:?}", errors);
        assert!(errors.is_empty());

        let db = indexer.get_db();

        let file1 = db.get(String::from("file1.txt"));
        assert!(file1.is_ok());
        let first_content = String::from(FIRST_FILE_CONTENT);
        assert_eq!(file1.unwrap().content.unwrap(), first_content);

        let file3 = db.get(String::from("file2.txt"));
        assert!(file3.is_ok());
        let third_content = String::from(THIRD_FILE_CONTENT);
        assert_eq!(file3.unwrap().content.unwrap(), third_content);

        let _ = tempdir.close();
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn test_relative_path() {
        let folder = String::from("/tmp/ffsearch-test");
        if !Path::new(&folder).exists() {
            let _ = create_dir(folder.clone());
        } else {
            let _ = remove_dir_all(&folder);
            let _ = create_dir(folder.clone());
        }

        let mut file_indexer = FileIndexer::new(folder.clone(), FileIndexerConfig::new()).unwrap();
        assert!(file_indexer.sync().unwrap().is_empty());
        generate_test_datas(folder.as_str());
        assert!(file_indexer.sync().unwrap().is_empty());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn test_sync_three() {
        let (mut indexer, tempdir, folder_path) = setup();
        generate_test_datas(folder_path.as_str());
        indexer.sync().unwrap();

        let path = OsString::from_str(join_path(folder_path.as_str(), "file1.txt").as_str());
        assert!(path.is_ok());
        let _ = remove_file(path.unwrap());

        indexer.sync().unwrap();

        let db = indexer.get_db();

        let file1 = db.get(String::from("file1.txt"));
        assert!(match file1.clone().err().unwrap() {
            DatabaseError::NoResult => true,
            _ => false,
        });

        let _ = tempdir.close();
    }
}
