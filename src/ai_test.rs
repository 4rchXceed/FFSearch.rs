use std::{io::Write, sync::Arc};

use application::{
    ai::actions::ai_chat::AiChat,
    indexer::{actions::sync::FileIndexerSync, traits::registry::FileIndexerRegistry},
};
use domain::{
    ai::{config::AgentConfig, rag_config::RagConfig},
    indexer::config::FileIndexerConfig,
    searcher::config::DatabaseConfig,
};
use infrastructure::{
    ai::rig::agent::RigAgent,
    indexer::{
        stdfile::{indexer::StdFileIndexerProvider, registry::InMemoryRegistry},
        xberg::extractor::XbergFileExtractor,
    },
    searcher::tantivy::database::TantivyDatabase,
};
use tokio::sync::mpsc::channel;

const FOLDER: &str = "E:\\IN_3\\1_4\\M107\\S_Scripts";

fn print(cmd: &str) {
    print!("{cmd}");
    std::io::stdout().flush().expect("Failed to flush");
}

pub async fn test_ai() {
    let indexer = StdFileIndexerProvider::new(FileIndexerConfig::new());
    let indexer = Arc::new(indexer);

    let db = TantivyDatabase::new(String::from(FOLDER), DatabaseConfig::new())
        .expect("Failed to create db");
    let db = Arc::new(db);

    let text_extractor = XbergFileExtractor::new(&FileIndexerConfig::new());
    let text_extractor = Arc::new(text_extractor);

    let registry = InMemoryRegistry::new();
    let registry = Arc::new(registry);

    let id = registry.add(indexer, db.clone(), FOLDER.to_string()).await;

    println!("Syncing...");

    let indexer_sync = FileIndexerSync::new(registry.clone(), text_extractor);
    indexer_sync.sync(id).await.expect("Failed to sync");

    println!("Sync OK!");
    println!("Building rag...");

    let ai = RigAgent::new_auto_provider(
        AgentConfig::new(
            "llama3.1:8b",
            RagConfig::new(String::from("embeddinggemma:latest")),
            String::from("ollama"),
        ),
        db,
    )
    .await
    .expect("Failed to create ai");
    let ai = Arc::new(ai);
    registry
        .assign_agent(id, ai)
        .await
        .expect("Failed to add AI");

    println!("Finished!");

    let chat = AiChat::new(registry);

    let (send, mut recv) = channel::<String>(100);

    tokio::spawn(async move {
        loop {
            let message = recv.recv().await.unwrap_or_default();
            print(message.as_str());
        }
    });

    loop {
        print("> ");
        let mut buffer = String::new();
        std::io::stdin()
            .read_line(&mut buffer)
            .expect("Failed to read stdin");

        if buffer == "exit" {
            break;
        }

        match chat.ask(id, buffer, send.clone()).await {
            Ok(_) => {}
            Err(e) => eprintln!("{e}"),
        };
        println!("");
    }
}
