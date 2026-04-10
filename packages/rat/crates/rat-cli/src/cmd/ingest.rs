use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use aws_sdk_sqs::Client as SqsClient;
use dialoguer::console::Style;
use dialoguer::theme::ColorfulTheme;
use dialoguer::{Confirm, Select};

use rat_cli::{aws, config};
use rat_core::message::{Action, ChunkEntry, FileMessage, SourceType};
use rat_cli::{chunk, git, ratignore};

fn source_type_for(path: &Path) -> SourceType {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    if ext == "md" {
        SourceType::Doc
    } else {
        SourceType::Code
    }
}

async fn send_message(sqs: &SqsClient, queue_url: &str, msg: &FileMessage) -> Result<()> {
    let body = serde_json::to_string(msg)?;
    sqs.send_message()
        .queue_url(queue_url)
        .message_body(body)
        .send()
        .await
        .context("failed to send SQS message")?;
    Ok(())
}

pub async fn handle(target: &str, force: bool, since: Option<&str>, profile_name: Option<&str>) -> Result<()> {
    let target_path = Path::new(target).canonicalize()?;
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

    // AWS 설정 및 SQS 클라이언트 초기화
    let cfg = config::load_config()?.context("No configuration found. Run `rat configure` first.")?;
    let mut profile = config::resolve_profile(&cfg, profile_name).context("Profile not found")?;
    let token = config::load_valid_token(&profile, profile_name).await?
        .context("Not logged in. Run `rat login` first.")?;

    let aws_config = aws::load_aws_config(&profile, &token).await?;
    let ssm = aws_sdk_ssm::Client::new(&aws_config);
    aws::resolve_ssm_values(profile_name, &mut profile, &ssm).await?;

    anyhow::ensure!(!profile.sqs_queue_url.is_empty(), "sqs_queue_url not configured");
    let queue_url = &profile.sqs_queue_url;
    let sqs = SqsClient::new(&aws_config);

    let extra_dirs: Vec<PathBuf> = prefix
        .as_ref()
        .map(|p| vec![repo_root.join(p)])
        .unwrap_or_default();
    let extra_refs: Vec<&Path> = extra_dirs.iter().map(|p| p.as_ref()).collect();
    let ignore = ratignore::load(&repo_root, &extra_refs);

    // since가 있으면 diff 기반, 없으면 전체 파일 목록
    let (target_files, deleted_files): (Vec<PathBuf>, Vec<PathBuf>) = match since {
        Some(prev_commit) => {
            eprintln!(
                "Incremental : from {} to {}",
                &prev_commit[..8.min(prev_commit.len())],
                &commit_id[..8]
            );
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
            let files =
                git::list_files_at_branch(&repo_root, &default_branch, prefix.as_deref())?;
            (files, Vec::new())
        }
    };

    let supported: Vec<&PathBuf> = target_files
        .iter()
        .filter(|f| {
            !ratignore::is_ignored(&ignore, &repo_root.join(f), false) && chunk::is_supported(f)
        })
        .collect();
    eprintln!(
        "{}/{} supported files ({} deleted)",
        supported.len(),
        target_files.len(),
        deleted_files.len()
    );

    // force: 레포 전체 삭제 메시지 먼저 전송
    if force {
        eprintln!("Sending purge message...");
        let purge = FileMessage {
            action: Action::Purge,
            repo_id: repo_url.clone(),
            commit_id: commit_id.clone(),
            source_path: None,
            content: None,
            chunks: Vec::new(),
        };
        send_message(&sqs, queue_url, &purge).await?;
    }

    // 삭제 메시지 전송
    for file in &deleted_files {
        eprintln!("[delete] {}", file.display());
        let msg = FileMessage {
            action: Action::Delete,
            repo_id: repo_url.clone(),
            commit_id: commit_id.clone(),
            source_path: Some(file.display().to_string()),
            content: None,
            chunks: Vec::new(),
        };
        send_message(&sqs, queue_url, &msg).await?;
    }

    let mut total_chunks = 0;
    let total_files = supported.len();
    for (i, file) in supported.iter().enumerate() {
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

        eprintln!("[{}/{}] {}", i + 1, total_files, file.display());
        send_message(&sqs, queue_url, &msg).await?;
    }
    eprintln!("{} chunks from {} files sent.", total_chunks, total_files);

    Ok(())
}
