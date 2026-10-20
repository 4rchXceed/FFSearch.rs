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

    let mut file = File::create(file_path.clone())
        .await
        .expect("Failed to create file");

    file.write_all(b"Hatsuneeeee")
        .await
        .expect("Failed to write file");

    file.flush().await.expect("Failed to flush file");
    drop(file);

    let file_extractor = XbergFileExtractor::new(&FileIndexerConfig::new());

    // ---------------------------------------
    // Tests extract() (text file)
    // ---------------------------------------

    let result = file_extractor
        .extract(file_path.to_string_lossy().to_string())
        .await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap().unwrap(), "Hatsuneeeee");
}

#[tokio::test]
async fn test_extract_docx() {
    // ---------------------------------------
    // Tear up
    // ---------------------------------------

    let temp_dir = TempDir::new("ffsearch_test").expect("Temp dir creation failed");
    let path = temp_dir.path();

    let file_path = path.join("test.docx");

    let mut file = File::create(file_path.clone())
        .await
        .expect("Failed to create file");

    file.write_all(include_bytes!("./sample_datas/test_text_extractor.docx"))
        .await
        .expect("Failed to write to file");

    file.flush().await.expect("Failed to flush file");
    drop(file);

    let file_extractor = XbergFileExtractor::new(&FileIndexerConfig::new());

    // ---------------------------------------
    // Tests extract (binary file)
    // ---------------------------------------

    let result = file_extractor
        .extract(file_path.to_string_lossy().to_string())
        .await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap().unwrap(), "Hatsune\u{a0}Miku~");
}
