use anyhow::Result;
use rmcp::{
    ErrorData as McpError, ServerHandler, ServiceExt,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{CallToolResult, Content, Implementation, ProtocolVersion, ServerCapabilities, ServerInfo},
    schemars, tool, tool_handler, tool_router,
    transport::stdio,
};
use tracing_subscriber::EnvFilter;

use super::{list, search};

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct SearchParams {
    #[schemars(
        description = "Search query. MUST be written in English — translate non-English user input before calling this tool. Natural language or keywords both work."
    )]
    pub query: String,
    #[schemars(description = "Optional repository id to restrict the search to")]
    #[serde(default)]
    pub repo_id: Option<String>,
    #[schemars(description = "Source type filter: 'code' or 'doc' (default: code)")]
    #[serde(default)]
    pub source_type: Option<String>,
    #[schemars(description = "Maximum number of results (default: 3)")]
    #[serde(default)]
    pub limit: Option<i64>,
}

#[derive(Clone)]
pub struct RatMcpServer {
    profile_name: Option<String>,
    #[allow(dead_code)]
    tool_router: ToolRouter<RatMcpServer>,
}

#[tool_router]
impl RatMcpServer {
    pub fn new(profile_name: Option<String>) -> Self {
        Self {
            profile_name,
            tool_router: Self::tool_router(),
        }
    }

    #[tool(
        description = "Search the rat reusable asset store for indexed code snippets and documentation. \
            Use this tool WHENEVER the user's request mentions \"rat\" (e.g. \"use rat\", \"search rat\", \"ask rat\") \
            or asks to find, reuse, or reference existing internal code assets, snippets, examples, or past implementations \
            from the team's reusable asset store. \
            IMPORTANT: the `query` parameter MUST be written in English. If the user's request is in another language, \
            translate the search intent to English before calling this tool. \
            Returns matching snippets ranked by relevance, including file path, repository, symbol, and source content."
    )]
    async fn search(
        &self,
        Parameters(params): Parameters<SearchParams>,
    ) -> Result<CallToolResult, McpError> {
        let source_type = params.source_type.as_deref().unwrap_or("code");
        let limit = params.limit.unwrap_or(3);

        let results = search::run_search(
            &params.query,
            params.repo_id.as_deref(),
            source_type,
            limit,
            self.profile_name.as_deref(),
        )
        .await
        .map_err(|e| McpError::internal_error(format!("search failed: {e:#}"), None))?;

        if results.is_empty() {
            return Ok(CallToolResult::success(vec![Content::text(
                "No results found.",
            )]));
        }

        let contents = results
            .iter()
            .map(|r| {
                let mut header = format!(
                    "─── [{}] {} (score: {:.4}) ───\n  repo: {}  type: {}",
                    r.id, r.source_path, r.score, r.repo_id, r.source_type
                );
                if let Some(symbol) = &r.symbol_name {
                    header.push_str(&format!("  symbol: {symbol}"));
                }
                if let (Some(start), Some(end)) = (r.start_line, r.end_line) {
                    header.push_str(&format!("  lines: {start}-{end}"));
                }
                if let Some(lang) = &r.language {
                    header.push_str(&format!("  lang: {lang}"));
                }
                Content::text(format!(
                    "{header}\n  {}\n\n{}\n",
                    r.description, r.content
                ))
            })
            .collect();

        Ok(CallToolResult::success(contents))
    }

    #[tool(
        description = "List all repositories indexed in the rat reusable asset store, with file and snippet counts. \
            Use this to discover which repositories are available before calling `search` with a `repo_id` filter, \
            or whenever the user asks what repos / assets are indexed in rat."
    )]
    async fn list_repos(&self) -> Result<CallToolResult, McpError> {
        let repos = list::run_list(self.profile_name.as_deref())
            .await
            .map_err(|e| McpError::internal_error(format!("list failed: {e:#}"), None))?;

        if repos.is_empty() {
            return Ok(CallToolResult::success(vec![Content::text(
                "No repositories indexed.",
            )]));
        }

        let mut text = format!(
            "{:<60}  {:<20}  {:<10}  {:>10}  {:>12}\n",
            "REPO_ID", "BRANCH", "COMMIT", "FILES", "SNIPPETS"
        );
        for repo in &repos {
            let short_commit = &repo.indexed_commit_id[..8.min(repo.indexed_commit_id.len())];
            text.push_str(&format!(
                "{:<60}  {:<20}  {:<10}  {:>10}  {:>12}\n",
                repo.repo_id, repo.branch, short_commit, repo.file_count, repo.snippet_count
            ));
        }

        Ok(CallToolResult::success(vec![Content::text(text)]))
    }
}

#[tool_handler]
impl ServerHandler for RatMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(
            ServerCapabilities::builder().enable_tools().build(),
        )
        .with_server_info(Implementation::from_build_env())
        .with_protocol_version(ProtocolVersion::V_2024_11_05)
        .with_instructions(
            "rat MCP server — reusable asset store for internal code snippets and documentation. \
             Call the `search` tool whenever the user mentions \"rat\" (e.g. \"use rat\", \"search rat\", \"ask rat\") \
             or asks to find/reuse existing internal code, examples, or past implementations. \
             The `query` argument MUST be in English; translate non-English user intent before calling."
                .to_string(),
        )
    }
}

pub async fn handle(profile_name: Option<&str>) -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();

    tracing::info!("Starting rat MCP server (stdio)");

    let server = RatMcpServer::new(profile_name.map(|s| s.to_string()));
    let service = server.serve(stdio()).await.inspect_err(|e| {
        tracing::error!("serving error: {:?}", e);
    })?;

    service.waiting().await?;
    Ok(())
}
