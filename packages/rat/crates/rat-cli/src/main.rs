use std::path::Path;

use clap::Parser;
use dialoguer::console::Style;
use dialoguer::theme::ColorfulTheme;
use dialoguer::Select;
mod git;

#[derive(Parser)]
#[command(name = "rat", about = "Reusable Asset Toolkit")]
enum Cli {
    /// Ingest a repository
    Ingest {
        /// Local path to the repository
        target: String,
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
        Cli::Ingest { target, force: _ } => {
            let target_path = Path::new(&target).canonicalize()?;
            let repo_root = git::discover_repo_root(&target_path)?;

            let prefix = if target_path != repo_root {
                let rel = target_path.strip_prefix(&repo_root)?;
                eprintln!(
                    "Warning: '{}' is not the repository root (root: {})",
                    target_path.display(),
                    repo_root.display()
                );

                let items = vec![
                    format!("Current folder only ({})", rel.display()),
                    format!("Entire repository ({})", repo_root.display()),
                ];

                let theme = ColorfulTheme {
                    active_item_style: Style::new().color256(183), // light purple
                    active_item_prefix: dialoguer::console::style("❯ ".to_string()).color256(183),
                    inactive_item_prefix: dialoguer::console::style("  ".to_string()),
                    ..ColorfulTheme::default()
                };

                let selection = Select::with_theme(&theme)
                    .with_prompt("Select scope")
                    .items(&items)
                    .default(0)
                    .interact()?;

                match selection {
                    0 => Some(rel.to_path_buf()),
                    _ => None,
                }
            } else {
                None
            };

            let files = git::list_files(&repo_root, prefix.as_deref())?;
            for file in &files {
                println!("{}", file.display());
            }
            println!("\n{} files found", files.len());
        }
        Cli::Status => {
            println!("Status: OK");
        }
    }

    Ok(())
}
