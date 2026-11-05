use std::sync::Arc;

use application::searcher::actions::{get::DatabaseGet, search::SearchDocument};
use domain::searcher::search_query::SearchContent;
use hyper_util::{
    rt::{TokioExecutor, TokioIo},
    service::TowerToHyperService,
};
use rmcp::{
    ErrorData, ServerHandler,
    handler::server::wrapper::Parameters,
    model::{CallToolResult, ContentBlock, ServerCapabilities, ServerConfig},
    tool, tool_handler, tool_router,
    transport::{
        StreamableHttpService, streamable_http_server::session::local::LocalSessionManager,
    },
};
use rust_string_utils::timestamp_to_string;
use serde::Deserialize;
use tokio::net::{TcpListener, TcpStream};

use crate::config::McpServerConfig;

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
    pub db_get: Arc<DatabaseGet>,
    pub db_search: Arc<SearchDocument>,
}

/// The McpServer the the main MCP class
/// It manages the TCP connection, Tool callbacks, etc.
pub struct McpServer {
    context: McpInstancesContext,
}

#[tool_router]
impl McpServer {
    /// Creates and runs an MCP server.
    /// Uses TcpListener.
    /// Parameters:
    /// - context: the context containing the folders
    /// - config: the MCP server config
    pub async fn run_server(
        get: Arc<DatabaseGet>,
        search: Arc<SearchDocument>,
        config: McpServerConfig,
    ) -> Result<(), std::io::Error> {
        let context = McpInstancesContext {
            db_get: get,
            db_search: search,
        };

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

        for (id, folder_path) in self.context.db_get.list_folders().await {
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
        let result = self.context.db_get.get_one(folder_id as u32, path).await;

        return match result {
            Ok(result) => {
                let content =
                    ContentBlock::text(result.content.unwrap_or(String::from("No content")));
                Ok(CallToolResult::success(vec![content]))
            }
            Err(error) => {
                let content = ContentBlock::text(format!("Error: {}", error.to_string()));

                Ok(CallToolResult::error(vec![content]))
            }
        };
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
        let search_results = self
            .context
            .db_search
            .search(
                folder_id as u32,
                query,
                limit,
                SearchContent::ContentAndPath,
            )
            .await;

        match search_results {
            Err(error) => {
                let error_text = ContentBlock::text(format!("Error while searching: {}", error));
                return Ok(CallToolResult::error(vec![error_text]));
            }

            Ok(search_results) => {
                let results: Vec<String> = search_results
                    .iter()
                    .map(|r| {
                        format!(
                            "Path: {}, Last modification: {}",
                            r.path,
                            timestamp_to_string(
                                r.modification_date * 1000, // it's in MS!
                                &String::from("%Y %m %d %H:%M:%S")
                            )
                        )
                    })
                    .collect();

                let content = ContentBlock::text(results.join("\n"));

                return Ok(CallToolResult::success(vec![content]));
            }
        }
    }
}

#[tool_handler]
impl ServerHandler for McpServer {
    /// Gets the server informations
    /// Returns:
    /// - the server infos
    fn get_info(&self) -> ServerConfig {
        return ServerConfig::new(
            ServerCapabilities::builder()
                .enable_tools()
                .enable_tool_list_changed()
                .build(),
        );
    }
}
