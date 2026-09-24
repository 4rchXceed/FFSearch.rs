use std::sync::Arc;

use hyper_util::{
    rt::{TokioExecutor, TokioIo},
    service::TowerToHyperService,
};
use rmcp::{
    ErrorData, ServerHandler,
    handler::server::wrapper::Parameters,
    model::{CallToolResult, ContentBlock, ServerCapabilities, ServerInfo},
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

use crate::{controller::controller::Controller, mcp::config::McpServerConfig};

// Re-used text
static INDEXER_DELETED_TEXT: &'static str =
    "The searcher has been deleted by the user while the action was running";

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
    pub controller: Arc<RwLock<Controller>>,
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

        let listen_addr = config.listen_addr;

        log::info!("Server listening to {listen_addr}");

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
                    log::info!("Ctrl+C received, shutting down...");
                    break;
                }
                accept = listener.accept() => {
                    match accept {
                        Ok((stream, _)) => {
                            Self::accept(stream, context.clone(), session_manager.clone());
                        }
                        Err(e) => {
                            let error = e.to_string();
                            log::error!("Accept error: {error}");
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

            if let Err(error) = conn {
                let error = error.to_string();
                log::error!("Connection error: {error}");
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

        for (id, folder_path) in self
            .context
            .controller
            .read()
            .await
            .get_file_indexers()
            .await
        {
            paths.push(format!("{}: {}", id, folder_path));
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
        let folder_controller = self
            .context
            .controller
            .read()
            .await
            .get_controller_for(folder_id as u32, self.context.controller.clone());

        match folder_controller {
            Ok(controller) => {
                let result = controller.get_content(path).await;

                if let Some(result) = result {
                    return match result {
                        Ok(result) => {
                            let content = ContentBlock::text(
                                result.content.unwrap_or(String::from("No content")),
                            );
                            Ok(CallToolResult::success(vec![content]))
                        }
                        Err(error) => {
                            let content =
                                ContentBlock::text(format!("Error: {}", error.to_string()));

                            Ok(CallToolResult::error(vec![content]))
                        }
                    };
                } else {
                    return Ok(CallToolResult::error(vec![ContentBlock::text(
                        INDEXER_DELETED_TEXT,
                    )]));
                }
            }
            Err(e) => {
                let error_text = format!("Error: {}", e);

                return Ok(CallToolResult::error(vec![ContentBlock::text(error_text)]));
            }
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
        let folder_controller = self
            .context
            .controller
            .read()
            .await
            .get_controller_for(folder_id as u32, self.context.controller.clone());

        match folder_controller {
            Ok(controller) => {
                let search_results = controller.search(query, limit).await;

                if let Some(search_results) = search_results {
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
                } else {
                    return Ok(CallToolResult::error(vec![ContentBlock::text(
                        INDEXER_DELETED_TEXT,
                    )]));
                }
            }
            Err(e) => {
                let error_text = ContentBlock::text(format!("Error: {}", e));

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
