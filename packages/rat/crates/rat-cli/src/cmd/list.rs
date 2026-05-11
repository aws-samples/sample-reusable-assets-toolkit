// SPDX-License-Identifier: MIT

use anyhow::Result;

use crate::git::short_commit;
use crate::session::CliSession;
use rat_client::ops;
use rat_core::queries::RepoRow;

pub async fn run_list(profile_name: Option<&str>) -> Result<Vec<RepoRow>> {
    let session = CliSession::init(profile_name).await?;
    let lambda = aws_sdk_lambda::Client::new(&session.aws_config);
    ops::list(&lambda, &session.profile.api_function_arn).await
}

pub async fn handle(profile_name: Option<&str>) -> Result<()> {
    let repos = run_list(profile_name).await?;

    if repos.is_empty() {
        println!("No repositories indexed.");
        return Ok(());
    }

    println!(
        "{:<60}  {:<20}  {:<10}  {:>10}  {:>12}",
        "REPO_ID", "BRANCH", "COMMIT", "FILES", "SNIPPETS"
    );
    for repo in &repos {
        let commit = repo
            .indexed_commit_id
            .as_deref()
            .map(short_commit)
            .unwrap_or("-");
        println!(
            "{:<60}  {:<20}  {:<10}  {:>10}  {:>12}",
            repo.repo_id, repo.branch, commit, repo.file_count, repo.snippet_count
        );
        match repo.description.as_deref() {
            Some(d) if !d.trim().is_empty() => println!("{}", d.trim()),
            _ => println!("(no description)"),
        }
        println!();
    }

    Ok(())
}
