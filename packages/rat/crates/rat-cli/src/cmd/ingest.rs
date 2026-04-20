use std::path::{Path, PathBuf};

use anyhow::Result;
use aws_sdk_sqs::Client as SqsClient;
use dialoguer::console::Style;
use dialoguer::theme::ColorfulTheme;
use dialoguer::{Input, Select};

use crate::git::short_commit;
use crate::session::CliSession;
use crate::{chunk, git, ratignore, sqs as sqs_helper};
use rat_client::api_client;
use rat_core::message::{Action, ChunkEntry, FileMessage, SourceType};
use rat_core::queries::RepoRow;

// ── Repo state classification (pure) ────────────────────────────

enum RepoState<'a> {
    /// Repo row not yet created.
    NotIndexed,
    /// Row exists but `indexed_commit_id` is NULL (previous run interrupted).
    Interrupted(&'a RepoRow),
    /// Already indexed at the current commit.
    AlreadyIndexed(&'a RepoRow),
    /// Indexed at a different commit — can be updated incrementally.
    OutOfDate(&'a RepoRow),
}

impl<'a> RepoState<'a> {
    fn classify(existing: Option<&'a RepoRow>, commit_id: &str) -> Self {
        match existing {
            None => Self::NotIndexed,
            Some(info) if info.indexed_commit_id.is_none() => Self::Interrupted(info),
            Some(info) if info.indexed_commit_id.as_deref() == Some(commit_id) => {
                Self::AlreadyIndexed(info)
            }
            Some(info) => Self::OutOfDate(info),
        }
    }
}

enum IngestMode {
    Full,
    Incremental { since: String },
}

// ── Main handler ────────────────────────────────────────────────

pub async fn handle(target: &str, force: bool, profile_name: Option<&str>) -> Result<()> {
    let target_path = Path::new(target).canonicalize()?;
    let repo_root = git::discover_repo_root(&target_path)?;

    let theme = ColorfulTheme {
        active_item_style: Style::new().color256(183),
        active_item_prefix: dialoguer::console::style("❯ ".to_string()).color256(183),
        inactive_item_prefix: dialoguer::console::style("  ".to_string()),
        ..ColorfulTheme::default()
    };

    if target_path != repo_root {
        eprintln!(
            "Target '{}' is not the repository root; using detected root: {}",
            target_path.display(),
            repo_root.display()
        );
    }

    let default_branch = git::default_branch(&repo_root)?;
    let current_branch = git::current_branch(&repo_root)?
        .unwrap_or_else(|| "HEAD".to_string());
    let commit_id = git::branch_commit_id(&repo_root, &default_branch)?;

    let repo_id = match git::select_remote_url(&repo_root)? {
        Some(url) => git::canonicalize_remote_url(&url),
        None => {
            eprintln!("No git remote found for this repository.");
            Input::<String>::with_theme(&theme)
                .with_prompt("Enter a repository ID")
                .interact_text()?
        }
    };

    if current_branch != default_branch {
        eprintln!(
            "Note: indexing default branch '{}', not current '{}'.",
            default_branch, current_branch
        );
    }

    eprintln!("Repository : {}", repo_id);
    eprintln!("Branch     : {} ({})", default_branch, short_commit(&commit_id));

    // 설정 로드 및 AWS 클라이언트 초기화
    let session = CliSession::init(profile_name).await?;
    let sqs = SqsClient::new(&session.aws_config);
    let lambda = aws_sdk_lambda::Client::new(&session.aws_config);
    let profile = &session.profile;

    // repo 상태 조회 → 분류 → 계획 결정 (프롬프트 없음)
    eprintln!("Checking repository state...");
    let existing = api_client::fetch_repo(&lambda, &profile.api_function_arn, &repo_id).await?;
    let state = RepoState::classify(existing.as_ref(), &commit_id);
    let mode = match plan_mode(&state, force) {
        Some(m) => m,
        None => {
            eprintln!("Already indexed at {}. Nothing to do.", short_commit(&commit_id));
            return Ok(());
        }
    };

    // 파일 목록 결정
    let ignore = ratignore::load(&repo_root, &[]);
    let (target_files, deleted_files): (Vec<PathBuf>, Vec<PathBuf>) = match &mode {
        IngestMode::Incremental { since } => {
            eprintln!(
                "Incremental : from {} to {}",
                short_commit(since),
                short_commit(&commit_id)
            );
            let diff = git::diff_between_commits(&repo_root, since, &commit_id)?;
            (diff.changed, diff.deleted)
        }
        IngestMode::Full => {
            let files = git::list_files_at_branch(&repo_root, &default_branch, None)?;
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

    // 최종 confirm
    if !confirm(&theme, "Proceed with ingest?", true)? {
        eprintln!("Aborted.");
        return Ok(());
    }

    // 신규 repo인 경우에만 사전 row 생성 (commit_id는 NULL — 완료 후 갱신)
    if existing.is_none() {
        eprintln!("Registering repo...");
        api_client::upsert_repo(
            &lambda,
            &profile.api_function_arn,
            &repo_id,
            &default_branch,
            None,
            None,
        )
        .await?;
    }

    // 파일 메시지 전송 (Ctrl-C 감지)
    let send_result = tokio::select! {
        r = send_all_messages(
            &sqs,
            &profile.sqs_queue_url,
            &repo_id,
            &repo_root,
            &deleted_files,
            &supported,
        ) => r,
        _ = tokio::signal::ctrl_c() => {
            eprintln!();
            Err(anyhow::anyhow!("interrupted by user"))
        }
    };

    if let Err(e) = send_result {
        eprintln!();
        eprintln!("⚠ Ingest did not complete: {e}");
        if existing.is_none() {
            eprintln!("  Repository row was created but indexed_commit_id remains unset.");
            eprintln!("  Re-run `rat ingest` to retry (it will start from scratch).");
        } else {
            eprintln!("  Re-run `rat ingest --force` to fully re-index.");
        }
        return Err(e);
    }

    // README 읽기 (있으면 서버가 description + embedding 생성)
    let readme = read_readme(&repo_root);
    if readme.is_some() {
        eprintln!(
            "Finalizing repo state at {} (generating description from README)...",
            short_commit(&commit_id)
        );
    } else {
        eprintln!("Finalizing repo state at {}...", short_commit(&commit_id));
    }

    api_client::upsert_repo(
        &lambda,
        &profile.api_function_arn,
        &repo_id,
        &default_branch,
        Some(&commit_id),
        readme.as_deref(),
    )
    .await?;

    Ok(())
}

/// Repo root에서 `README.md`를 대소문자 구분 없이 찾아 내용을 반환.
fn read_readme(repo_root: &Path) -> Option<String> {
    let entries = std::fs::read_dir(repo_root).ok()?;
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if name_str.to_lowercase() == "readme.md" {
            if let Ok(content) = std::fs::read_to_string(entry.path()) {
                if !content.trim().is_empty() {
                    return Some(content);
                }
            }
        }
    }
    None
}

/// Yes/No 프롬프트. `default_yes`로 어느 쪽이 안전 기본값인지 지정.
fn confirm(theme: &ColorfulTheme, prompt: impl Into<String>, default_yes: bool) -> Result<bool> {
    let items: &[&str] = if default_yes {
        &["Yes", "No"]
    } else {
        &["No", "Yes"]
    };
    let selection = Select::with_theme(theme)
        .with_prompt(prompt.into())
        .items(items)
        .default(0)
        .interact()?;
    Ok(if default_yes {
        selection == 0
    } else {
        selection == 1
    })
}

/// 상태 + force 플래그로부터 실행 계획을 결정. 프롬프트는 하지 않고 안내 메시지만 출력.
/// `None`이면 할 일이 없어 종료.
fn plan_mode(state: &RepoState<'_>, force: bool) -> Option<IngestMode> {
    match (state, force) {
        (RepoState::NotIndexed, _) => {
            eprintln!("Plan       : create and ingest entire repository");
            Some(IngestMode::Full)
        }
        (
            RepoState::Interrupted(info)
            | RepoState::AlreadyIndexed(info)
            | RepoState::OutOfDate(info),
            true,
        ) => {
            eprintln!(
                "Plan       : force re-index (existing: {} files, {} snippets at {})",
                info.file_count,
                info.snippet_count,
                info.indexed_commit_id
                    .as_deref()
                    .map(short_commit)
                    .unwrap_or("-"),
            );
            Some(IngestMode::Full)
        }
        (RepoState::Interrupted(_), false) => {
            eprintln!("Plan       : re-index entire repository (previous ingest was interrupted)");
            Some(IngestMode::Full)
        }
        (RepoState::AlreadyIndexed(_), false) => None,
        (RepoState::OutOfDate(info), false) => {
            let prev = info.indexed_commit_id.as_deref().unwrap();
            eprintln!(
                "Plan       : incremental update from {} (branch {})",
                short_commit(prev),
                info.branch
            );
            Some(IngestMode::Incremental {
                since: prev.to_string(),
            })
        }
    }
}

async fn send_all_messages(
    sqs: &SqsClient,
    queue_url: &str,
    repo_id: &str,
    repo_root: &Path,
    deleted_files: &[PathBuf],
    supported: &[&PathBuf],
) -> Result<()> {
    for file in deleted_files {
        eprintln!("[delete] {}", file.display());
        let msg = FileMessage {
            action: Action::Delete,
            repo_id: repo_id.to_string(),
            source_path: file.display().to_string(),
            content: None,
            chunks: Vec::new(),
        };
        sqs_helper::send_file_message(sqs, queue_url, &msg).await?;
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

        let source_type = SourceType::from_path(file);
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
            repo_id: repo_id.to_string(),
            source_path: file.display().to_string(),
            content: Some(content),
            chunks: chunk_entries,
        };

        eprintln!("[{}/{}] {}", i + 1, total_files, file.display());
        sqs_helper::send_file_message(sqs, queue_url, &msg).await?;
    }
    eprintln!("{} chunks from {} files sent.", total_chunks, total_files);

    Ok(())
}

