// SPDX-License-Identifier: MIT

use std::path::{Path, PathBuf};

use anyhow::Context;
use git2::Repository;

/// Returns the first 8 characters of a commit SHA for display.
pub fn short_commit(commit: &str) -> &str {
    &commit[..8.min(commit.len())]
}

/// Discovers the git repository root from the given path.
pub fn discover_repo_root(path: &Path) -> anyhow::Result<PathBuf> {
    let repo = Repository::discover(path)
        .with_context(|| format!("failed to find git repository from {}", path.display()))?;

    let workdir = repo
        .workdir()
        .context("bare repositories are not supported")?;

    Ok(workdir.to_path_buf())
}

/// Returns the current branch name (e.g. "main", "feature/foo").
pub fn current_branch(repo_root: &Path) -> anyhow::Result<Option<String>> {
    let repo = Repository::open(repo_root)
        .with_context(|| format!("failed to open git repository at {}", repo_root.display()))?;

    let head = repo.head().context("failed to get HEAD reference")?;
    Ok(head.shorthand().map(|s| s.to_string()))
}

/// Returns the default branch name by checking refs/remotes/origin/HEAD.
/// Falls back to "main" if not configured.
pub fn default_branch(repo_root: &Path) -> anyhow::Result<String> {
    let repo = Repository::open(repo_root)
        .with_context(|| format!("failed to open git repository at {}", repo_root.display()))?;

    let reference = repo.find_reference("refs/remotes/origin/HEAD");
    if let Ok(r) = reference {
        if let Some(target) = r.symbolic_target() {
            // "refs/remotes/origin/main" → "main"
            let branch = target.strip_prefix("refs/remotes/origin/").unwrap_or(target);
            return Ok(branch.to_string());
        }
    }

    // fallback: "main" or "master"
    for name in ["main", "master"] {
        if repo
            .find_branch(name, git2::BranchType::Local)
            .is_ok()
        {
            return Ok(name.to_string());
        }
    }

    Ok("main".to_string())
}

/// Returns the commit id (SHA) of the given branch.
pub fn branch_commit_id(repo_root: &Path, branch: &str) -> anyhow::Result<String> {
    let repo = Repository::open(repo_root)
        .with_context(|| format!("failed to open git repository at {}", repo_root.display()))?;

    let reference = repo
        .find_branch(branch, git2::BranchType::Local)
        .with_context(|| format!("branch '{branch}' not found"))?;

    let commit = reference
        .get()
        .peel_to_commit()
        .context("branch does not point to a commit")?;

    Ok(commit.id().to_string())
}

/// Diff result between two commits: changed (added/modified/renamed) and deleted files.
pub struct DiffResult {
    pub changed: Vec<PathBuf>,
    pub deleted: Vec<PathBuf>,
}

/// Returns file changes between two commits.
pub fn diff_between_commits(
    repo_root: &Path,
    from_commit: &str,
    to_commit: &str,
) -> anyhow::Result<DiffResult> {
    let repo = Repository::open(repo_root)
        .with_context(|| format!("failed to open git repository at {}", repo_root.display()))?;

    let from_oid = git2::Oid::from_str(from_commit)
        .with_context(|| format!("invalid commit id: {from_commit}"))?;
    let to_oid = git2::Oid::from_str(to_commit)
        .with_context(|| format!("invalid commit id: {to_commit}"))?;

    let from_tree = repo.find_commit(from_oid)?.tree()?;
    let to_tree = repo.find_commit(to_oid)?.tree()?;

    let diff = repo.diff_tree_to_tree(Some(&from_tree), Some(&to_tree), None)?;

    let mut changed = Vec::new();
    let mut deleted = Vec::new();

    diff.foreach(
        &mut |delta, _| {
            match delta.status() {
                git2::Delta::Added
                | git2::Delta::Modified
                | git2::Delta::Renamed
                | git2::Delta::Copied => {
                    if let Some(path) = delta.new_file().path() {
                        changed.push(path.to_path_buf());
                    }
                }
                git2::Delta::Deleted => {
                    if let Some(path) = delta.old_file().path() {
                        deleted.push(path.to_path_buf());
                    }
                }
                _ => {}
            }
            true
        },
        None,
        None,
        None,
    )?;

    Ok(DiffResult { changed, deleted })
}

/// Returns the remote "origin" URL, if configured.
pub fn remote_url(repo_root: &Path) -> anyhow::Result<Option<String>> {
    let repo = Repository::open(repo_root)
        .with_context(|| format!("failed to open git repository at {}", repo_root.display()))?;

    let remote = repo.find_remote("origin");
    Ok(remote.ok().and_then(|r| r.url().map(|u| u.to_string())))
}

/// Selects a remote URL using the following rules:
/// 1. Prefer the remote named `origin`.
/// 2. Otherwise, use the sole remote if exactly one exists.
/// 3. Otherwise, return `None`.
pub fn select_remote_url(repo_root: &Path) -> anyhow::Result<Option<String>> {
    let repo = Repository::open(repo_root)
        .with_context(|| format!("failed to open git repository at {}", repo_root.display()))?;

    if let Ok(remote) = repo.find_remote("origin") {
        if let Some(url) = remote.url() {
            return Ok(Some(url.to_string()));
        }
    }

    let names = repo.remotes().context("failed to list remotes")?;
    if names.len() == 1 {
        if let Some(name) = names.get(0) {
            if let Ok(remote) = repo.find_remote(name) {
                return Ok(remote.url().map(|u| u.to_string()));
            }
        }
    }

    Ok(None)
}

/// Canonicalizes a git remote URL to `host/owner/repo` form.
/// Handles SSH (`git@host:owner/repo.git`), `ssh://`, and `https://` variants.
pub fn canonicalize_remote_url(url: &str) -> String {
    let trimmed = url.trim();
    let without_git = trimmed.strip_suffix(".git").unwrap_or(trimmed);

    if !without_git.contains("://") {
        if let Some((authority, path)) = without_git.split_once(':') {
            let host = authority
                .rsplit_once('@')
                .map(|(_, h)| h)
                .unwrap_or(authority);
            return format!(
                "{}/{}",
                host.to_lowercase(),
                path.trim_start_matches('/').to_lowercase()
            );
        }
    }

    if let Some((_, rest)) = without_git.split_once("://") {
        let (authority, path) = rest.split_once('/').unwrap_or((rest, ""));
        let host = authority
            .rsplit_once('@')
            .map(|(_, h)| h)
            .unwrap_or(authority);
        let host = host.split_once(':').map(|(h, _)| h).unwrap_or(host);
        return format!("{}/{}", host.to_lowercase(), path.to_lowercase());
    }

    without_git.to_lowercase()
}

/// Lists files from a specific branch's tree, optionally filtered by a subdirectory prefix.
pub fn list_files_at_branch(
    repo_root: &Path,
    branch: &str,
    prefix: Option<&Path>,
) -> anyhow::Result<Vec<PathBuf>> {
    let repo = Repository::open(repo_root)
        .with_context(|| format!("failed to open git repository at {}", repo_root.display()))?;

    let reference = repo
        .find_branch(branch, git2::BranchType::Local)
        .with_context(|| format!("branch '{branch}' not found"))?;

    let commit = reference
        .get()
        .peel_to_commit()
        .context("branch does not point to a commit")?;

    let tree = commit.tree().context("failed to get commit tree")?;

    let mut files = Vec::new();
    tree.walk(git2::TreeWalkMode::PreOrder, |dir, entry| {
        if entry.kind() == Some(git2::ObjectType::Blob) {
            let path = PathBuf::from(format!("{}{}", dir, entry.name().unwrap_or("")));
            let include = match prefix {
                Some(p) => path.starts_with(p),
                None => true,
            };
            if include {
                files.push(path);
            }
        }
        git2::TreeWalkResult::Ok
    })?;

    Ok(files)
}
