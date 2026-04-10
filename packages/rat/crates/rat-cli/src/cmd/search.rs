use anyhow::{bail, Context, Result};
use aws_sdk_lambda::primitives::Blob;
use serde::{Deserialize, Serialize};

use rat_cli::aws;
use rat_cli::config;

#[derive(Serialize)]
struct SearchRequest<'a> {
    query: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    repo_id: Option<&'a str>,
    limit: i64,
}

#[derive(Deserialize)]
struct SearchResponse {
    results: Vec<SearchResult>,
}

#[derive(Deserialize)]
struct SearchResult {
    id: i64,
    repo_id: String,
    source_path: String,
    content: String,
    description: String,
    source_type: String,
    symbol_name: Option<String>,
    start_line: Option<i32>,
    end_line: Option<i32>,
    language: Option<String>,
    score: f64,
}

pub async fn handle(
    query: &str,
    repo_id: Option<&str>,
    limit: i64,
    profile_name: Option<&str>,
) -> Result<()> {
    let cfg = config::load_config()?.context("No configuration found. Run `rat configure` first.")?;
    let mut profile = config::resolve_profile(&cfg, profile_name)
        .context("Profile not found")?;
    let token = config::load_valid_token(&profile, profile_name).await?
        .context("Not logged in. Run `rat login` first.")?;

    let aws_config = aws::load_aws_config(&profile, &token).await?;
    let ssm = aws_sdk_ssm::Client::new(&aws_config);
    aws::resolve_ssm_values(profile_name, &mut profile, &ssm).await?;

    anyhow::ensure!(!profile.search_function_arn.is_empty(), "search_function_arn not configured");
    let function_arn = &profile.search_function_arn;

    let lambda_client = aws_sdk_lambda::Client::new(&aws_config);

    let request = SearchRequest {
        query,
        repo_id,
        limit,
    };
    let payload = serde_json::to_vec(&request)?;

    let response = lambda_client
        .invoke()
        .function_name(function_arn)
        .payload(Blob::new(payload))
        .send()
        .await
        .context("failed to invoke search Lambda")?;

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

    let search_response: SearchResponse =
        serde_json::from_slice(payload.as_ref()).context("failed to parse search response")?;

    for result in &search_response.results {
        println!("─── [{}] {} (score: {:.4}) ───", result.id, result.source_path, result.score);
        print!("  repo: {}  type: {}", result.repo_id, result.source_type);
        if let Some(ref symbol) = result.symbol_name {
            print!("  symbol: {}", symbol);
        }
        if let (Some(start), Some(end)) = (result.start_line, result.end_line) {
            print!("  lines: {}-{}", start, end);
        }
        if let Some(ref lang) = result.language {
            print!("  lang: {}", lang);
        }
        println!();
        println!("  {}", result.description);
        println!();
        println!("{}", result.content);
        println!();
    }

    if search_response.results.is_empty() {
        println!("No results found.");
    }

    Ok(())
}
