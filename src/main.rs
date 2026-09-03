use std::{collections::HashMap, io::Write, process::ExitCode, sync::Arc};

use rig::client::{EmbeddingsClient, Nothing};
use tokio::sync::RwLock;

use crate::{
    ai::{
        agent::Agent,
        config::{AgentConfig, AgentProvider},
        rag::config::RagConfig,
    },
    file_indexer::{config::FileIndexerConfig, indexer::FileIndexer},
    mcp::{
        config::McpServerConfig,
        server::{McpInstancesContext, McpServer},
    },
};

mod ai;
mod file_indexer;
mod mcp;
mod searcher;
mod third_party;
mod utils;

async fn test_ai() -> Option<ExitCode> {
    let mut indexer = FileIndexer::new(
        String::from("E:\\Tronc_commun\\TC_24\\M117\\S_Scripts"),
        FileIndexerConfig::new(),
    )
    .expect("Failed to create file indexer");
    indexer.sync().ok()?;
    println!("Indexer synced");
    let client = rig::providers::ollama::Client::new(Nothing).expect("Client failed to connect");
    let model = client.embedding_model("embeddinggemma:latest");
    println!("Building agent...");
    let mut ai =
        Agent::new_auto::<rig::providers::ollama::Client, rig::providers::ollama::EmbeddingModel>(
            client,
            AgentConfig::new("llama3.1:8b", RagConfig::new(), AgentProvider::Ollama),
            indexer.get_db(),
            model,
        )
        .await
        .expect("Failed to create agent");
    let mut stop = false;
    while !stop {
        let mut buf = String::new();
        print!("Enter a command: ");
        std::io::stdout().flush().ok();
        std::io::stdin()
            .read_line(&mut buf)
            .expect("Failed to read line");
        let buf = buf.trim();
        if buf == "exit" {
            stop = true;
        } else {
            let r = ai
                .send(
                    buf,
                    |_, c| {
                        print!("{}", c);
                        std::io::stdout().flush().ok();
                    },
                    &mut (),
                )
                .await;
            if r.is_err() {
                println!("{:?}", r.unwrap_err());
            }
            println!("");
        }
    }
    return None;
}

async fn test_mcpserver() -> Result<(), String> {
    let mut folders = HashMap::new();
    let mut fserver1 = FileIndexer::new(String::from("src"), FileIndexerConfig::new())
        .map_err(|e| e.to_string())?;
    fserver1.sync().expect("Failed to sync");
    let mut fserver2 = FileIndexer::new(
        String::from("/run/media/archxceed/MikuMikuLove_LyamZambaz/Tronc_commun/"),
        FileIndexerConfig::new(),
    )
    .map_err(|e| e.to_string())?;
    fserver2.sync().expect("Failed to sync");
    folders.insert(1, fserver1);
    folders.insert(2, fserver2);
    let folders_threaded = Arc::new(RwLock::new(folders));

    McpServer::run_server(
        McpInstancesContext {
            folders: folders_threaded,
        },
        McpServerConfig::new(),
    )
    .await
    .expect("Failed to run the server");
    return Ok(());
}

async fn cli() {
    let mut path = String::new();
    println!("Enter the path to index:");
    std::io::stdin()
        .read_line(&mut path)
        .expect("Failed to read line");
    let path = path.trim().to_string();
    let mut indexer =
        FileIndexer::new(path, FileIndexerConfig::new()).expect("Failed to create file indexer");
    let mut index = String::new();
    println!("Index now (Y/n):");
    std::io::stdin()
        .read_line(&mut index)
        .expect("Failed to read line");
    let index = index.trim().to_string();
    if index == "Y" || index == "y" || index == "" {
        indexer.sync().expect("Failed to sync indexer");
        println!("Indexer synced");
    } else {
        println!("Indexer not synced");
    }
    loop {
        let mut query = String::new();
        println!("Enter a query (or 'exit' to quit):");
        std::io::stdin()
            .read_line(&mut query)
            .expect("Failed to read line");
        let query = query.trim().to_string();
        if query == "exit" {
            break;
        }
        let start = std::time::Instant::now();
        let results = indexer
            .get_db()
            .search(query.clone(), 5)
            .expect("Failed to search");
        let duration = start.elapsed();
        println!("Search took: {:?}", duration);
        println!("Results for query '{}':", query);
        for (i, result) in results.iter().enumerate() {
            println!("Result {}: {}", i + 1, result.path);
        }
    }
}

#[tokio::main]
async fn main() {
    let action = std::env::args().nth(1);
    if action.is_none() {
        println!("Please provide an action: ai, mcp or cli");
        return;
    }
    let action = action.unwrap();
    if action == "ai" {
        test_ai().await;
    } else if action == "mcp" {
        test_mcpserver().await.expect("Error while running MCP");
    } else if action == "cli" {
        cli().await;
    } else {
        println!("Unknown action: {}", action);
    }
}
