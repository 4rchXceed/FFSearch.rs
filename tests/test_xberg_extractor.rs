use application::indexer::traits::text_extractor::TextExtractor;
use domain::indexer::config::FileIndexerConfig;
use infrastructure::indexer::xberg::extractor::XbergFileExtractor;
use tempdir::TempDir;
use tokio::{fs::File, io::AsyncWriteExt};

#[tokio::test]
async fn test_text_extractor_palin_txt() {
    // ---------------------------------------
    // Tear up
    // ---------------------------------------

    let temp_dir = TempDir::new("ffsearch_test").expect("Temp dir creation failed");
    let path = temp_dir.path();

    let file_path = path.join("test.txt");

    File::create(file_path.clone())
        .await
        .expect("Failed to create file")
        .write(b"Hatsuneeeee")
        .await
        .expect("Failed to write file");

    let file_extractor = XbergFileExtractor::new(&FileIndexerConfig::new());

    // ---------------------------------------
    // Tests extract() (text file)
    // ---------------------------------------

    let result = file_extractor
        .extract(file_path.to_string_lossy().to_string())
        .await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "Hatsuneeeee");
}

#[tokio::test]
async fn test_extract_docx() {
    // ---------------------------------------
    // Tear up
    // ---------------------------------------

    let temp_dir = TempDir::new("ffsearch_test").expect("Temp dir creation failed");
    let path = temp_dir.path();

    let file_path = path.join("test.docx");

    File::create(file_path.clone())
        .await
        .expect("Failed to create file")
        .write(include_bytes!("./sample_datas/test_text_extractor.docx"))
        .await
        .expect("Failed to write to file");

    let file_extractor = XbergFileExtractor::new(&FileIndexerConfig::new());

    // ---------------------------------------
    // Tests extract (binary file)
    // ---------------------------------------

    let result = file_extractor
        .extract(file_path.to_string_lossy().to_string())
        .await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "Hatsune\u{a0}Miku~");
}
