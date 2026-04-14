use std::collections::HashMap;

use lambda_runtime::Error;
use pgvector::Vector;
use rat_core::embedding;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use tracing::info;

use crate::AppState;

#[derive(Deserialize)]
pub struct SearchRequest {
    query: String,
    #[serde(default)]
    repo_id: Option<String>,
    #[serde(default)]
    source_type: Option<String>,
    #[serde(default)]
    tags: Option<Vec<String>>,
    #[serde(default = "default_limit")]
    limit: i64,
}

fn default_limit() -> i64 {
    3
}

#[derive(Serialize)]
pub struct SearchResponse {
    results: Vec<SearchResult>,
}

#[derive(Serialize, sqlx::FromRow)]
struct SnippetRow {
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
}

#[derive(Serialize)]
struct SearchResult {
    #[serde(flatten)]
    snippet: SnippetRow,
    score: f64,
}

const RRF_K: f64 = 60.0;
const SEARCH_POOL_SIZE: i64 = 50;

pub async fn handle_search(
    state: &AppState,
    req: SearchRequest,
) -> Result<SearchResponse, Error> {
    info!(query = %req.query, repo_id = ?req.repo_id, "Search request");

    let query_embedding = embedding::generate_embedding(&state.bedrock, &req.query, "GENERIC_RETRIEVAL").await?;

    let (fts_rows, vec_rows) = tokio::join!(
        full_text_search(&state.pool, &req),
        vector_search(&state.pool, &req, &query_embedding),
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

async fn full_text_search(
    pool: &PgPool,
    req: &SearchRequest,
) -> Result<Vec<SnippetRow>, Error> {
    let rows = sqlx::query_as::<_, SnippetRow>(
        r#"
        SELECT s.id, s.repo_id, f.source_path, s.content, s.description,
               s.source_type, s.symbol_name, s.start_line, s.end_line, f.language
        FROM snippets s
        JOIN files f ON f.id = s.file_id
        WHERE s.search_vector @@ websearch_to_tsquery('english', $1)
          AND ($2::text IS NULL OR s.repo_id = $2)
          AND ($3::text IS NULL OR s.source_type = $3)
          AND ($4::text[] IS NULL OR s.tags && $4)
        ORDER BY ts_rank(s.search_vector, websearch_to_tsquery('english', $1)) DESC
        LIMIT $5
        "#,
    )
    .bind(&req.query)
    .bind(&req.repo_id)
    .bind(&req.source_type)
    .bind(&req.tags)
    .bind(SEARCH_POOL_SIZE)
    .fetch_all(pool)
    .await?;

    Ok(rows)
}

async fn vector_search(
    pool: &PgPool,
    req: &SearchRequest,
    query_embedding: &[f32],
) -> Result<Vec<SnippetRow>, Error> {
    let embedding = Vector::from(query_embedding.to_vec());

    let rows = sqlx::query_as::<_, SnippetRow>(
        r#"
        SELECT s.id, s.repo_id, f.source_path, s.content, s.description,
               s.source_type, s.symbol_name, s.start_line, s.end_line, f.language
        FROM snippets s
        JOIN files f ON f.id = s.file_id
        WHERE ($1::text IS NULL OR s.repo_id = $1)
          AND ($2::text IS NULL OR s.source_type = $2)
          AND ($3::text[] IS NULL OR s.tags && $3)
        ORDER BY s.embedding <=> $4
        LIMIT $5
        "#,
    )
    .bind(&req.repo_id)
    .bind(&req.source_type)
    .bind(&req.tags)
    .bind(embedding)
    .bind(SEARCH_POOL_SIZE)
    .fetch_all(pool)
    .await?;

    Ok(rows)
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
