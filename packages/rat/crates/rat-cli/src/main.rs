use std::path::{Path, PathBuf};

use clap::Parser;
use dialoguer::console::Style;
use dialoguer::theme::ColorfulTheme;
use dialoguer::Select;
use dialoguer::Confirm;
use rat_cli::{chunk, git, ratignore};

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
    /// Chunk a file using tree-sitter AST
    Chunk {
        /// Path to the file to chunk
        file: String,
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

            let repo_url = git::remote_url(&repo_root)?
                .unwrap_or_else(|| repo_root.display().to_string());
            let default_branch = git::default_branch(&repo_root)?;
            let current_branch = git::current_branch(&repo_root)?
                .unwrap_or_else(|| "HEAD".to_string());
            let commit_id = git::branch_commit_id(&repo_root, &default_branch)?;

            let theme = ColorfulTheme {
                active_item_style: Style::new().color256(183),
                active_item_prefix: dialoguer::console::style("❯ ".to_string()).color256(183),
                inactive_item_prefix: dialoguer::console::style("  ".to_string()),
                ..ColorfulTheme::default()
            };

            // scope 선택 (서브디렉토리인 경우)
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

            // 기본 브랜치 확인
            if current_branch != default_branch {
                eprintln!(
                    "Current branch '{}' differs from default branch '{}'.",
                    current_branch, default_branch
                );
                eprintln!("Indexing will use the default branch '{}'.", default_branch);
            }

            eprintln!("Repository : {}", repo_url);
            eprintln!("Branch     : {} ({})", default_branch, &commit_id[..8]);

            let confirmed = Confirm::with_theme(&theme)
                .with_prompt("Proceed with indexing?")
                .default(true)
                .interact()?;

            if !confirmed {
                eprintln!("Aborted.");
                return Ok(());
            }

            let extra_dirs: Vec<PathBuf> = prefix
                .as_ref()
                .map(|p| vec![repo_root.join(p)])
                .unwrap_or_default();
            let extra_refs: Vec<&Path> = extra_dirs.iter().map(|p| p.as_ref()).collect();
            let ignore = ratignore::load(&repo_root, &extra_refs);
            let files = git::list_files_at_branch(&repo_root, &default_branch, prefix.as_deref())?;
            let supported: Vec<_> = files
                .iter()
                .filter(|f| !ratignore::is_ignored(&ignore, &repo_root.join(f), false) && chunk::is_supported(f))
                .collect();
            eprintln!("{}/{} supported files", supported.len(), files.len());

            let mut total_chunks = 0;
            for file in &supported {
                let abs_path = repo_root.join(file);
                match chunk::chunk_file(&abs_path) {
                    Ok(chunks) => {
                        for c in &chunks {
                            println!(
                                "[{}] {} L{}-L{} {}",
                                repo_url,
                                file.display(),
                                c.start_line,
                                c.end_line,
                                c.symbol_name.as_deref().unwrap_or("")
                            );
                        }
                        total_chunks += chunks.len();
                    }
                    Err(e) => {
                        eprintln!("Warning: failed to chunk {}: {e}", file.display());
                    }
                }
            }
            eprintln!("{} chunks from {} files", total_chunks, supported.len());
        }
        Cli::Chunk { file } => {
            let path = Path::new(&file).canonicalize()?;
            let chunks = chunk::chunk_file(&path)?;
            for (i, c) in chunks.iter().enumerate() {
                println!("--- chunk {} (L{}-L{}) {} ---", i + 1, c.start_line, c.end_line, c.symbol_name.as_deref().unwrap_or(""));
                if !c.imports.is_empty() {
                    println!("[imports]\n{}\n", c.imports);
                }
                println!("{}", c.content);
                println!();
            }
            println!("{} chunks total", chunks.len());
        }
        Cli::Status => {
            println!("Status: OK");
        }
    }

    Ok(())
}
