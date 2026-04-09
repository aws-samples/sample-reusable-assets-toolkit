use std::path::{Path, PathBuf};

use clap::Parser;
use dialoguer::console::Style;
use dialoguer::theme::ColorfulTheme;
use dialoguer::Select;
use dialoguer::Confirm;
use rat_cli::message::{Action, ChunkEntry, FileMessage, SourceType};
use rat_cli::{chunk, git, ratignore};

#[derive(Parser)]
#[command(name = "rat", about = "Reusable Asset Toolkit")]
enum Cli {
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
}

fn source_type_for(path: &Path) -> SourceType {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    if ext == "md" {
        SourceType::Doc
    } else {
        SourceType::Code
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli {
        Cli::Ingest { target, force, since } => {
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

            // since가 있으면 diff 기반, 없으면 전체 파일 목록
            let (target_files, deleted_files): (Vec<PathBuf>, Vec<PathBuf>) = match &since {
                Some(prev_commit) => {
                    eprintln!("Incremental : from {} to {}", &prev_commit[..8.min(prev_commit.len())], &commit_id[..8]);
                    let diff = git::diff_between_commits(&repo_root, prev_commit, &commit_id)?;
                    let filter_by_prefix = |p: &PathBuf| match prefix.as_ref() {
                        Some(pre) => p.starts_with(pre),
                        None => true,
                    };
                    let changed: Vec<_> = diff.changed.into_iter().filter(filter_by_prefix).collect();
                    let deleted: Vec<_> = diff.deleted.into_iter().filter(filter_by_prefix).collect();
                    (changed, deleted)
                }
                None => {
                    let files = git::list_files_at_branch(&repo_root, &default_branch, prefix.as_deref())?;
                    (files, Vec::new())
                }
            };

            let supported: Vec<&PathBuf> = target_files
                .iter()
                .filter(|f| !ratignore::is_ignored(&ignore, &repo_root.join(f), false) && chunk::is_supported(f))
                .collect();
            eprintln!(
                "{}/{} supported files ({} deleted)",
                supported.len(),
                target_files.len(),
                deleted_files.len()
            );

            // force: 레포 전체 삭제 메시지 먼저 전송
            if force {
                let purge = FileMessage {
                    action: Action::Purge,
                    repo_id: repo_url.clone(),
                    commit_id: commit_id.clone(),
                    source_path: None,
                    content: None,
                    chunks: Vec::new(),
                };
                println!("{}", serde_json::to_string(&purge)?);
            }

            // 삭제 메시지 전송
            for file in &deleted_files {
                let msg = FileMessage {
                    action: Action::Delete,
                    repo_id: repo_url.clone(),
                    commit_id: commit_id.clone(),
                    source_path: Some(file.display().to_string()),
                    content: None,
                    chunks: Vec::new(),
                };
                println!("{}", serde_json::to_string(&msg)?);
            }

            let mut total_chunks = 0;
            for file in &supported {
                let abs_path = repo_root.join(file);
                let chunks = match chunk::chunk_file(&abs_path) {
                    Ok(c) => c,
                    Err(e) => {
                        eprintln!("Warning: failed to chunk {}: {e}", file.display());
                        continue;
                    }
                };

                let content = match std::fs::read_to_string(&abs_path) {
                    Ok(s) => s,
                    Err(e) => {
                        eprintln!("Warning: failed to read {}: {e}", file.display());
                        continue;
                    }
                };

                let source_type = source_type_for(file);
                let chunk_entries: Vec<ChunkEntry> = chunks
                    .iter()
                    .map(|c| ChunkEntry {
                        source_type: source_type.clone(),
                        start_line: c.start_line,
                        end_line: c.end_line,
                        content: if c.imports.is_empty() {
                            c.content.clone()
                        } else {
                            format!("{}\n\n{}", c.imports, c.content)
                        },
                    })
                    .collect();

                total_chunks += chunk_entries.len();

                let msg = FileMessage {
                    action: Action::Upsert,
                    repo_id: repo_url.clone(),
                    commit_id: commit_id.clone(),
                    source_path: Some(file.display().to_string()),
                    content: Some(content),
                    chunks: chunk_entries,
                };

                println!("{}", serde_json::to_string(&msg)?);
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
