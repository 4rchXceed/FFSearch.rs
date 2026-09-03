use std::{fs::File, io::Read};

use xberg::{ExtractInput, ExtractionConfig, detect_mime_type_from_bytes};

/// Extracts text from a file at the given path.
/// Returns the extracted text as a string, or an error if extraction fails.
/// Parameters:
/// - path: the path to the file to extract text from
pub async fn extract(path: String, max_bytes: usize) -> Result<String, String> {
    let file = File::open(path);
    if file.is_err() {
        return Err(String::from("File not found"));
    }
    let mut file = file.unwrap();
    let metadata = file.metadata();
    if metadata.is_ok() {
        if metadata.unwrap().len() > max_bytes as u64 {
            return Err(String::from("File too large"));
        }
    } else {
        return Err(String::from("Could not read file metadata"));
    }
    let mut content_bin = Vec::new();
    file.read_to_end(&mut content_bin).unwrap();
    let content = String::from_utf8(content_bin.clone());
    if content.is_ok() {
        return Ok(content.unwrap());
    }
    let result = xberg::extract(
        ExtractInput::from_bytes(
            content_bin.as_slice(),
            detect_mime_type_from_bytes(&content_bin)
                .map_err(|e| format!("Couldn't detect file MIME type: {}", e.to_string()))?, // TODO
            None,
        ),
        &ExtractionConfig::default(),
    )
    .await;
    if result.is_ok() {
        let result = result.unwrap();
        return Ok(result
            .results
            .iter()
            .map(|e| e.content.clone())
            .collect::<Vec<String>>()
            .join("\n"));
    } else {
        return Err(result.unwrap_err().to_string());
    }
}

#[cfg(test)]
mod tests {
    use tempdir::TempDir;

    use crate::utils::join_path;

    use super::*;

    fn setup() -> (TempDir, String) {
        let temp_dir = TempDir::new("test").unwrap();
        let file_path = temp_dir.path().to_string_lossy().into_owned();
        return (temp_dir, file_path);
    }

    fn write_test_file(temp_dir: &TempDir, file_name: &str, content: &[u8]) -> String {
        let file_dir = temp_dir.path().to_string_lossy().into_owned();
        let file_name = join_path(&file_dir, file_name);
        std::fs::write(file_name.clone(), content).unwrap();
        return file_name;
    }

    #[tokio::test]
    async fn test_extract() {
        let (temp_dir, _) = setup();

        let file_name = write_test_file(&temp_dir, "test.txt", b"Hatsuneeeee");
        let result = extract(file_name, 1024 * 1024).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Hatsuneeeee");
    }

    #[tokio::test]
    async fn test_extract_docx() {
        let (temp_dir, _) = setup();

        let file_name = write_test_file(
            &temp_dir,
            "test.docx",
            include_bytes!("../sample_datas/test_text_extractor.docx"),
        );
        let result = extract(file_name, 1024 * 1024).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Hatsune\u{a0}Miku~");
    }
}
