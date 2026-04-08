use clap::Parser;

#[derive(Parser)]
#[command(name = "rat", about = "Reusable Asset Toolkit")]
enum Cli {
    /// Ingest a repository
    Ingest {
        /// Local path or remote URL
        target: String,
        /// Git branch (remote only)
        #[arg(long)]
        branch: Option<String>,
        /// Force re-indexing
        #[arg(long)]
        force: bool,
    },
    /// Check indexing status
    Status,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli {
        Cli::Ingest { target, branch, force } => {
            println!("Ingesting: {target} (branch: {branch:?}, force: {force})");
        }
        Cli::Status => {
            println!("Status: OK");
        }
    }

    Ok(())
}
