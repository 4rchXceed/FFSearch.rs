use std::{fs::File, io::Read};

use application::indexer::traits::text_extractor::TextExtractor;
use async_trait::async_trait;
use domain::indexer::config::FileIndexerConfig;
use xberg::{ExtractInput, ExtractionConfig, detect_mime_type_from_bytes};

pub struct XbergFileExtractor {
    max_single_file_size: u64,
}

impl XbergFileExtractor {
    pub fn new(config: &FileIndexerConfig) -> Self {
        return Self {
            max_single_file_size: config.max_file_size as u64,
        };
    }
}

#[async_trait]
impl TextExtractor for XbergFileExtractor {
    async fn extract(&self, path: String) -> Result<Option<String>, String> {
        let mut file =
            File::open(path).map_err(|e| format!("Failed to open file: {}", e.to_string()))?;

        let metadata = file
            .metadata()
            .map_err(|e| format!("Could not read file metadata: {}", e))?;

        if metadata.len() > self.max_single_file_size {
            return Ok(None);
        }

        let mut content_bin = Vec::new();

        file.read_to_end(&mut content_bin)
            .map_err(|e| e.to_string())?;

        let mime_type = match detect_mime_type_from_bytes(&content_bin) {
            Ok(mime) => mime,
            Err(_) => return Ok(None),
        };

        if mime_type == "application/pdf" {
            return Ok(Some(
                pdf_extract::extract_text_from_mem(&content_bin).map_err(|e| e.to_string())?,
            ));
        } else {
            let result = xberg::extract(
                ExtractInput::from_bytes(content_bin.as_slice(), mime_type, None),
                &ExtractionConfig::default(),
            )
            .await
            .map_err(|e| e.to_string())?;

            return Ok(Some(
                result
                    .results
                    .iter()
                    .map(|e| e.content.clone())
                    .collect::<Vec<String>>()
                    .join("\n"),
            ));
        }
    }
}
