use std::path::{Path, PathBuf};

use anyhow::Context;
use git2::Repository;

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

/// Returns the remote "origin" URL, if configured.
pub fn remote_url(repo_root: &Path) -> anyhow::Result<Option<String>> {
    let repo = Repository::open(repo_root)
        .with_context(|| format!("failed to open git repository at {}", repo_root.display()))?;

    let remote = repo.find_remote("origin");
    Ok(remote.ok().and_then(|r| r.url().map(|u| u.to_string())))
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
