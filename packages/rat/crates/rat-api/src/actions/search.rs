use std::collections::HashMap;

use lambda_runtime::Error;
use rat_core::api::{SearchRequest, SearchResponse, SearchResult};
use rat_core::embedding;
use rat_core::queries::{self, SnippetRow};
use tracing::info;

use crate::AppState;

const RRF_K: f64 = 60.0;
const SEARCH_POOL_SIZE: i64 = 50;

pub async fn handle_search(
    state: &AppState,
    req: SearchRequest,
) -> Result<SearchResponse, Error> {
    info!(query = %req.query, repo_id = ?req.repo_id, "Search request");

    let query_embedding = embedding::generate_embedding(&state.bedrock, &req.query, "GENERIC_RETRIEVAL").await?;

    let (fts_rows, vec_rows) = tokio::join!(
        queries::full_text_search(
            &state.pool,
            &req.query,
            req.repo_id.as_deref(),
            req.source_type.as_deref(),
            SEARCH_POOL_SIZE,
        ),
        queries::vector_search(
            &state.pool,
            &query_embedding,
            req.repo_id.as_deref(),
            req.source_type.as_deref(),
            SEARCH_POOL_SIZE,
        ),
    );

    let fts_rows = fts_rows?;
    let vec_rows = vec_rows?;

    info!(
        fts_count = fts_rows.len(),
        vec_count = vec_rows.len(),
        "Search results"
    );

    let results = fuse_rrf(fts_rows, vec_rows, req.limit as usize);

    Ok(SearchResponse { results })
}

fn fuse_rrf(
    fts_rows: Vec<SnippetRow>,
    vec_rows: Vec<SnippetRow>,
    limit: usize,
) -> Vec<SearchResult> {
    let mut scores: HashMap<i64, f64> = HashMap::new();
    let mut snippets: HashMap<i64, SnippetRow> = HashMap::new();

    for (rank, row) in fts_rows.into_iter().enumerate() {
        let rrf = 1.0 / (RRF_K + rank as f64 + 1.0);
        *scores.entry(row.id).or_default() += rrf;
        snippets.entry(row.id).or_insert(row);
    }

    for (rank, row) in vec_rows.into_iter().enumerate() {
        let rrf = 1.0 / (RRF_K + rank as f64 + 1.0);
        *scores.entry(row.id).or_default() += rrf;
        snippets.entry(row.id).or_insert(row);
    }

    let mut results: Vec<SearchResult> = snippets
        .into_iter()
        .map(|(id, snippet)| SearchResult {
            snippet,
            score: scores[&id],
        })
        .collect();

    results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
    results.truncate(limit);
    results
}
