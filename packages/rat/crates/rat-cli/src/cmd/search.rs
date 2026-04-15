use anyhow::{Context, Result};

use rat_cli::api_client;
use rat_cli::git::short_commit;
use rat_cli::highlight;
use rat_cli::session::CliSession;
use rat_core::api::{
    ApiRequest, RepoSearchRequest, RepoSearchResponse, RepoSearchResult, SearchRequest,
    SearchResponse, SearchResult,
};

use crate::SearchScope;

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

pub(crate) async fn run_repo_search(
    query: &str,
    limit: i64,
    profile_name: Option<&str>,
) -> Result<Vec<RepoSearchResult>> {
    let session = CliSession::init(profile_name).await?;
    let lambda = aws_sdk_lambda::Client::new(&session.aws_config);

    let request = ApiRequest::RepoSearch(RepoSearchRequest {
        query: query.to_string(),
        limit,
    });
    let bytes = api_client::invoke_api(&lambda, &session.profile.api_function_arn, &request).await?;
    let response: RepoSearchResponse =
        serde_json::from_slice(&bytes).context("failed to parse repo_search response")?;
    Ok(response.results)
}

pub async fn handle(
    query: &str,
    repo_id: Option<&str>,
    scope: SearchScope,
    limit: Option<i64>,
    profile_name: Option<&str>,
) -> Result<()> {
    match scope {
        SearchScope::Code | SearchScope::Doc => {
            let source_type = match scope {
                SearchScope::Code => "code",
                SearchScope::Doc => "doc",
                _ => unreachable!(),
            };
            let limit = limit.unwrap_or(3);
            let results = run_search(query, repo_id, source_type, limit, profile_name).await?;
            print_snippet_results(&results);
        }
        SearchScope::Repo => {
            let limit = limit.unwrap_or(5);
            let results = run_repo_search(query, limit, profile_name).await?;
            print_repo_results(&results);
        }
    }
    Ok(())
}

fn print_snippet_results(results: &[SearchResult]) {
    for result in results {
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
        println!();
        println!("{}", highlight::highlight(&s.content, s.language.as_deref()));
        println!();
    }

    if results.is_empty() {
        println!("No results found.");
    }
}

fn print_repo_results(results: &[RepoSearchResult]) {
    for result in results {
        let r = &result.repo;
        let commit = r
            .indexed_commit_id
            .as_deref()
            .map(short_commit)
            .unwrap_or("-");
        println!(
            "─── {} (score: {:.4}) ───",
            r.repo_id, result.score
        );
        println!(
            "  branch: {}  commit: {}  files: {}  snippets: {}",
            r.branch, commit, r.file_count, r.snippet_count
        );
        match r.description.as_deref() {
            Some(d) if !d.trim().is_empty() => println!("{}", d.trim()),
            _ => println!("(no description)"),
        }
        println!();
    }

    if results.is_empty() {
        println!("No results found.");
    }
}
