use std::collections::HashMap;
use std::path::Path;

#[cfg(feature = "ai")]
use domain::ai::prompts::DEFAULT_PREAMBLE;
#[cfg(feature = "ai")]
use domain::ai::{config::AgentConfig, rag_config::RagConfig};
use domain::{
    indexer::config::FileIndexerConfig,
    searcher::config::{DEFAULT_DATABASE_SIZE, DEFAULT_SAVE_FOLDER_PREFIX, DatabaseConfig},
};

#[cfg(feature = "mcp_server")]
use mcp::config::DEFAULT_LISTEN_ADDR;

// #[cfg(feature = "ai")]
// #[cfg(feature = "mcp_server")]
use ini::{ini, iniwrite};
#[cfg(feature = "mcp_server")]
use mcp::config::McpServerConfig;

#[cfg(feature = "ai")]
static DEFAULT_MODEL: &str = "llama3.1:8b";

#[cfg(feature = "ai")]
static DEFAULT_EMBEDDING_MODEL: &str = "embeddinggemma:latest";

pub struct Settings {
    pub file_indexers: HashMap<String, FileIndexerConfig>,
    file: String,
    #[cfg(feature = "ai")]
    pub ai: AgentConfig,
    #[cfg(feature = "mcp_server")]
    pub mcp: McpServerConfig,
}

impl Settings {
    pub fn load(settings_file: String) -> Result<Self, String> {
        let parsed = ini!(safe settings_file.clone())?;

        let file_indexer_config = Settings::process_file_indexers_sections(&parsed);

        #[cfg(feature = "ai")]
        let ai_config = Settings::process_ai_section(&parsed);

        #[cfg(feature = "mcp_server")]
        let mcp_config = Settings::process_mcp_section(&parsed);

        return Ok(Self {
            file_indexers: file_indexer_config,
            file: settings_file,
            #[cfg(feature = "ai")]
            ai: ai_config,
            #[cfg(feature = "mcp_server")]
            mcp: mcp_config,
        });
    }

    pub fn save(&self) -> Result<(), String> {
        let mut sections: HashMap<String, HashMap<String, Option<String>>> = HashMap::new();

        sections.extend(self.generate_file_indexers_sections());

        #[cfg(feature = "ai")]
        sections.insert(String::from("ai"), self.generate_ai_section());

        #[cfg(feature = "ai")]
        sections.insert(String::from("rag"), self.generate_rag_section());

        #[cfg(feature = "mcp_server")]
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

    #[cfg(feature = "mcp_server")]
    fn generate_mcp_section(&self) -> HashMap<String, Option<String>> {
        let mut section = HashMap::new();

        section.insert(
            String::from("listen_at"),
            Some(self.mcp.listen_addr.clone()),
        );

        return section;
    }

    #[cfg(feature = "mcp_server")]
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

        section.insert(String::from("llm_provider"), Some(self.ai.provider.clone()));

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

        let agent_provider =
            Settings::process_str(&ai_section, "llm_provider", "ollama").to_lowercase();

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

        let max_batch_size = Settings::process_int(&rag_section, "file_batch_size", 2000);

        return RagConfig {
            top_results: nbr_results,
            embedding_model: embedding_model,
            max_batch_size: max_batch_size,
        };
    }

    fn generate_db_section(&self, config: &DatabaseConfig) -> HashMap<String, Option<String>> {
        let mut section = HashMap::new();

        section.insert(
            String::from("max_size"),
            Some(config.max_db_size.to_string()),
        );

        section.insert(
            String::from("db_folder_prefix"),
            Some(config.database_folder_prefix.clone()),
        );

        return section;
    }

    fn process_db_section(database_section: &HashMap<String, String>) -> DatabaseConfig {
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

    fn generate_file_indexer_section(
        &self,
        config: &FileIndexerConfig,
        folder: String,
    ) -> HashMap<String, Option<String>> {
        let mut section = HashMap::new();

        section.insert(
            String::from("threads"),
            Some(config.number_of_threads.to_string()),
        );

        section.insert(
            String::from("max_single_file_size"),
            Some(config.max_file_size.to_string()),
        );

        section.insert(String::from("folder"), Some(folder));

        return section;
    }

    fn generate_file_indexers_sections(&self) -> HashMap<String, HashMap<String, Option<String>>> {
        let mut sections = HashMap::new();

        let mut i = 0;

        for (folder, config) in &self.file_indexers {
            sections.insert(
                format!("file_indexer.{}", i),
                self.generate_file_indexer_section(config, folder.clone()),
            );
            sections.insert(
                format!("database.{}", i),
                self.generate_db_section(&config.db_config),
            );

            i += 1;
        }

        return sections;
    }

    fn process_file_indexers_sections(
        parsed: &HashMap<String, HashMap<String, Option<String>>>,
    ) -> HashMap<String, FileIndexerConfig> {
        let mut settings_sections = HashMap::new();

        for (key, _) in parsed {
            if let Some(id) = key.strip_prefix("file_indexer.") {
                let db_key = &format!("database.{id}");
                let indexer_section = &Self::process_section(parsed, key);

                if let Some(_) = parsed.get(db_key)
                    && let Some(folder) = indexer_section.get("folder")
                {
                    let settings = Self::process_file_indexer_section(
                        indexer_section,
                        &Self::process_section(parsed, db_key),
                    );

                    settings_sections.insert(folder.clone(), settings);
                }
            }
        }

        return settings_sections;
    }

    fn process_file_indexer_section(
        file_indexer_section: &HashMap<String, String>,
        database_section: &HashMap<String, String>,
    ) -> FileIndexerConfig {
        let database_config = Settings::process_db_section(database_section);

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

    pub fn set_file(&mut self, file: String) {
        self.file = file;
    }
}

impl Default for Settings {
    fn default() -> Self {
        return Self {
            file_indexers: HashMap::new(),
            file: String::from("./ffsearch_cfg.ini"),
            #[cfg(feature = "ai")]
            ai: AgentConfig::new(
                DEFAULT_MODEL,
                RagConfig::new(DEFAULT_EMBEDDING_MODEL.to_string()),
                String::from("ollama"),
            ),
            #[cfg(feature = "mcp_server")]
            mcp: McpServerConfig::new(),
        };
    }
}
