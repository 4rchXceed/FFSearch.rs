use std::collections::HashMap;
use std::path::Path;

#[cfg(feature = "ai")]
use crate::ai::DEFAULT_PREAMBLE;
#[cfg(feature = "ai")]
use crate::ai::{
    config::{AgentConfig, AgentProvider},
    rag::config::RagConfig,
};
#[cfg(feature = "mcp")]
use crate::mcp::config::DEFAULT_LISTEN_ADDR;
#[cfg(feature = "mcp")]
use crate::mcp::config::McpServerConfig;
use crate::{
    file_indexer::config::FileIndexerConfig,
    searcher::config::{DEFAULT_DATABASE_SIZE, DEFAULT_SAVE_FOLDER_PREFIX, DatabaseConfig},
};

use ini::{ini, iniwrite};

static DEFAULT_MODEL: &str = "llama3.1:8b";

static DEFAULT_EMBEDDING_MODEL: &str = "embeddinggemma:latest";

pub struct Settings {
    pub file_indexer: FileIndexerConfig,
    file: String,
    #[cfg(feature = "ai")]
    pub ai: AgentConfig,
    #[cfg(feature = "mcp")]
    pub mcp: McpServerConfig,
}

impl Settings {
    pub fn load(settings_file: String) -> Result<Self, String> {
        let parsed = ini!(safe settings_file.clone())?;

        let file_indexer_config = Settings::process_file_indexer_section(&parsed);

        #[cfg(feature = "ai")]
        let ai_config = Settings::process_ai_section(&parsed);

        #[cfg(feature = "mcp")]
        let mcp_config = Settings::process_mcp_section(&parsed);

        return Ok(Self {
            file_indexer: file_indexer_config,
            file: settings_file,
            #[cfg(feature = "ai")]
            ai: ai_config,
            #[cfg(feature = "mcp")]
            mcp: mcp_config,
        });
    }

    pub fn save(&self) -> Result<(), String> {
        let mut sections: HashMap<String, HashMap<String, Option<String>>> = HashMap::new();

        sections.insert(
            String::from("file_indexer"),
            self.generate_file_indexer_section(),
        );

        sections.insert(String::from("database"), self.generate_db_section());

        #[cfg(feature = "ai")]
        sections.insert(String::from("ai"), self.generate_ai_section());

        #[cfg(feature = "ai")]
        sections.insert(String::from("rag"), self.generate_rag_section());

        #[cfg(feature = "mcp")]
        sections.insert(String::from("mcp"), self.generate_mcp_section());

        let path = Path::new(&self.file);
        iniwrite!(safe path, sections).map_err(|e| e.to_string())?;

        return Ok(());
    }

    fn process_str(section: &HashMap<String, String>, name: &str, default_value: &str) -> String {
        return section
            .get(name)
            .unwrap_or(&default_value.to_string())
            .clone();
    }

    fn process_int(section: &HashMap<String, String>, name: &str, default_value: usize) -> usize {
        return section
            .get(name)
            .unwrap_or(&default_value.to_string())
            .parse::<usize>()
            .unwrap_or(default_value);
    }

    fn process_section(
        parsed: &HashMap<String, HashMap<String, Option<String>>>,
        name: &str,
    ) -> HashMap<String, String> {
        return parsed
            .get(name)
            .unwrap_or(&HashMap::new())
            .iter()
            .map(|(k, v)| (k.clone(), v.clone().unwrap_or(String::from(""))))
            .collect();
    }

    #[cfg(feature = "mcp")]
    fn generate_mcp_section(&self) -> HashMap<String, Option<String>> {
        let mut section = HashMap::new();

        section.insert(
            String::from("listen_at"),
            Some(self.mcp.listen_addr.clone()),
        );

        return section;
    }

    #[cfg(feature = "mcp")]
    fn process_mcp_section(
        parsed: &HashMap<String, HashMap<String, Option<String>>>,
    ) -> McpServerConfig {
        let mcp_section = Settings::process_section(parsed, "mcp");

        let listen_address = Settings::process_str(&mcp_section, "listen_at", DEFAULT_LISTEN_ADDR);

        return McpServerConfig {
            listen_addr: listen_address,
        };
    }

    #[cfg(feature = "ai")]
    fn generate_ai_section(&self) -> HashMap<String, Option<String>> {
        let mut section = HashMap::new();

        section.insert(String::from("model"), Some(self.ai.model.clone()));

        section.insert(
            String::from("system_prompt"),
            Some(self.ai.preamble.clone()),
        );

        section.insert(String::from("api_key"), Some(self.ai.api_key.clone()));

        section.insert(
            String::from("ollama_think_enabled"),
            Some(String::from(if self.ai.think { "true" } else { "false" })),
        );

        section.insert(
            String::from("llm_provider"),
            Some(String::from(match self.ai.provider {
                AgentProvider::Ollama => "ollama",
                AgentProvider::OpenAI => "openai",
            })),
        );

        return section;
    }

    #[cfg(feature = "ai")]
    fn process_ai_section(
        parsed: &HashMap<String, HashMap<String, Option<String>>>,
    ) -> AgentConfig {
        let ai_section = Settings::process_section(parsed, "ai");

        let model = Settings::process_str(&ai_section, "model", DEFAULT_MODEL);

        let system_prompt = Settings::process_str(&ai_section, "system_prompt", DEFAULT_PREAMBLE);

        let api_key = Settings::process_str(&ai_section, "api_key", "");

        let think_enabled = Settings::process_str(&ai_section, "ollama_think_enabled", "false")
            .to_lowercase()
            == "true";

        let agent_provider = match Settings::process_str(&ai_section, "llm_provider", "ollama")
            .to_lowercase()
            .as_str()
        {
            "ollama" => AgentProvider::Ollama,
            "openai" => AgentProvider::OpenAI,
            _ => AgentProvider::Ollama,
        };

        let rag_config = Settings::process_rag_section(parsed);

        return AgentConfig {
            model: model,
            preamble: system_prompt,
            think: think_enabled,
            provider: agent_provider,
            rag_config: rag_config,
            api_key: api_key,
        };
    }

    #[cfg(feature = "ai")]
    fn generate_rag_section(&self) -> HashMap<String, Option<String>> {
        let mut section = HashMap::new();

        section.insert(
            String::from("number_results"),
            Some(self.ai.rag_config.top_results.to_string()),
        );

        section.insert(
            String::from("embedding_model"),
            Some(self.ai.rag_config.embedding_model.clone()),
        );

        return section;
    }

    #[cfg(feature = "ai")]
    fn process_rag_section(parsed: &HashMap<String, HashMap<String, Option<String>>>) -> RagConfig {
        let rag_section = Settings::process_section(parsed, "rag");

        let nbr_results = Settings::process_int(&rag_section, "number_results", 5);

        let embedding_model =
            Settings::process_str(&rag_section, "embedding_model", DEFAULT_EMBEDDING_MODEL);

        return RagConfig {
            top_results: nbr_results,
            embedding_model: embedding_model,
        };
    }

    fn generate_db_section(&self) -> HashMap<String, Option<String>> {
        let mut section = HashMap::new();

        section.insert(
            String::from("max_size"),
            Some(self.file_indexer.db_config.max_db_size.to_string()),
        );

        section.insert(
            String::from("db_folder_prefix"),
            Some(self.file_indexer.db_config.database_folder_prefix.clone()),
        );

        return section;
    }

    fn process_db_section(
        parsed: &HashMap<String, HashMap<String, Option<String>>>,
    ) -> DatabaseConfig {
        let database_section = Settings::process_section(parsed, "database");

        let max_db_size =
            Settings::process_int(&database_section, "max_size", DEFAULT_DATABASE_SIZE);

        let folder_prefix = Settings::process_str(
            &database_section,
            "db_folder_prefix",
            DEFAULT_SAVE_FOLDER_PREFIX,
        );

        let database_config = DatabaseConfig {
            database_folder_prefix: folder_prefix,
            max_db_size: max_db_size,
        };
        return database_config;
    }

    fn generate_file_indexer_section(&self) -> HashMap<String, Option<String>> {
        let mut section = HashMap::new();

        section.insert(
            String::from("threads"),
            Some(self.file_indexer.number_of_threads.to_string()),
        );

        section.insert(
            String::from("max_single_file_size"),
            Some(self.file_indexer.max_file_size.to_string()),
        );

        return section;
    }

    fn process_file_indexer_section(
        parsed: &HashMap<String, HashMap<String, Option<String>>>,
    ) -> FileIndexerConfig {
        let database_config = Settings::process_db_section(parsed);

        let file_indexer_section: HashMap<String, String> =
            Settings::process_section(parsed, "file_indexer");

        let threads_nbr = Settings::process_int(&file_indexer_section, "threads", 6);

        let max_single_file_size = Settings::process_int(
            &file_indexer_section,
            "max_single_file_size",
            1024 * 1024 * 20,
        );

        let file_indexer_config = FileIndexerConfig {
            max_file_size: max_single_file_size,
            number_of_threads: threads_nbr,
            db_config: database_config,
        };
        return file_indexer_config;
    }

    #[cfg(test)]
    fn set_file(&mut self, file: String) {
        self.file = file;
    }
}

impl Default for Settings {
    fn default() -> Self {
        return Self {
            file_indexer: FileIndexerConfig::new(),
            file: String::from("./ffsearch_cfg.ini"),
            #[cfg(feature = "ai")]
            ai: AgentConfig::new(
                DEFAULT_MODEL,
                RagConfig::new(DEFAULT_EMBEDDING_MODEL.to_string()),
                AgentProvider::Ollama,
            ),
            #[cfg(feature = "mcp")]
            mcp: McpServerConfig::new(),
        };
    }
}

#[cfg(test)]
pub mod tests {
    use std::{fs::File, io::Write};

    use tempdir::TempDir;

    use super::*;

    static SAMPLE_SETTINGS: &str = "
    [FILE_INDEXER]
    THREADS=12
    MAX_SINGLE_FILE_SIZE=1000000

    [DATABASE]
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
        let (dir, ini_path) = generate_ini();

        let mut settings = Settings::default();

        settings.set_file(ini_path.clone());

        settings.file_indexer.number_of_threads = 1;
        settings.file_indexer.max_file_size = 2;
        settings.file_indexer.db_config.database_folder_prefix = String::from(".data");
        settings.file_indexer.db_config.max_db_size = 3;

        settings.save().expect("Failed to save settings");

        let settings_new = Settings::load(ini_path).expect("Failed to load new settings");

        assert_eq!(settings_new.file_indexer.number_of_threads, 1);
        assert_eq!(settings_new.file_indexer.max_file_size, 2);
        assert_eq!(
            settings_new.file_indexer.db_config.database_folder_prefix,
            ".data"
        );
        assert_eq!(settings_new.file_indexer.db_config.max_db_size, 3);

        dir.close().expect("Failed to remove temp dir");
    }

    #[test]
    fn test_load_config_normal() {
        let (ini, dir) = write_sample_ini();

        let config = Settings::load(ini).expect("Failed to read config: ");

        assert_eq!(config.file_indexer.number_of_threads, 12);

        assert_eq!(config.file_indexer.max_file_size, 1000000);

        assert_eq!(config.file_indexer.db_config.max_db_size, 100000000);

        assert_eq!(
            config.file_indexer.db_config.database_folder_prefix,
            String::from(".db")
        );

        dir.close().expect("Failed to remove temp dir");
    }

    #[test]
    #[cfg(feature = "ai")]
    fn test_save_config_ai() {
        let (dir, ini_path) = generate_ini();

        let mut settings = Settings::default();

        settings.set_file(ini_path.clone());

        settings.ai.model = String::from("miku:4000b");
        settings.ai.preamble = String::from("Only answer in morse code");
        settings.ai.provider = AgentProvider::OpenAI;
        settings.ai.think = true;
        settings.ai.api_key = String::from("xyz");

        settings.save().expect("Failed to save settings");

        let settings_new = Settings::load(ini_path).expect("Failed to load new settings");

        assert_eq!(settings_new.ai.model, "miku:4000b");
        assert_eq!(settings_new.ai.preamble, "Only answer in morse code");
        assert!(match settings_new.ai.provider {
            AgentProvider::OpenAI => true,
            _ => false,
        });
        assert_eq!(settings_new.ai.think, true);
        assert_eq!(settings_new.ai.api_key, "xyz");

        dir.close().expect("Failed to remove temp dir");
    }

    #[test]
    #[cfg(feature = "ai")]
    fn test_load_config_ai() {
        let (ini, dir) = write_sample_ini();

        let config = Settings::load(ini).expect("Failed to read config: ");

        assert_eq!(config.ai.model, "minimodel:1b");

        assert_eq!(config.ai.preamble, "You are Hatsune Miku");

        assert_eq!(config.ai.think, true);

        assert!(match config.ai.provider {
            AgentProvider::OpenAI => true,
            _ => false,
        });

        assert_eq!(config.ai.api_key, "Miku-393939");

        dir.close().expect("Failed to remove temp dir");
    }

    #[test]
    #[cfg(feature = "ai")]
    fn test_save_config_rag() {
        let (dir, ini_path) = generate_ini();

        let mut settings = Settings::default();

        settings.set_file(ini_path.clone());

        settings.ai.rag_config.top_results = 39;
        settings.ai.rag_config.embedding_model = String::from("mikusuper:1b");

        settings.save().expect("Failed to save settings");

        let settings_new = Settings::load(ini_path).expect("Failed to load new settings");

        assert_eq!(settings_new.ai.rag_config.top_results, 39);
        assert_eq!(settings_new.ai.rag_config.embedding_model, "mikusuper:1b");

        dir.close().expect("Failed to remove temp dir");
    }

    #[test]
    #[cfg(feature = "ai")]
    fn test_load_config_rag() {
        let (ini, dir) = write_sample_ini();

        let config = Settings::load(ini).expect("Failed to read config: ");

        assert_eq!(config.ai.rag_config.top_results, 10);
        assert_eq!(config.ai.rag_config.embedding_model, "mikunano:latest");

        dir.close().expect("Failed to remove temp dir");
    }

    #[test]
    #[cfg(feature = "mcp")]
    fn test_save_config_mcp() {
        let (dir, ini_path) = generate_ini();

        let mut settings = Settings::default();

        settings.set_file(ini_path.clone());

        settings.mcp.listen_addr = String::from("localhost:3333");

        settings.save().expect("Failed to save settings");

        let settings_new = Settings::load(ini_path).expect("Failed to load new settings");

        assert_eq!(settings_new.mcp.listen_addr, "localhost:3333");

        dir.close().expect("Failed to remove temp dir");
    }

    #[test]
    #[cfg(feature = "mcp")]
    fn test_load_config_mcp() {
        let (ini, dir) = write_sample_ini();

        let config = Settings::load(ini).expect("Failed to read config: ");

        assert_eq!(config.mcp.listen_addr, "127.0.0.1:3939");

        dir.close().expect("Failed to remove temp dir");
    }
}
