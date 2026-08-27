/// The default maximum length of a single file in chars.
static DEFAULT_FILE_MAX_LENGTH: usize = 10_000;

/// The configuration for the file indexer.
/// These are the config properties for the file indexer:
/// - one_file_max_length: the maximum length of a single file in (chars, I guess? See extractous's config)
/// - enable_ocr: whether to enable OCR for the file indexer
/// - ocr_lang: the language to use for OCR
pub struct FileIndexerConfig {
    /// The maximum length of a single file in (chars, I guess? See extractous's config)
    pub one_file_max_length: usize,
    /// Whether to enable OCR for the file indexer
    pub enable_ocr: bool,
    /// The language to use for OCR
    pub ocr_lang: String,
}

impl FileIndexerConfig {
    pub fn new() -> Self {
        Self {
            one_file_max_length: DEFAULT_FILE_MAX_LENGTH,
            enable_ocr: false,
            ocr_lang: String::new(),
        }
    }
}
