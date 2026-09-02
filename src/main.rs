use std::{io::Write, process::ExitCode};

use rig::client::{EmbeddingsClient, Nothing};

use crate::{
    ai::{
        agent::Agent,
        config::{AgentConfig, AgentProvider},
        rag::config::RagConfig,
    },
    file_indexer::{config::FileIndexerConfig, indexer::FileIndexer},
};

mod ai;
mod file_indexer;
mod searcher;
mod third_party;
mod utils;

async fn paf() -> Option<ExitCode> {
    let mut indexer = FileIndexer::new(
        String::from("E:\\Tronc_commun\\TC_24\\M117\\S_Scripts"),
        FileIndexerConfig::new(),
    )
    .expect("Failed to create file indexer");
    indexer.sync();
    println!("Indexer synced");
    let client = rig::providers::ollama::Client::new(Nothing).expect("Client failed to connect");
    let model = client.embedding_model("embeddinggemma:latest");
    println!("Building agent...");
    let mut ai =
        Agent::new::<rig::providers::ollama::Client, rig::providers::ollama::EmbeddingModel>(
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

#[tokio::main]
async fn main() {
    std::thread::Builder::new()
        .stack_size(33554432000)
        .spawn(async move || {
            return paf().await.unwrap_or(0.into());
        })
        .unwrap()
        .join()
        .unwrap()
        .await;
}
