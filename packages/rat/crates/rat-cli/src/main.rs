mod cmd;

use clap::Parser;

#[derive(Parser)]
#[command(name = "rat", version, about = "Reusable Asset Toolkit")]
enum Cli {
    /// Configure server endpoint and credentials
    Configure {
        #[command(subcommand)]
        action: Option<cmd::configure::ConfigureAction>,
        /// Profile name (default: "default")
        #[arg(long, global = true)]
        profile: Option<String>,
    },
    /// Ingest a repository
    Ingest {
        /// Local path to the repository
        target: String,
        /// Force re-indexing (re-index every file regardless of commit state).
        #[arg(long)]
        force: bool,
        /// Profile name (default: "default")
        #[arg(long)]
        profile: Option<String>,
    },
    /// Chunk a file using tree-sitter AST
    Chunk {
        /// Path to the file to chunk
        file: String,
    },
    /// Check indexing status
    Status {
        /// Profile name (default: "default")
        #[arg(long)]
        profile: Option<String>,
    },
    /// Authenticate with Cognito (browser-based OIDC PKCE)
    Login {
        /// Show current token status instead of logging in
        #[arg(long)]
        status: bool,
        /// Profile name (default: "default")
        #[arg(long)]
        profile: Option<String>,
    },
    /// Remove stored credentials
    Logout {
        /// Profile name (default: "default")
        #[arg(long)]
        profile: Option<String>,
    },
    /// List indexed repositories
    List {
        /// Profile name (default: "default")
        #[arg(long)]
        profile: Option<String>,
    },
    /// Purge all indexed data for a repository
    Purge {
        /// Repository ID to purge
        repo_id: String,
        /// Profile name (default: "default")
        #[arg(long)]
        profile: Option<String>,
    },
    /// Run rat as a stdio MCP server exposing the search tool
    Mcp {
        /// Profile name (default: "default")
        #[arg(long)]
        profile: Option<String>,
    },
    /// Run database migrations via the migration Lambda
    Migration {
        /// Drop all tables and re-run migrations from scratch (DESTRUCTIVE)
        #[arg(long)]
        reset: bool,
        /// Profile name (default: "default")
        #[arg(long)]
        profile: Option<String>,
    },
    /// Search code snippets
    Search {
        /// Search query
        query: String,
        /// Filter by repository ID
        #[arg(long)]
        repo_id: Option<String>,
        /// Filter by source type (code, doc)
        #[arg(long, default_value = "code")]
        source_type: String,
        /// Maximum number of results
        #[arg(long, default_value = "3")]
        limit: i64,
        /// Profile name (default: "default")
        #[arg(long)]
        profile: Option<String>,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli {
        Cli::Configure { action, profile } => {
            cmd::configure::handle(action, profile.as_deref())?;
        }
        Cli::Ingest { target, force, profile } => {
            cmd::ingest::handle(&target, force, profile.as_deref()).await?;
        }
        Cli::Chunk { file } => {
            cmd::chunk::handle(&file)?;
        }
        Cli::Status { profile } => {
            cmd::status::handle(profile.as_deref()).await?;
        }
        Cli::Login { status, profile } => {
            cmd::login::handle(profile.as_deref(), status).await?;
        }
        Cli::Logout { profile } => {
            cmd::login::logout(profile.as_deref())?;
        }
        Cli::List { profile } => {
            cmd::list::handle(profile.as_deref()).await?;
        }
        Cli::Purge { repo_id, profile } => {
            cmd::purge::handle(&repo_id, profile.as_deref()).await?;
        }
        Cli::Mcp { profile } => {
            cmd::mcp::handle(profile.as_deref()).await?;
        }
        Cli::Migration { reset, profile } => {
            cmd::migration::handle(reset, profile.as_deref()).await?;
        }
        Cli::Search { query, repo_id, source_type, limit, profile } => {
            cmd::search::handle(&query, repo_id.as_deref(), &source_type, limit, profile.as_deref()).await?;
        }
    }

    Ok(())
}
