use std::{fs::File, io::Read};

use xberg::{ExtractInput, ExtractionConfig, detect_mime_type_from_bytes};

/// Extracts text from a file at the given path.
/// Returns the extracted text as a string, or an error if extraction fails.
/// Parameters:
/// - path: the path to the file to extract text from
pub async fn extract(path: String, max_bytes: usize) -> Result<String, String> {
    let mut file =
        File::open(path).map_err(|e| format!("Failed to open file: {}", e.to_string()))?;

    let metadata = file
        .metadata()
        .map_err(|e| format!("Could not read file metadata: {}", e))?;

    if metadata.len() > max_bytes as u64 {
        return Err(String::from("File too large"));
    }

    let mut content_bin = Vec::new();

    file.read_to_end(&mut content_bin)
        .map_err(|e| e.to_string())?;

    let mime_type = detect_mime_type_from_bytes(&content_bin)
        .map_err(|e| format!("Couldn't detect file MIME type: {}", e.to_string()))?;

    if mime_type == "application/pdf" {
        return Ok(pdf_extract::extract_text_from_mem(&content_bin).map_err(|e| e.to_string())?);
    } else {
        let result = xberg::extract(
            ExtractInput::from_bytes(content_bin.as_slice(), mime_type, None),
            &ExtractionConfig::default(),
        )
        .await
        .map_err(|e| e.to_string())?;

        return Ok(result
            .results
            .iter()
            .map(|e| e.content.clone())
            .collect::<Vec<String>>()
            .join("\n"));
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
        // ---------------------------------------
        // Tear up
        // ---------------------------------------

        let (temp_dir, _) = setup();

        let file_name = write_test_file(&temp_dir, "test.txt", b"Hatsuneeeee");

        // ---------------------------------------
        // Tests extract() (text file)
        // ---------------------------------------

        let result = extract(file_name, 1024 * 1024).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Hatsuneeeee");
    }

    #[tokio::test]
    async fn test_extract_docx() {
        // ---------------------------------------
        // Tear up
        // ---------------------------------------

        let (temp_dir, _) = setup();

        let file_name = write_test_file(
            &temp_dir,
            "test.docx",
            include_bytes!("../sample_datas/test_text_extractor.docx"),
        );

        // ---------------------------------------
        // Tests extract (binary file)
        // ---------------------------------------

        let result = extract(file_name, 1024 * 1024).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Hatsune\u{a0}Miku~");
    }
}
