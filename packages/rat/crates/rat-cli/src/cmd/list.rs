use anyhow::{bail, Context, Result};
use aws_sdk_lambda::primitives::Blob;
use serde::{Deserialize, Serialize};

use rat_cli::aws;
use rat_cli::config;

#[derive(Serialize)]
#[serde(tag = "action", rename_all = "snake_case")]
enum ApiRequest {
    List {},
}

#[derive(Deserialize)]
struct ListResponse {
    repos: Vec<RepoRow>,
}

#[derive(Deserialize)]
pub(crate) struct RepoRow {
    pub repo_id: String,
    pub branch: String,
    pub indexed_commit_id: String,
    pub file_count: i64,
    pub snippet_count: i64,
}

pub(crate) async fn run_list(profile_name: Option<&str>) -> Result<Vec<RepoRow>> {
    let cfg = config::load_config()?.context("No configuration found. Run `rat configure` first.")?;
    let mut profile = config::resolve_profile(&cfg, profile_name)
        .context("Profile not found")?;
    let token = config::load_valid_token(&profile, profile_name).await?
        .context("Not logged in. Run `rat login` first.")?;

    let aws_config = aws::load_aws_config(&profile, &token).await?;
    let ssm = aws_sdk_ssm::Client::new(&aws_config);
    aws::resolve_ssm_values(profile_name, &mut profile, &ssm).await?;

    anyhow::ensure!(!profile.api_function_arn.is_empty(), "api_function_arn not configured");
    let function_arn = &profile.api_function_arn;

    let lambda_client = aws_sdk_lambda::Client::new(&aws_config);

    let request = ApiRequest::List {};
    let payload = serde_json::to_vec(&request)?;

    let response = lambda_client
        .invoke()
        .function_name(function_arn)
        .payload(Blob::new(payload))
        .send()
        .await
        .context("failed to invoke API Lambda")?;

    if let Some(err) = response.function_error() {
        let body = response
            .payload()
            .map(|p| String::from_utf8_lossy(p.as_ref()).to_string())
            .unwrap_or_default();
        bail!("Lambda error ({}): {}", err, body);
    }

    let payload = response
        .payload()
        .context("no response payload from Lambda")?;

    let list_response: ListResponse =
        serde_json::from_slice(payload.as_ref()).context("failed to parse list response")?;

    Ok(list_response.repos)
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
        let short_commit = &repo.indexed_commit_id[..8.min(repo.indexed_commit_id.len())];
        println!(
            "{:<60}  {:<20}  {:<10}  {:>10}  {:>12}",
            repo.repo_id, repo.branch, short_commit, repo.file_count, repo.snippet_count
        );
    }

    Ok(())
}
