// SPDX-License-Identifier: MIT

//! Stateless API operation helpers.
//!
//! Each function takes an authenticated `LambdaClient` plus the API function ARN
//! and returns parsed response data. Callers (CLI, MCP Lambda, etc.) own the
//! credential / configuration story.

use anyhow::{Context, Result};
use aws_sdk_lambda::Client as LambdaClient;

use rat_core::api::{
    ApiRequest, FileGetRequest, FileGetResponse, ListRequest, ListResponse, RepoSearchRequest,
    RepoSearchResponse, RepoSearchResult, SearchRequest, SearchResponse, SearchResult,
};
use rat_core::queries::{FileRow, RepoRow};

use crate::api_client::invoke_api;

pub async fn search(
    lambda: &LambdaClient,
    function_arn: &str,
    query: &str,
    repo_id: Option<&str>,
    source_type: &str,
    limit: i64,
) -> Result<Vec<SearchResult>> {
    let request = ApiRequest::Search(SearchRequest {
        query: query.to_string(),
        repo_id: repo_id.map(|s| s.to_string()),
        source_type: Some(source_type.to_string()),
        limit,
    });
    let bytes = invoke_api(lambda, function_arn, &request).await?;
    let response: SearchResponse =
        serde_json::from_slice(&bytes).context("failed to parse search response")?;
    Ok(response.results)
}

pub async fn repo_search(
    lambda: &LambdaClient,
    function_arn: &str,
    query: &str,
    limit: i64,
) -> Result<Vec<RepoSearchResult>> {
    let request = ApiRequest::RepoSearch(RepoSearchRequest {
        query: query.to_string(),
        limit,
    });
    let bytes = invoke_api(lambda, function_arn, &request).await?;
    let response: RepoSearchResponse =
        serde_json::from_slice(&bytes).context("failed to parse repo_search response")?;
    Ok(response.results)
}

pub async fn list(lambda: &LambdaClient, function_arn: &str) -> Result<Vec<RepoRow>> {
    let bytes = invoke_api(lambda, function_arn, &ApiRequest::List(ListRequest {})).await?;
    let response: ListResponse =
        serde_json::from_slice(&bytes).context("failed to parse list response")?;
    Ok(response.repos)
}

pub async fn file_get(
    lambda: &LambdaClient,
    function_arn: &str,
    repo_id: &str,
    source_path: &str,
) -> Result<Option<FileRow>> {
    let request = ApiRequest::FileGet(FileGetRequest {
        repo_id: repo_id.to_string(),
        source_path: source_path.to_string(),
    });
    let bytes = invoke_api(lambda, function_arn, &request).await?;
    let response: FileGetResponse =
        serde_json::from_slice(&bytes).context("failed to parse file_get response")?;
    Ok(response.file)
}
