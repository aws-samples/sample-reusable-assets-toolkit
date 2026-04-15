use std::collections::HashMap;

use lambda_runtime::Error;
use rat_core::api::{RepoSearchRequest, RepoSearchResponse, RepoSearchResult};
use rat_core::embedding;
use rat_core::queries::{self, RepoRow};
use tracing::info;

use crate::AppState;

const RRF_K: f64 = 60.0;
const SEARCH_POOL_SIZE: i64 = 50;

pub async fn handle_repo_search(
    state: &AppState,
    req: RepoSearchRequest,
) -> Result<RepoSearchResponse, Error> {
    info!(query = %req.query, "Repo search request");

    let query_embedding =
        embedding::generate_embedding(&state.bedrock, &req.query, "GENERIC_RETRIEVAL").await?;

    let (fts_rows, vec_rows) = tokio::join!(
        queries::full_text_search_repos(&state.pool, &req.query, SEARCH_POOL_SIZE),
        queries::vector_search_repos(&state.pool, &query_embedding, SEARCH_POOL_SIZE),
    );

    let fts_rows = fts_rows?;
    let vec_rows = vec_rows?;

    info!(
        fts_count = fts_rows.len(),
        vec_count = vec_rows.len(),
        "Repo search results"
    );

    let results = fuse_rrf(fts_rows, vec_rows, req.limit as usize);

    Ok(RepoSearchResponse { results })
}

fn fuse_rrf(
    fts_rows: Vec<RepoRow>,
    vec_rows: Vec<RepoRow>,
    limit: usize,
) -> Vec<RepoSearchResult> {
    let mut scores: HashMap<String, f64> = HashMap::new();
    let mut repos: HashMap<String, RepoRow> = HashMap::new();

    for (rank, row) in fts_rows.into_iter().enumerate() {
        let rrf = 1.0 / (RRF_K + rank as f64 + 1.0);
        *scores.entry(row.repo_id.clone()).or_default() += rrf;
        repos.entry(row.repo_id.clone()).or_insert(row);
    }

    for (rank, row) in vec_rows.into_iter().enumerate() {
        let rrf = 1.0 / (RRF_K + rank as f64 + 1.0);
        *scores.entry(row.repo_id.clone()).or_default() += rrf;
        repos.entry(row.repo_id.clone()).or_insert(row);
    }

    let mut results: Vec<RepoSearchResult> = repos
        .into_iter()
        .map(|(id, repo)| RepoSearchResult {
            score: scores[&id],
            repo,
        })
        .collect();

    results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
    results.truncate(limit);
    results
}
