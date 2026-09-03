use std::{collections::HashMap, io::Error, net::SocketAddr, sync::Arc};

use chrono::Local;
use hyper_util::{
    rt::{TokioExecutor, TokioIo},
    service::TowerToHyperService,
};
use rig::schemars;
use rmcp::{
    ErrorData, ServerHandler,
    handler::server::{tool::ToolRouter, wrapper::Parameters},
    model::{CallToolResult, ContentBlock, ServerCapabilities, ServerInfo, TextContent},
    tool, tool_handler, tool_router,
    transport::{
        StreamableHttpService, streamable_http_server::session::local::LocalSessionManager,
    },
};
use serde::Deserialize;
use tokio::{
    net::{TcpListener, TcpStream},
    sync::RwLock,
};

use crate::{file_indexer::indexer::FileIndexer, mcp::config::McpServerConfig};

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct SearchRequest {
    #[schemars(description = "The folder ID you got from list folders")]
    pub folder_id: i32,
    #[schemars(description = "The search query (tantivy)")]
    pub query: String,
    #[schemars(description = "The max number of documents to return")]
    pub limit: usize,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct GetContentRequest {
    #[schemars(description = "The folder ID you got from list folders")]
    pub folder_id: i32,
    #[schemars(description = "The file path")]
    pub path: String,
}

#[derive(Clone)]
pub struct McpInstancesContext {
    pub folders: Arc<RwLock<HashMap<i32, FileIndexer>>>,
}

pub struct McpServer {
    context: McpInstancesContext,
    tool_router: ToolRouter<McpServer>,
}

#[tool_router]
impl McpServer {
    pub async fn run_server(
        context: McpInstancesContext,
        config: McpServerConfig,
    ) -> Result<(), std::io::Error> {
        let listener = tokio::net::TcpListener::bind(config.listen_addr.clone()).await?;
        println!("Server listening to {}", config.listen_addr);
        Self::single_run_server(listener, context).await;
        return Ok(());
    }

    pub async fn single_run_server(listener: TcpListener, context: McpInstancesContext) {
        let session_manager = Arc::new(LocalSessionManager::default());
        loop {
            tokio::select! {
                _ = tokio::signal::ctrl_c() => {
                    println!("Ctrl+C received, shutting down...");
                    break;
                }
                accept = listener.accept() => {
                    Self::accept(accept, context.clone(), session_manager.clone());
                }
            }
        }
    }

    fn accept(
        accept: Result<(TcpStream, SocketAddr), Error>,
        context: McpInstancesContext,
        session_manager: Arc<LocalSessionManager>,
    ) {
        match accept {
            Ok((stream, _addr)) => {
                let io = TokioIo::new(stream);
                tokio::spawn(async move {
                    let service = TowerToHyperService::new(StreamableHttpService::new(
                        move || Ok(McpServer::new(context.clone())),
                        session_manager.clone(),
                        Default::default(),
                    ));

                    let conn =
                        hyper_util::server::conn::auto::Builder::new(TokioExecutor::default())
                            .serve_connection(io, service)
                            .await;
                    if conn.is_err() {
                        eprintln!("Connection error: {:?}", conn.unwrap_err());
                    }
                });
            }
            Err(e) => {
                eprintln!("Accept error: {:?}", e);
            }
        }
    }

    pub fn new(context: McpInstancesContext) -> Self {
        return Self {
            context: context.clone(),
            tool_router: Self::tool_router(),
        };
    }

    #[tool(description = "Lists all folders that are searchable")]
    async fn list_folders(&self) -> Result<CallToolResult, ErrorData> {
        let mut paths = Vec::new();
        for (id, folder) in self.context.folders.read().await.iter() {
            paths.push(format!("{}: {}", id, folder.get_db_ro().get_index_path()));
        }
        let text = TextContent::new(paths.join(", "));
        return Ok(CallToolResult::success(vec![ContentBlock::Text(text)]));
    }

    #[tool(description = "Get the content of a folder")]
    async fn get_content(
        &self,
        Parameters(GetContentRequest { folder_id, path }): Parameters<GetContentRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let folders = self.context.folders.read().await;
        let indexer = folders.get(&folder_id);
        if indexer.is_some() {
            let indexer = indexer.unwrap();
            let result = indexer.get_db_ro().get(path);
            if result.is_ok() {
                return Ok(CallToolResult::success(vec![ContentBlock::text(
                    result
                        .unwrap()
                        .content
                        .unwrap_or(String::from("No content")),
                )]));
            } else {
                return Ok(CallToolResult::error(vec![ContentBlock::text(format!(
                    "Error: {}",
                    result.unwrap_err().to_string()
                ))]));
            }
        } else {
            let error_text = format!("Folder with id {} not found", folder_id);
            return Ok(CallToolResult::error(vec![ContentBlock::text(error_text)]));
        }
    }

    #[tool(description = "Search in a folder")]
    async fn search(
        &self,
        Parameters(SearchRequest {
            folder_id,
            query,
            limit,
        }): Parameters<SearchRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let folders = self.context.folders.read().await;
        let indexer = folders.get(&folder_id);
        if indexer.is_some() {
            let indexer = indexer.unwrap();
            let search_results = indexer.get_db_ro().search(query, limit);
            if search_results.is_err() {
                let error_text = format!("Error while searching: {}", search_results.unwrap_err());
                return Ok(CallToolResult::error(vec![ContentBlock::text(error_text)]));
            } else {
                let mut results = Vec::new();
                for search_result in search_results.unwrap() {
                    println!(
                        "{}",
                        search_result.modification_date.into_primitive().to_string()
                    );
                    results.push(format!(
                        "Path: {}, Last modification: {}",
                        search_result.path,
                        search_result.modification_date.into_primitive().to_string()
                    ));
                }
                return Ok(CallToolResult::success(vec![ContentBlock::text(
                    results.join("\n"),
                )]));
            }
        } else {
            let error_text = format!("Folder with id {} not found", folder_id);
            return Ok(CallToolResult::error(vec![ContentBlock::text(error_text)]));
        }
    }
}

#[tool_handler]
impl ServerHandler for McpServer {
    fn get_info(&self) -> ServerInfo {
        return ServerInfo::new(
            ServerCapabilities::builder()
                .enable_tools()
                .enable_tool_list_changed()
                .build(),
        );
    }
}
