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

/// Lists tracked files in the git repository, optionally filtered by a subdirectory prefix.
pub fn list_files(repo_root: &Path, prefix: Option<&Path>) -> anyhow::Result<Vec<PathBuf>> {
    let repo = Repository::open(repo_root)
        .with_context(|| format!("failed to open git repository at {}", repo_root.display()))?;

    let index = repo.index().context("failed to read repository index")?;

    let files = index
        .iter()
        .map(|entry| {
            let path = String::from_utf8_lossy(&entry.path);
            PathBuf::from(path.as_ref())
        })
        .filter(|path| match prefix {
            Some(p) => path.starts_with(p),
            None => true,
        })
        .collect();

    Ok(files)
}
