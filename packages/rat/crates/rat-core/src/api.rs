use serde::{Deserialize, Serialize};

use crate::queries::{FileListRow, FileRow, RepoRow, SnippetRow};

// ── Requests ──

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchRequest {
    pub query: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repo_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_type: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: i64,
}

fn default_limit() -> i64 {
    3
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ListRequest {}

#[derive(Debug, Serialize, Deserialize)]
pub struct PurgeRequest {
    pub repo_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RepoUpsertRequest {
    pub repo_id: String,
    pub branch: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub commit_id: Option<String>,
    /// Optional README content. If provided, the server generates a
    /// description + embedding and stores them on the repo row.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub readme: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RepoGetRequest {
    pub repo_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FileGetRequest {
    pub repo_id: String,
    pub source_path: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FileListRequest {
    pub repo_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SnippetListRequest {
    pub repo_id: String,
    pub source_path: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RepoSearchRequest {
    pub query: String,
    #[serde(default = "default_repo_limit")]
    pub limit: i64,
}

fn default_repo_limit() -> i64 {
    5
}

// ── Responses ──

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchResult {
    #[serde(flatten)]
    pub snippet: SnippetRow,
    pub score: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchResponse {
    pub results: Vec<SearchResult>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ListResponse {
    pub repos: Vec<RepoRow>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PurgeResponse {
    pub repo_id: String,
    pub found: bool,
    pub deleted_files: i64,
    pub deleted_snippets: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RepoUpsertResponse {
    pub repo_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RepoGetResponse {
    pub repo: Option<RepoRow>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FileGetResponse {
    pub file: Option<FileRow>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FileListResponse {
    pub files: Vec<FileListRow>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SnippetListResponse {
    pub snippets: Vec<SnippetRow>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RepoSearchResult {
    #[serde(flatten)]
    pub repo: RepoRow,
    pub score: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RepoSearchResponse {
    pub results: Vec<RepoSearchResult>,
}

// ── Routing enums ──

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum ApiRequest {
    Search(SearchRequest),
    List(ListRequest),
    Purge(PurgeRequest),
    RepoUpsert(RepoUpsertRequest),
    RepoGet(RepoGetRequest),
    RepoSearch(RepoSearchRequest),
    FileGet(FileGetRequest),
    FileList(FileListRequest),
    SnippetList(SnippetListRequest),
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ApiResponse {
    Search(SearchResponse),
    List(ListResponse),
    Purge(PurgeResponse),
    RepoUpsert(RepoUpsertResponse),
    RepoGet(RepoGetResponse),
    RepoSearch(RepoSearchResponse),
    FileGet(FileGetResponse),
    FileList(FileListResponse),
    SnippetList(SnippetListResponse),
}
