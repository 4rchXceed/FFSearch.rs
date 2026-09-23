#[cfg(feature = "ai")]
use rig::client::{EmbeddingsClient, Nothing};
use std::io::Write;
#[cfg(feature = "ai")]
use std::process::ExitCode;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;

#[cfg(feature = "ai")]
use crate::ai::{
    agent::Agent,
    config::{AgentConfig, AgentProvider},
    rag::config::RagConfig,
};

#[cfg(feature = "mcp")]
use crate::mcp::{
    config::McpServerConfig,
    server::{McpInstancesContext, McpServer},
};

#[cfg(feature = "daemon")]
use crate::daemon::manager::DaemonManager;

use crate::file_indexer::{config::FileIndexerConfig, indexer::FileIndexer};

#[cfg(feature = "ai")]
mod ai;
mod controller;
#[cfg(feature = "daemon")]
mod daemon;
mod file_indexer;
#[cfg(feature = "mcp")]
mod mcp;
mod searcher;
mod third_party;
mod utils;

#[cfg(feature = "ai")]
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
            AgentConfig::new(
                "llama3.1:8b",
                RagConfig::new(String::from("embeddinggemma:latest")),
                AgentProvider::Ollama,
            ),
            Arc::new(RwLock::new(indexer)),
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
            let (tx, mut rx) = tokio::sync::mpsc::channel(100);

            tokio::spawn(async move {
                loop {
                    match rx.recv().await {
                        Some(m) => print!("{}", m),
                        None => break,
                    }
                }
            });

            let r = ai.send(String::from(buf), tx).await;

            if r.is_err() {
                println!("{:?}", r.unwrap_err());
            }
            println!("");
        }
    }
    return None;
}

#[cfg(feature = "mcp")]
async fn test_mcpserver() -> Result<(), String> {
    // let mut folders = HashMap::new();
    // let mut fserver1 = FileIndexer::new(String::from("src"), FileIndexerConfig::new())
    //     .map_err(|e| e.to_string())?;
    // fserver1.sync().expect("Failed to sync");
    // let mut fserver2 = FileIndexer::new(
    //     String::from(
    //         std::env::args()
    //             .nth(2)
    //             .expect("Please provide a path for the MCP"),
    //     ),
    //     FileIndexerConfig::new(),
    // )
    // .map_err(|e| e.to_string())?;
    // // fserver2.sync().expect("Failed to sync");
    // folders.insert(1, fserver1);
    // folders.insert(2, fserver2);
    // let folders_threaded = Arc::new(RwLock::new(folders));

    // McpServer::run_server(
    //     McpInstancesContext {
    //         folders: folders_threaded,
    //     },
    //     McpServerConfig::new(),
    // )
    // .await
    // .expect("Failed to run the server");
    return Ok(());
}

#[cfg(feature = "daemon")]
async fn daemon() {
    // let mut folders = HashMap::new();
    // let mut config = FileIndexerConfig::new();
    // config.number_of_threads = 16;
    // let mut daemon_manager = DaemonManager::new().expect("Failed to create daemon manager");
    // daemon_manager.start();
    // loop {
    //     let mut command = String::new();
    //     print!(">");
    //     std::io::stdout().flush().expect("Failed to flush");
    //     std::io::stdin()
    //         .read_line(&mut command)
    //         .expect("Failed to read line");
    //     let command = command.trim();
    //     let parts = command.split(" ").collect::<Vec<&str>>();
    //     if let Some(action) = parts.first() {
    //         if *action == "/add" {
    //             if let Some(path) = parts.get(1) {
    //                 let file_indexer = FileIndexer::new(path.to_string(), config.clone())
    //                     .expect("Failed to create file indexer");
    //                 let arc = Arc::new(RwLock::new(file_indexer));
    //                 let id = folders.len().to_string();
    //                 println!("ID: {}", id);
    //                 folders.insert(id, arc.clone());
    //                 daemon_manager.add_folder(path.to_string(), arc).await;
    //             }
    //         } else if *action == "/rm" {
    //             if let Some(path) = parts.get(1) {
    //                 daemon_manager.remove_folder(path.to_string()).await;
    //             }
    //         } else if *action == "/search" {
    //             if let Some(id) = parts.get(1) {
    //                 if let Some(query) = parts.get(2) {
    //                     let limit = parts.get(3).unwrap_or(&"5").parse::<usize>().unwrap_or(5);

    //                     if let Some(folder) = folders.get(&id.to_string()) {
    //                         let res = folder
    //                             .read()
    //                             .await
    //                             .get_db_ro()
    //                             .search(String::from(*query), limit);
    //                         if let Ok(results) = res {
    //                             for (i, result) in results.iter().enumerate() {
    //                                 println!("{}: {}", i + 1, result.path);
    //                             }
    //                         } else if let Err(error) = res {
    //                             println!("Error while searching: {}", error.to_string());
    //                         }
    //                         println!("Search complete");
    //                     } else {
    //                         println!("Cannot find folder");
    //                     }
    //                 } else {
    //                     println!("Query invalid");
    //                 }
    //             } else {
    //                 println!("ID invalid");
    //             }
    //         } else if *action == "exit" {
    //             daemon_manager.stop_daemon().await;
    //             break;
    //         } else {
    //             println!("{}: command not found", *action);
    //         }
    //     }
    // }
}

async fn cli() {
    let mut path = String::new();
    println!("Enter the path to index:");
    std::io::stdin()
        .read_line(&mut path)
        .expect("Failed to read line");
    let path = path.trim().to_string();
    let mut config = FileIndexerConfig::new();
    config.number_of_threads = 16;
    let mut indexer = FileIndexer::new(path, config).expect("Failed to create file indexer");
    let mut index = String::new();
    println!("Index now (Y/n):");
    std::io::stdin()
        .read_line(&mut index)
        .expect("Failed to read line");
    let index = index.trim().to_string();
    if index == "Y" || index == "y" || index == "" {
        let start = std::time::Instant::now();
        indexer.sync().expect("Failed to sync indexer");
        let duration = start.elapsed();
        println!("Search took: {:?}", duration);
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
    // Small test
    let action = std::env::args().nth(1);
    if let Some(action) = action {
        if action == "ai" {
            #[cfg(feature = "ai")]
            test_ai().await;
        } else if action == "mcp" {
            #[cfg(feature = "mcp")]
            test_mcpserver().await.expect("Error while running MCP");
        } else if action == "cli" {
            cli().await;
        } else if action == "daemon" {
            #[cfg(feature = "daemon")]
            daemon().await;
        } else {
            println!("Unknown action: {}", action);
        }
    } else {
        println!("Please provide an action: ai, mcp, cli or daemon");
        return;
    }
}
