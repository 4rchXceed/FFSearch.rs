use std::{fs::File, io::Write, path::Path};

use domain::indexer::config::FileIndexerConfig;
use settings::settings::Settings;
use tempdir::TempDir;

static SAMPLE_SETTINGS: &str = "
    [FILE_INDEXER.0]
    FOLDER=/randompath
    THREADS=12
    MAX_SINGLE_FILE_SIZE=1000000

    [DATABASE.0]
    MAX_SIZE=100000000
    DB_FOLDER_PREFIX=.db

    [RAG]
    NUMBER_RESULTS=10
    EMBEDDING_MODEL=mikunano:latest

    [AI]
    MODEL=minimodel:1b
    SYSTEM_PROMPT=You are Hatsune Miku
    OLLAMA_THINK_ENABLED=TRUE
    LLM_PROVIDER=OPENAI
    API_KEY=Miku-393939

    [MCP]
    LISTEN_AT=127.0.0.1:3939
    ";

pub fn write_sample_ini() -> (String, TempDir) {
    let (dir, ini_path) = generate_ini();

    let mut file = File::create(Path::new(&ini_path)).expect("Failed to create test ini cfg");

    file.write(SAMPLE_SETTINGS.as_bytes())
        .expect("Failed to write test ini cfg");

    return (ini_path, dir);
}

fn generate_ini() -> (TempDir, String) {
    let dir = TempDir::new(".ffsearch-test").expect("Temp dir creation failed");

    let dir_path = dir.path();

    let ini_path = dir_path.join("test_cfg.ini").to_string_lossy().to_string();

    return (dir, ini_path);
}

#[test]
fn test_save_config_normal() {
    // ---------------------------------------
    // Tear up
    // ---------------------------------------
    // Generate the ini file
    let (_dir, ini_path) = generate_ini();

    // Create the settings class
    let mut settings = Settings::default();

    settings.set_file(ini_path.clone());

    let mut file_indexer_01 = FileIndexerConfig::new();

    file_indexer_01.number_of_threads = 1;
    file_indexer_01.max_file_size = 2;

    let mut file_indexer_02 = FileIndexerConfig::new();

    file_indexer_02.db_config.database_folder_prefix = String::from(".data");
    file_indexer_02.db_config.max_db_size = 3;

    settings
        .file_indexers
        .insert(String::from("/randompath"), file_indexer_01);

    settings
        .file_indexers
        .insert(String::from("/randompath2"), file_indexer_02);

    // ---------------------------------------
    // Test save()
    // ---------------------------------------
    settings.save().expect("Failed to save settings");

    // ---------------------------------------
    // load() after save()
    // ---------------------------------------
    let settings_new = Settings::load(ini_path).expect("Failed to load new settings");

    let file_indexer_01_new = settings_new
        .file_indexers
        .get("/randompath")
        .expect("Failed to get /randompath settings");

    let file_indexer_02_new = settings_new
        .file_indexers
        .get("/randompath2")
        .expect("Failed to get /randompath2 settings");

    // Checks
    assert_eq!(file_indexer_01_new.number_of_threads, 1);
    assert_eq!(file_indexer_01_new.max_file_size, 2);
    assert_eq!(
        file_indexer_02_new.db_config.database_folder_prefix,
        ".data"
    );
    assert_eq!(file_indexer_02_new.db_config.max_db_size, 3);
}

#[test]
fn test_load_config_normal() {
    // ---------------------------------------
    // Tear up
    // ---------------------------------------
    // Generate the ini file
    let (ini, _dir) = write_sample_ini();

    // ---------------------------------------
    // Tests load() for the file indexer settings
    // ---------------------------------------
    // Loads
    let config = Settings::load(ini).expect("Failed to read config: ");

    // Checks

    let file_indexer = config
        .file_indexers
        .get("/randompath")
        .expect("Failed to get /randompath");

    assert_eq!(file_indexer.number_of_threads, 12);
    assert_eq!(file_indexer.max_file_size, 1000000);
    assert_eq!(file_indexer.db_config.max_db_size, 100000000);
    assert_eq!(
        file_indexer.db_config.database_folder_prefix,
        String::from(".db")
    );
}

#[test]
#[cfg(feature = "ai")]
fn test_save_config_ai() {
    // ---------------------------------------
    // Tear up
    // ---------------------------------------

    // Generates the ini
    let (_dir, ini_path) = generate_ini();

    let mut settings = Settings::default();

    settings.set_file(ini_path.clone());

    settings.ai.model = String::from("miku:4000b");
    settings.ai.preamble = String::from("Only answer in morse code");
    settings.ai.provider = String::from("openai");
    settings.ai.think = true;
    settings.ai.api_key = String::from("xyz");

    // ---------------------------------------
    // Tests load() after save() (AI settings)
    // ---------------------------------------

    // Save
    settings.save().expect("Failed to save settings");

    // Load
    let settings_new = Settings::load(ini_path).expect("Failed to load new settings");

    // Checks
    assert_eq!(settings_new.ai.model, "miku:4000b");
    assert_eq!(settings_new.ai.preamble, "Only answer in morse code");
    assert_eq!(settings_new.ai.provider, "openai");
    assert_eq!(settings_new.ai.think, true);
    assert_eq!(settings_new.ai.api_key, "xyz");
}

#[test]
#[cfg(feature = "ai")]
fn test_load_config_ai() {
    // ---------------------------------------
    // Tear up
    // ---------------------------------------

    // Writes the ini
    let (ini, _dir) = write_sample_ini();

    // ---------------------------------------
    // Tests load() (AI Settings)
    // ---------------------------------------

    let config = Settings::load(ini).expect("Failed to read config: ");

    assert_eq!(config.ai.model, "minimodel:1b");
    assert_eq!(config.ai.preamble, "You are Hatsune Miku");
    assert_eq!(config.ai.think, true);
    assert_eq!(config.ai.provider, "openai");
    assert_eq!(config.ai.api_key, "Miku-393939");
}

#[test]
#[cfg(feature = "ai")]
fn test_save_config_rag() {
    // ---------------------------------------
    // Tear up
    // ---------------------------------------

    // Generates the ini
    let (_dir, ini_path) = generate_ini();

    let mut settings = Settings::default();

    settings.set_file(ini_path.clone());

    settings.ai.rag_config.top_results = 39;
    settings.ai.rag_config.embedding_model = String::from("mikusuper:1b");

    // ---------------------------------------
    // Tests load() after save() (RAG Settings)
    // ---------------------------------------

    settings.save().expect("Failed to save settings");

    let settings_new = Settings::load(ini_path).expect("Failed to load new settings");

    assert_eq!(settings_new.ai.rag_config.top_results, 39);
    assert_eq!(settings_new.ai.rag_config.embedding_model, "mikusuper:1b");
}

#[test]
#[cfg(feature = "ai")]
fn test_load_config_rag() {
    // ---------------------------------------
    // Tear up
    // ---------------------------------------

    let (ini, dir) = write_sample_ini();

    // ---------------------------------------
    // Tests load() (RAG Settings)
    // ---------------------------------------

    let config = Settings::load(ini).expect("Failed to read config: ");

    assert_eq!(config.ai.rag_config.top_results, 10);
    assert_eq!(config.ai.rag_config.embedding_model, "mikunano:latest");

    dir.close().expect("Failed to remove temp dir");
}

#[test]
#[cfg(feature = "mcp_server")]
fn test_save_config_mcp() {
    // ---------------------------------------
    // Tear up
    // ---------------------------------------

    let (dir, ini_path) = generate_ini();

    let mut settings = Settings::default();

    settings.set_file(ini_path.clone());

    settings.mcp.listen_addr = String::from("localhost:3333");

    // ---------------------------------------
    // Tests load() after save() (MCP Settings)
    // ---------------------------------------

    settings.save().expect("Failed to save settings");

    let settings_new = Settings::load(ini_path).expect("Failed to load new settings");

    assert_eq!(settings_new.mcp.listen_addr, "localhost:3333");

    dir.close().expect("Failed to remove temp dir");
}

#[test]
#[cfg(feature = "mcp_server")]
fn test_load_config_mcp() {
    // ---------------------------------------
    // Tear up
    // ---------------------------------------

    let (ini, dir) = write_sample_ini();

    // ---------------------------------------
    // Tests load() (MCP Settings)
    // ---------------------------------------

    let config = Settings::load(ini).expect("Failed to read config: ");

    assert_eq!(config.mcp.listen_addr, "127.0.0.1:3939");

    dir.close().expect("Failed to remove temp dir");
}
