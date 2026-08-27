use std::path::PathBuf;

use walkdir::{DirEntry, WalkDir};

use crate::{not_null_check, searcher::database::Database};

#[derive(Debug)]
pub enum FileIndexerError {}

pub struct FileIndexer {
    folder: String,
    db: Database,
}

impl FileIndexer {
    pub fn new(folder: String, database: Database) -> Self {
        return Self {
            folder: folder,
            db: database,
        };
    }

    fn is_correct_filename(&self, file: &DirEntry) -> bool {
        let db_folder = self.db.get_config().database_folder_prefix;
        not_null_check!(file, file.file_name().to_str(), false);
        if file.starts_with(".") || file == db_folder {
            return false;
        } else {
            return true;
        }
    }

    pub fn sync(&mut self) -> Result<(), FileIndexerError> {
        let all_db_files = self.db.get_all();

        let files: Vec<PathBuf> = WalkDir::new(self.folder)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| self.is_correct_filename(e))
            .map(|x| x.path().to_owned())
            .collect();

        for file in files {}
    }

    pub fn get_db(&mut self) -> &mut Database {
        return &mut self.db;
    }
}

#[cfg(test)]
mod tests {
    use std::{
        ffi::OsString,
        fs::{File, create_dir, remove_file},
        io::Write,
        str::FromStr,
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

        let db = Database::new(folder.clone(), DatabaseConfig::new());
        assert!(db.is_ok());
        let db = db.unwrap();

        let file_indexer = FileIndexer::new(folder.clone(), db);

        return (file_indexer, temp_dir, folder);
    }

    fn test_sync_one() {
        let (mut indexer, tempdir, folder_path) = setup();
        generate_test_datas(folder_path.as_str());
        indexer.sync();

        let db = indexer.get_db();

        let file1 = db.get(String::from("file1.txt"));
        let file2 = db.get(String::from("dir1/file2.txt"));

        assert!(file1.is_some());
        assert!(file2.is_some());

        assert_eq!(file1.unwrap().content, String::from(FIRST_FILE_CONTENT));
        assert_eq!(file2.unwrap().content, String::from(SECOND_FILE_CONTENT));

        tempdir.close();
    }

    fn test_sync_two() {
        let (mut indexer, tempdir, folder_path) = setup();
        generate_test_datas(folder_path.as_str());
        indexer.sync();

        let path = OsString::from_str(join_path(folder_path.as_str(), "file2.txt").as_str());
        assert!(path.is_ok());
        write_to_file(path.unwrap(), THIRD_FILE_CONTENT);

        indexer.sync();

        let db = indexer.get_db();

        let file1 = db.get(String::from("file1.txt"));
        assert!(file1.is_some());
        assert_eq!(file1.unwrap().content, String::from(FIRST_FILE_CONTENT));

        let file3 = db.get(String::from("file2.txt"));
        assert!(file3.is_some());
        assert_eq!(file3.unwrap().content, String::from(THIRD_FILE_CONTENT));

        tempdir.close();
    }

    fn test_sync_three() {
        let (mut indexer, tempdir, folder_path) = setup();
        generate_test_datas(folder_path.as_str());
        indexer.sync();

        let path = OsString::from_str(join_path(folder_path.as_str(), "file1.txt").as_str());
        assert!(path.is_ok());
        remove_file(path.unwrap());

        indexer.sync();

        let db = indexer.get_db();

        let file1 = db.get(String::from("file1.txt"));
        assert!(file1.is_none());

        tempdir.close();
    }
}
