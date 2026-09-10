use std::{collections::HashMap, sync::Arc};

use hyper_util::{
    rt::{TokioExecutor, TokioIo},
    service::TowerToHyperService,
};
use rmcp::{
    ErrorData, ServerHandler,
    handler::server::wrapper::Parameters,
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

/// Search request tool
/// There's 3 parameters:
/// - The ID of the folder
/// - The Query
/// - The max number of documents
#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct SearchRequest {
    #[schemars(description = "The folder ID you got from list folders")]
    pub folder_id: i32,
    #[schemars(description = "The search query (tantivy)")]
    pub query: String,
    #[schemars(description = "The max number of documents to return")]
    pub limit: usize,
}

/// Gets the content of a file
/// There's 2 parameters
/// - The ID of the folder
/// - The path of the file (relative)
#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct GetContentRequest {
    #[schemars(description = "The folder ID you got from list folders")]
    pub folder_id: i32,
    #[schemars(description = "The file path")]
    pub path: String,
}

/// This is the shared context between all MCP threeads
#[derive(Clone)]
pub struct McpInstancesContext {
    /// Thread-Safe reference to the sync folders
    pub folders: Arc<RwLock<HashMap<i32, FileIndexer>>>,
}

/// The McpServer the the main MCP class
/// It manages the TCP connection, Tool callbacks, etc.
pub struct McpServer {
    context: McpInstancesContext,
    // tool_router: ToolRouter<McpServer>,
}

#[tool_router]
impl McpServer {
    /// Creates and runs an MCP server.
    /// Uses TcpListener.
    /// Parameters:
    /// - context: the context containing the folders
    /// - config: the MCP server config
    pub async fn run_server(
        context: McpInstancesContext,
        config: McpServerConfig,
    ) -> Result<(), std::io::Error> {
        let listener = tokio::net::TcpListener::bind(config.listen_addr.clone()).await?;

        // TODO: Proper logging
        println!("Server listening to {}", config.listen_addr);
        Self::listen(listener, context).await;

        return Ok(());
    }

    /// Accepts client connections, with a specified context
    /// Then runs Self::accept();
    /// Parameters:
    /// - listener: the socket listener
    /// - context: the MCP server thread-safe context
    async fn listen(listener: TcpListener, context: McpInstancesContext) {
        let session_manager = Arc::new(LocalSessionManager::default());

        loop {
            tokio::select! {
                _ = tokio::signal::ctrl_c() => {
                    println!("Ctrl+C received, shutting down...");
                    break;
                }
                accept = listener.accept() => {
                    match accept {
                        Ok((stream, _)) => {
                            Self::accept(stream, context.clone(), session_manager.clone());
                        }
                        Err(e) => {
                            eprintln!("Accept error: {:?}", e);
                        }
                    }
                }
            }
        }
    }

    /// Handles a client by passing the client to the Hyper / RMCP
    /// Parameters:
    /// - stream: the "client" (as socket)
    /// - context: the MCP context (thread-safe)
    fn accept(
        stream: TcpStream,
        context: McpInstancesContext,
        session_manager: Arc<LocalSessionManager>,
    ) {
        let io = TokioIo::new(stream);

        tokio::spawn(async move {
            let service = TowerToHyperService::new(StreamableHttpService::new(
                move || Ok(McpServer::new(context.clone())),
                session_manager.clone(),
                Default::default(),
            ));

            let conn = hyper_util::server::conn::auto::Builder::new(TokioExecutor::default())
                .serve_connection(io, service)
                .await;

            if conn.is_err() {
                eprintln!("Connection error: {:?}", conn.unwrap_err());
            }
        });
    }

    /// Creates a single McpServer
    /// Parameters:
    /// - context: the thread safe context
    fn new(context: McpInstancesContext) -> Self {
        return Self {
            context: context,
            // tool_router: Self::tool_router(),
        };
    }

    /// Tool: "Lists all folders that are searchable"
    #[tool(description = "Lists all folders that are searchable")]
    async fn list_folders(&self) -> Result<CallToolResult, ErrorData> {
        let mut paths = Vec::new();

        for (id, folder) in self.context.folders.read().await.iter() {
            paths.push(format!("{}: {}", id, folder.get_db_ro().get_index_path()));
        }

        let content = ContentBlock::text(paths.join(", "));
        return Ok(CallToolResult::success(vec![content]));
    }

    /// Tool: "Get the content of a file"
    #[tool(description = "Get the content of a file")]
    async fn get_content(
        &self,
        Parameters(GetContentRequest { folder_id, path }): Parameters<GetContentRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let folders = self.context.folders.read().await;
        let indexer = folders.get(&folder_id);

        if let Some(indexer) = indexer {
            let result = indexer.get_db_ro().get(path);

            if let Ok(result) = result {
                let content =
                    ContentBlock::text(result.content.unwrap_or(String::from("No content")));

                return Ok(CallToolResult::success(vec![content]));
            } else {
                let content =
                    ContentBlock::text(format!("Error: {}", result.unwrap_err().to_string()));

                return Ok(CallToolResult::error(vec![content]));
            }
        } else {
            let error_text = format!("Folder with id {} not found", folder_id);

            return Ok(CallToolResult::error(vec![ContentBlock::text(error_text)]));
        }
    }

    /// Tool: "Search in a folder (returns files without content)"
    #[tool(description = "Search in a folder (returns files without content)")]
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

        match indexer {
            Some(indexer) => {
                let search_results = indexer.get_db_ro().search(query, limit);

                match search_results {
                    Err(error) => {
                        let error_text =
                            ContentBlock::text(format!("Error while searching: {}", error));
                        return Ok(CallToolResult::error(vec![error_text]));
                    }

                    Ok(search_results) => {
                        let results: Vec<String> = search_results
                            .iter()
                            .map(|r| {
                                format!(
                                    "Path: {}, Last modification: {}",
                                    r.path,
                                    r.modification_date.into_primitive().to_string()
                                )
                            })
                            .collect();

                        let content = ContentBlock::text(results.join("\n"));

                        return Ok(CallToolResult::success(vec![content]));
                    }
                }
            }
            None => {
                let error_text =
                    ContentBlock::text(format!("Folder with id {} not found", folder_id));

                return Ok(CallToolResult::error(vec![error_text]));
            }
        }
    }
}

#[tool_handler]
impl ServerHandler for McpServer {
    /// Gets the server informations
    /// Returns:
    /// - the server infos
    fn get_info(&self) -> ServerInfo {
        return ServerInfo::new(
            ServerCapabilities::builder()
                .enable_tools()
                .enable_tool_list_changed()
                .build(),
        );
    }
}
