use std::{fs::File, io::Read};

use extractous::{Extractor, PdfParserConfig, TesseractOcrConfig};

/// The text extractor is a part of the file indexer component.
/// The text extractor is responsible for extracting text pretty much any file type.
/// It supports OCR, DOCX, ... (see extractous documentation)
pub struct TextExtractor {
    /// The extractor instance used to extract text from files.
    extractor: Extractor,
    max_bytes: usize,
}

impl TextExtractor {
    /// Creates a new text extractor instance.
    /// Parameters:
    /// - max_length: the maximum length in chars of the extracted text
    /// - enable_ocr: whether to enable OCR for the extractor
    /// - ocr_language: the language to use for OCR (if enabled)
    pub fn new(max_length: usize, enable_ocr: bool, ocr_language: &str, max_bytes: usize) -> Self {
        if enable_ocr {
            Self {
                extractor: Extractor::new()
                    .set_extract_string_max_length(max_length as i32)
                    .set_ocr_config(TesseractOcrConfig::new().set_language(ocr_language))
                    .set_pdf_config(
                        PdfParserConfig::new()
                            .set_ocr_strategy(extractous::PdfOcrStrategy::OCR_ONLY),
                    ),
                max_bytes: max_bytes,
            }
        } else {
            Self {
                extractor: Extractor::new().set_extract_string_max_length(max_length as i32),
                max_bytes: max_bytes,
            }
        }
    }

    /// Extracts text from a file at the given path.
    /// Returns the extracted text as a string, or an error if extraction fails.
    /// Parameters:
    /// - path: the path to the file to extract text from
    pub fn extract(&self, path: String) -> Result<String, extractous::Error> {
        let file = File::open(path);
        if file.is_err() {
            return Err(extractous::Error::IoError(String::from("File not found")));
        }
        let mut file = file.unwrap();
        let metadata = file.metadata();
        if metadata.is_ok() {
            if metadata.unwrap().len() > self.max_bytes as u64 {
                return Err(extractous::Error::IoError(String::from("File too large")));
            }
        } else {
            return Err(extractous::Error::IoError(String::from(
                "Could not read file metadata",
            )));
        }
        let mut content_bin = Vec::new();
        file.read_to_end(&mut content_bin).unwrap();
        let content = String::from_utf8(content_bin.clone());
        if content.is_ok() {
            return Ok(content.unwrap());
        }
        let result = self
            .extractor
            .extract_bytes_to_string(content_bin.as_slice());
        if result.is_ok() {
            let result = result.unwrap();
            return Ok(result.0);
        } else {
            return Err(result.unwrap_err());
        }
    }
}

#[cfg(test)]
mod tests {
    use tempdir::TempDir;

    use crate::utils::join_path;

    use super::*;

    fn setup(ocr: bool) -> (TextExtractor, TempDir, String) {
        let extractor = TextExtractor::new(1024, ocr, "eng", 1024 * 1024); // 1MB
        let temp_dir = TempDir::new("test").unwrap();
        let file_path = temp_dir.path().to_string_lossy().into_owned();
        return (extractor, temp_dir, file_path);
    }

    fn write_test_file(temp_dir: &TempDir, file_name: &str, content: &[u8]) -> String {
        let file_dir = temp_dir.path().to_string_lossy().into_owned();
        let file_name = join_path(&file_dir, file_name);
        std::fs::write(file_name.clone(), content).unwrap();
        return file_name;
    }

    #[test]
    fn test_extract() {
        let (extractor, temp_dir, _) = setup(false);

        let file_name = write_test_file(&temp_dir, "test.txt", b"Hatsuneeeee");
        let result = extractor.extract(file_name);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Hatsuneeeee\n");
    }

    #[test]
    fn test_extract_docx() {
        let (extractor, temp_dir, _) = setup(false);

        let file_name = write_test_file(
            &temp_dir,
            "test.docx",
            include_bytes!("../sample_datas/test_text_extractor.docx"),
        );
        let result = extractor.extract(file_name);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Hatsune\u{a0}Miku~\n");
    }
}
