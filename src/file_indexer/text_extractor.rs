use extractous::{Extractor, PdfParserConfig, TesseractOcrConfig};

pub struct TextExtractor {
    extractor: Extractor,
}

impl TextExtractor {
    pub fn new(max_length: usize, enable_ocr: bool, ocr_language: &str) -> Self {
        if enable_ocr {
            Self {
                extractor: Extractor::new()
                    .set_extract_string_max_length(max_length as i32)
                    .set_ocr_config(TesseractOcrConfig::new().set_language(ocr_language))
                    .set_pdf_config(
                        PdfParserConfig::new()
                            .set_ocr_strategy(extractous::PdfOcrStrategy::OCR_ONLY),
                    ),
            }
        } else {
            Self {
                extractor: Extractor::new().set_extract_string_max_length(max_length as i32),
            }
        }
    }

    pub fn extract(&self, path: String) -> Result<String, extractous::Error> {
        let result = self.extractor.extract_file_to_string(path.as_str());
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
        let extractor = TextExtractor::new(1024, ocr, "eng");
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
