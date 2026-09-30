use std::{
    path::{MAIN_SEPARATOR_STR, PathBuf},
    sync::Arc,
};

use domain::{
    indexer::{error::FileIndexerError, file_operation::FileOperationNeeded},
    searcher::document::Document,
};
#[cfg(not(test))]
use tqdm::tqdm;
use utils::{same_path, to_local_path, trim_custom};

use crate::{
    indexer::traits::{file_indexer::FileIndexerProvider, text_extractor::TextExtractor},
    searcher::{actions::get::DatabaseGet, traits::database::DatabaseProvider},
};

pub struct FileIndexerSync {
    indexer: Arc<dyn FileIndexerProvider>,
    database: Arc<dyn DatabaseProvider>,
    extractor: Arc<dyn TextExtractor>,
    folder: String,
}

impl FileIndexerSync {
    /// Creates a SyncFolder
    ///
    /// Parameters:
    /// - indexer: the file indexer to use
    /// - database: the database to use
    /// - extractor: the text extractor to use
    /// - config: the configuration to use
    ///
    /// Returns:
    /// - A SyncFolder
    pub fn new(
        indexer: Arc<dyn FileIndexerProvider>,
        database: Arc<dyn DatabaseProvider>,
        extractor: Arc<dyn TextExtractor>,
        folder: String,
    ) -> Self {
        return Self {
            indexer,
            database,
            extractor,
            folder,
        };
    }

    /// Local function to check if a file needs to be added, updated or ignored.
    ///
    /// Parameters:
    /// - file: the file to check.
    /// - db_files: the files in the database.
    /// - root: the "root" index folder
    ///
    /// Returns:
    /// - FileOperationNeeded: the operation needed for the file.
    fn check_one_file(
        &self,
        file: &PathBuf,
        db_files: &Vec<Document>,
        root: String,
    ) -> Result<FileOperationNeeded, FileIndexerError> {
        let timestamp = self.indexer.extract_modified_time(file)?;

        let path = file.to_string_lossy().to_string();

        let local_file_path = to_local_path(path.clone(), root)
            .ok_or(FileIndexerError::InvalidFilePath(path.clone()))?;

        let same = db_files
            .iter()
            .find(|h| same_path((**h).path.as_str(), local_file_path.as_str()));

        if let Some(same) = same
            && same.modification_date != timestamp
        {
            // Case: the file is in the database but the modification date has changed, update it
            return Ok(FileOperationNeeded::Update);
        } else if same.is_none() {
            // Case: the file is not in the database, add it
            return Ok(FileOperationNeeded::Add);
        }
        return Ok(FileOperationNeeded::None);
    }

    /// Syncs the files in the indexer with the database.
    /// Uses a multi-threaded approach to make the process faster
    ///
    /// Parameters:
    /// - folder: the "root" index folder
    ///
    /// Returns:
    /// - A list of errors that occurred during the sync process.
    pub async fn sync(&self) -> Result<Vec<FileIndexerError>, FileIndexerError> {
        let real_files = self.indexer.get_all_files(self.folder.clone());

        let db_files = DatabaseGet::new(self.database.clone())
            .get_all_without_content()
            .map_err(|e| FileIndexerError::GetAllDocumentsFailed(e.to_string()))?;

        let mut all_errors: Vec<FileIndexerError> = Vec::new();
        let mut all_files_to_process = Vec::new();

        #[cfg(not(test))]
        let iterator = tqdm(real_files.iter()).desc(Some(String::from("Checking every file")));
        #[cfg(test)]
        let iterator = real_files.iter();

        for file in iterator {
            let result = self.check_one_file(file, &db_files, self.folder.clone());

            if let Err(error) = result {
                all_errors.push(error);
            } else if let Ok(result) = result {
                match result {
                    FileOperationNeeded::None => {}
                    _ => all_files_to_process.push((file.clone(), result)),
                }
            }
        }

        self.threaded_sync(&mut all_errors, all_files_to_process)
            .await?;

        for db_file in db_files {
            if !real_files
                .iter()
                .filter_map(|f| {
                    f.strip_prefix(self.folder.as_str())
                        .map_or(None, |e| Some(e.to_string_lossy().to_string()))
                })
                .map(|f| trim_custom(f, MAIN_SEPARATOR_STR))
                .any(|f| f == db_file.path)
            {
                let result = self.database.remove_document(db_file.path).await;

                if let Err(error) = result {
                    all_errors.push(FileIndexerError::FailedToRemoveDocument(error.to_string()));
                }
            }
        }

        self.database
            .commit()
            .await
            .map_err(|e| FileIndexerError::DatabaseCommitError(e.to_string()))?;

        return Ok(all_errors);
    }

    /// Local function to process the files in a multi-threaded way.
    ///
    /// Parameters:
    /// - all_errors: a mut to all errors that occurred during the sync process.
    /// - all_files_to_process: a list of files to process.
    /// - folder: the "root" index folder
    async fn threaded_sync(
        &self,
        all_errors: &mut Vec<FileIndexerError>,
        all_files_to_process: Vec<(PathBuf, FileOperationNeeded)>,
    ) -> Result<(), FileIndexerError> {
        let config = self.indexer.get_config();

        let number_of_threads = config.number_of_threads;

        let mut threads = Vec::new();

        for th in 0..number_of_threads {
            let thread_process_files = all_files_to_process
                .iter()
                .enumerate()
                .filter(|(i, _)| i % number_of_threads == th)
                .map(|(_, d)| (d.0.clone(), d.1.clone()))
                .collect::<Vec<(PathBuf, FileOperationNeeded)>>()
                .clone();

            let extractor = self.extractor.clone();

            threads.push(tokio::spawn(async move {
                let mut results = Vec::new();

                #[cfg(not(test))]
                let iterator = tqdm(thread_process_files).desc(Some(format!("Thread: {}", th)));
                #[cfg(test)]
                let iterator = thread_process_files;

                for (file, operation) in iterator {
                    results.push((
                        file.clone(),
                        operation,
                        extractor.extract(file.to_string_lossy().to_string()).await,
                    ));
                }

                return results;
            }));
        }

        for thread in threads {
            let results = thread.await;

            if let Ok(results) = results {
                for (file, operation, result) in results {
                    if result.is_ok() {
                        self.process_result(
                            all_errors,
                            file,
                            operation,
                            result,
                            self.folder.clone(),
                        )
                        .await;
                    } else if let Err(error) = result {
                        all_errors.push(FileIndexerError::GetFileContentError(error));
                    }
                }
            } else {
                all_errors.push(FileIndexerError::ThreadError);
            }
        }

        return Ok(());
    }

    /// Local function to process the result of a file operation.
    ///
    /// Parameters:
    /// - all_errors: a mut to all errors that occurred during the sync process.
    /// - file: the file to process.
    /// - operation: the operation needed for the file.
    /// - file_content_op: the result of the file content extraction.
    /// - root: the "root" index folder
    async fn process_result(
        &self,
        all_errors: &mut Vec<FileIndexerError>,
        file: PathBuf,
        operation: FileOperationNeeded,
        file_content_op: Result<String, String>,
        root: String,
    ) {
        if let Ok(content) = file_content_op {
            let file_path = file.to_string_lossy().to_string();

            let local_file_path = to_local_path(file_path.clone(), root);

            if let Some(local_file_path) = local_file_path {
                let timestamp = self.indexer.extract_modified_time(&file);

                if let Ok(timestamp) = timestamp {
                    self.apply_file_changes(
                        timestamp,
                        local_file_path,
                        content,
                        all_errors,
                        operation,
                    )
                    .await;
                } else if let Err(error) = timestamp {
                    all_errors.push(error);
                }
            } else {
                all_errors.push(FileIndexerError::InvalidFilePath(file_path));
            }
        } else if let Err(error) = file_content_op {
            all_errors.push(FileIndexerError::GetFileContentError(error));
        }
    }

    /// Applies the file changes to the database
    ///
    /// Parameters:
    /// - timestamp: the timestamp of the file
    /// - local_file_path: the local path of the file
    /// - content: the content of the file
    /// - all_errors: a mut to all errors that occurred during the sync process.
    /// - operation: the operation needed for the file.
    async fn apply_file_changes(
        &self,
        timestamp: i64,
        local_file_path: String,
        content: String,
        all_errors: &mut Vec<FileIndexerError>,
        operation: FileOperationNeeded,
    ) {
        match operation {
            FileOperationNeeded::Add => {
                let result = self
                    .database
                    .add_document(Document::new(local_file_path, Some(content), timestamp))
                    .await;
                if let Err(error) = result {
                    all_errors.push(FileIndexerError::CouldNotAddDocument(error.to_string()));
                }
            }
            FileOperationNeeded::Update => {
                let result = self
                    .database
                    .update_document(
                        local_file_path.clone(),
                        Document::new(local_file_path, Some(content), timestamp),
                    )
                    .await;
                if let Err(error) = result {
                    all_errors.push(FileIndexerError::CouldNotUpdateDocument(error.to_string()));
                }
            }
            FileOperationNeeded::None => {}
        };
    }
}
