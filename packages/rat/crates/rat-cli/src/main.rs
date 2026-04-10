mod cmd;

use clap::Parser;

#[derive(Parser)]
#[command(name = "rat", about = "Reusable Asset Toolkit")]
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
        /// Force re-indexing (purge existing records and re-index everything).
        #[arg(long, conflicts_with = "since")]
        force: bool,
        /// Previous commit id. If provided, only changed/deleted files since this commit are processed.
        #[arg(long)]
        since: Option<String>,
    },
    /// Chunk a file using tree-sitter AST
    Chunk {
        /// Path to the file to chunk
        file: String,
    },
    /// Check indexing status
    Status,
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
    /// Search code snippets
    Search {
        /// Search query
        query: String,
        /// Filter by repository ID
        #[arg(long)]
        repo_id: Option<String>,
        /// Maximum number of results
        #[arg(long, default_value = "20")]
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
        Cli::Ingest { target, force, since } => {
            cmd::ingest::handle(&target, force, since.as_deref())?;
        }
        Cli::Chunk { file } => {
            cmd::chunk::handle(&file)?;
        }
        Cli::Status => {
            cmd::status::handle()?;
        }
        Cli::Login { status, profile } => {
            cmd::login::handle(profile.as_deref(), status).await?;
        }
        Cli::Logout { profile } => {
            cmd::login::logout(profile.as_deref())?;
        }
        Cli::Search { query, repo_id, limit, profile } => {
            cmd::search::handle(&query, repo_id.as_deref(), limit, profile.as_deref()).await?;
        }
    }

    Ok(())
}
