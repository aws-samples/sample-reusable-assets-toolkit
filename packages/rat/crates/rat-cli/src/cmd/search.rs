use anyhow::{Context, Result};

use rat_cli::api_client;
use rat_cli::session::CliSession;
use rat_core::api::{ApiRequest, SearchRequest, SearchResponse, SearchResult};

pub(crate) async fn run_search(
    query: &str,
    repo_id: Option<&str>,
    source_type: &str,
    limit: i64,
    profile_name: Option<&str>,
) -> Result<Vec<SearchResult>> {
    let session = CliSession::init(profile_name).await?;
    let lambda = aws_sdk_lambda::Client::new(&session.aws_config);

    let request = ApiRequest::Search(SearchRequest {
        query: query.to_string(),
        repo_id: repo_id.map(|s| s.to_string()),
        source_type: Some(source_type.to_string()),
        limit,
    });
    let bytes = api_client::invoke_api(&lambda, &session.profile.api_function_arn, &request).await?;
    let response: SearchResponse =
        serde_json::from_slice(&bytes).context("failed to parse search response")?;
    Ok(response.results)
}

pub async fn handle(
    query: &str,
    repo_id: Option<&str>,
    source_type: &str,
    limit: i64,
    profile_name: Option<&str>,
) -> Result<()> {
    let results = run_search(query, repo_id, source_type, limit, profile_name).await?;

    for result in &results {
        let s = &result.snippet;
        println!("─── [{}] {} (score: {:.4}) ───", s.id, s.source_path, result.score);
        print!("  repo: {}  type: {}", s.repo_id, s.source_type);
        if let Some(ref symbol) = s.symbol_name {
            print!("  symbol: {}", symbol);
        }
        if let (Some(start), Some(end)) = (s.start_line, s.end_line) {
            print!("  lines: {}-{}", start, end);
        }
        if let Some(ref lang) = s.language {
            print!("  lang: {}", lang);
        }
        println!();
        println!("  {}", s.description);
        println!();
        println!("{}", s.content);
        println!();
    }

    if results.is_empty() {
        println!("No results found.");
    }

    Ok(())
}
