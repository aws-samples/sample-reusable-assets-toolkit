// SPDX-License-Identifier: MIT

use pgvector::Vector;
use serde::{Deserialize, Serialize};
use sqlx::PgExecutor;

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct RepoRow {
    pub repo_id: String,
    pub branch: String,
    pub indexed_commit_id: Option<String>,
    pub description: Option<String>,
    pub file_count: i64,
    pub snippet_count: i64,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct FileRow {
    pub id: i64,
    pub repo_id: String,
    pub source_path: String,
    pub content: String,
    pub language: Option<String>,
}

/// Lightweight row for listing files of a repo (no content).
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct FileListRow {
    pub id: i64,
    pub source_path: String,
    pub language: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct SnippetRow {
    pub id: i64,
    pub repo_id: String,
    pub source_path: String,
    pub content: String,
    pub description: String,
    pub source_type: String,
    pub symbol_name: Option<String>,
    pub start_line: Option<i32>,
    pub end_line: Option<i32>,
    pub language: Option<String>,
}

pub struct PurgeCounts {
    pub deleted_files: i64,
    pub deleted_snippets: i64,
}

pub async fn upsert_repo<'e, E: PgExecutor<'e>>(
    executor: E,
    repo_id: &str,
    branch: &str,
    commit_id: Option<&str>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO repos (repo_id, branch, indexed_commit_id)
         VALUES ($1, $2, $3)
         ON CONFLICT (repo_id) DO UPDATE
         SET branch = EXCLUDED.branch,
             indexed_commit_id = COALESCE(EXCLUDED.indexed_commit_id, repos.indexed_commit_id),
             updated_at = NOW()",
    )
    .bind(repo_id)
    .bind(branch)
    .bind(commit_id)
    .execute(executor)
    .await?;
    Ok(())
}

pub async fn update_repo_description<'e, E: PgExecutor<'e>>(
    executor: E,
    repo_id: &str,
    description: &str,
    embedding: Vector,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE repos
         SET description = $2,
             embedding = $3::vector,
             updated_at = NOW()
         WHERE repo_id = $1",
    )
    .bind(repo_id)
    .bind(description)
    .bind(embedding)
    .execute(executor)
    .await?;
    Ok(())
}

pub async fn upsert_file<'e, E: PgExecutor<'e>>(
    executor: E,
    repo_id: &str,
    source_path: &str,
    content: &str,
    language: Option<&str>,
) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar(
        "INSERT INTO files (repo_id, source_path, content, language)
         VALUES ($1, $2, $3, $4)
         ON CONFLICT (repo_id, source_path) DO UPDATE
         SET content = EXCLUDED.content,
             language = EXCLUDED.language
         RETURNING id",
    )
    .bind(repo_id)
    .bind(source_path)
    .bind(content)
    .bind(language)
    .fetch_one(executor)
    .await
}

pub async fn delete_snippets_by_file<'e, E: PgExecutor<'e>>(
    executor: E,
    file_id: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM snippets WHERE file_id = $1")
        .bind(file_id)
        .execute(executor)
        .await?;
    Ok(())
}

pub async fn insert_snippet<'e, E: PgExecutor<'e>>(
    executor: E,
    file_id: i64,
    repo_id: &str,
    content: &str,
    description: &str,
    embedding: Vector,
    source_type: &str,
    start_line: i32,
    end_line: i32,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO snippets (file_id, repo_id, content, description, embedding, source_type, start_line, end_line)
         VALUES ($1, $2, $3, $4, $5::vector, $6, $7, $8)",
    )
    .bind(file_id)
    .bind(repo_id)
    .bind(content)
    .bind(description)
    .bind(embedding)
    .bind(source_type)
    .bind(start_line)
    .bind(end_line)
    .execute(executor)
    .await?;
    Ok(())
}

pub async fn delete_file<'e, E: PgExecutor<'e>>(
    executor: E,
    repo_id: &str,
    source_path: &str,
) -> Result<u64, sqlx::Error> {
    let result = sqlx::query("DELETE FROM files WHERE repo_id = $1 AND source_path = $2")
        .bind(repo_id)
        .bind(source_path)
        .execute(executor)
        .await?;
    Ok(result.rows_affected())
}

pub async fn delete_repo<'e, E: PgExecutor<'e>>(
    executor: E,
    repo_id: &str,
) -> Result<u64, sqlx::Error> {
    let result = sqlx::query("DELETE FROM repos WHERE repo_id = $1")
        .bind(repo_id)
        .execute(executor)
        .await?;
    Ok(result.rows_affected())
}

pub async fn count_repo_contents<'e, E: PgExecutor<'e>>(
    executor: E,
    repo_id: &str,
) -> Result<Option<PurgeCounts>, sqlx::Error> {
    let row: Option<(i64, i64)> = sqlx::query_as(
        r#"
        SELECT
            (SELECT COUNT(*) FROM files WHERE repo_id = $1),
            (SELECT COUNT(*) FROM snippets WHERE repo_id = $1)
        WHERE EXISTS (SELECT 1 FROM repos WHERE repo_id = $1)
        "#,
    )
    .bind(repo_id)
    .fetch_optional(executor)
    .await?;

    Ok(row.map(|(deleted_files, deleted_snippets)| PurgeCounts {
        deleted_files,
        deleted_snippets,
    }))
}

pub async fn list_snippets_by_file<'e, E: PgExecutor<'e>>(
    executor: E,
    repo_id: &str,
    source_path: &str,
) -> Result<Vec<SnippetRow>, sqlx::Error> {
    sqlx::query_as::<_, SnippetRow>(
        r#"
        SELECT s.id, s.repo_id, f.source_path, s.content, s.description,
               s.source_type, s.symbol_name, s.start_line, s.end_line, f.language
        FROM snippets s
        JOIN files f ON f.id = s.file_id
        WHERE s.repo_id = $1 AND f.source_path = $2
        ORDER BY s.start_line ASC NULLS LAST
        "#,
    )
    .bind(repo_id)
    .bind(source_path)
    .fetch_all(executor)
    .await
}

pub async fn list_files_by_repo<'e, E: PgExecutor<'e>>(
    executor: E,
    repo_id: &str,
) -> Result<Vec<FileListRow>, sqlx::Error> {
    sqlx::query_as::<_, FileListRow>(
        r#"
        SELECT id, source_path, language
        FROM files
        WHERE repo_id = $1
        ORDER BY source_path
        "#,
    )
    .bind(repo_id)
    .fetch_all(executor)
    .await
}

pub async fn get_file<'e, E: PgExecutor<'e>>(
    executor: E,
    repo_id: &str,
    source_path: &str,
) -> Result<Option<FileRow>, sqlx::Error> {
    sqlx::query_as::<_, FileRow>(
        r#"
        SELECT id, repo_id, source_path, content, language
        FROM files
        WHERE repo_id = $1 AND source_path = $2
        "#,
    )
    .bind(repo_id)
    .bind(source_path)
    .fetch_optional(executor)
    .await
}

pub async fn get_repo<'e, E: PgExecutor<'e>>(
    executor: E,
    repo_id: &str,
) -> Result<Option<RepoRow>, sqlx::Error> {
    sqlx::query_as::<_, RepoRow>(
        r#"
        SELECT r.repo_id,
               r.branch,
               r.indexed_commit_id,
               r.description,
               COUNT(DISTINCT f.id) AS file_count,
               COUNT(s.id) AS snippet_count
        FROM repos r
        LEFT JOIN files f ON f.repo_id = r.repo_id
        LEFT JOIN snippets s ON s.file_id = f.id
        WHERE r.repo_id = $1
        GROUP BY r.repo_id, r.branch, r.indexed_commit_id, r.description
        "#,
    )
    .bind(repo_id)
    .fetch_optional(executor)
    .await
}

pub async fn list_repos<'e, E: PgExecutor<'e>>(
    executor: E,
) -> Result<Vec<RepoRow>, sqlx::Error> {
    sqlx::query_as::<_, RepoRow>(
        r#"
        SELECT r.repo_id,
               r.branch,
               r.indexed_commit_id,
               r.description,
               COUNT(DISTINCT f.id) AS file_count,
               COUNT(s.id) AS snippet_count
        FROM repos r
        LEFT JOIN files f ON f.repo_id = r.repo_id
        LEFT JOIN snippets s ON s.file_id = f.id
        GROUP BY r.repo_id, r.branch, r.indexed_commit_id, r.description
        ORDER BY r.repo_id
        "#,
    )
    .fetch_all(executor)
    .await
}

pub async fn full_text_search_repos<'e, E: PgExecutor<'e>>(
    executor: E,
    query: &str,
    limit: i64,
) -> Result<Vec<RepoRow>, sqlx::Error> {
    sqlx::query_as::<_, RepoRow>(
        r#"
        SELECT r.repo_id,
               r.branch,
               r.indexed_commit_id,
               r.description,
               COUNT(DISTINCT f.id) AS file_count,
               COUNT(s.id) AS snippet_count
        FROM repos r
        LEFT JOIN files f ON f.repo_id = r.repo_id
        LEFT JOIN snippets s ON s.file_id = f.id
        WHERE r.search_vector @@ websearch_to_tsquery('english', $1)
        GROUP BY r.repo_id
        ORDER BY ts_rank(r.search_vector, websearch_to_tsquery('english', $1)) DESC
        LIMIT $2
        "#,
    )
    .bind(query)
    .bind(limit)
    .fetch_all(executor)
    .await
}

pub async fn vector_search_repos<'e, E: PgExecutor<'e>>(
    executor: E,
    query_embedding: &[f32],
    limit: i64,
) -> Result<Vec<RepoRow>, sqlx::Error> {
    let embedding = Vector::from(query_embedding.to_vec());
    sqlx::query_as::<_, RepoRow>(
        r#"
        SELECT r.repo_id,
               r.branch,
               r.indexed_commit_id,
               r.description,
               COUNT(DISTINCT f.id) AS file_count,
               COUNT(s.id) AS snippet_count
        FROM repos r
        LEFT JOIN files f ON f.repo_id = r.repo_id
        LEFT JOIN snippets s ON s.file_id = f.id
        WHERE r.embedding IS NOT NULL
        GROUP BY r.repo_id
        ORDER BY r.embedding <=> $1
        LIMIT $2
        "#,
    )
    .bind(embedding)
    .bind(limit)
    .fetch_all(executor)
    .await
}

pub async fn full_text_search<'e, E: PgExecutor<'e>>(
    executor: E,
    query: &str,
    repo_id: Option<&str>,
    source_type: Option<&str>,
    limit: i64,
) -> Result<Vec<SnippetRow>, sqlx::Error> {
    sqlx::query_as::<_, SnippetRow>(
        r#"
        SELECT s.id, s.repo_id, f.source_path, s.content, s.description,
               s.source_type, s.symbol_name, s.start_line, s.end_line, f.language
        FROM snippets s
        JOIN files f ON f.id = s.file_id
        WHERE s.search_vector @@ websearch_to_tsquery('english', $1)
          AND ($2::text IS NULL OR s.repo_id = $2)
          AND ($3::text IS NULL OR s.source_type = $3)
        ORDER BY ts_rank(s.search_vector, websearch_to_tsquery('english', $1)) DESC
        LIMIT $4
        "#,
    )
    .bind(query)
    .bind(repo_id)
    .bind(source_type)
    .bind(limit)
    .fetch_all(executor)
    .await
}

pub async fn vector_search<'e, E: PgExecutor<'e>>(
    executor: E,
    query_embedding: &[f32],
    repo_id: Option<&str>,
    source_type: Option<&str>,
    limit: i64,
) -> Result<Vec<SnippetRow>, sqlx::Error> {
    let embedding = Vector::from(query_embedding.to_vec());

    sqlx::query_as::<_, SnippetRow>(
        r#"
        SELECT s.id, s.repo_id, f.source_path, s.content, s.description,
               s.source_type, s.symbol_name, s.start_line, s.end_line, f.language
        FROM snippets s
        JOIN files f ON f.id = s.file_id
        WHERE ($1::text IS NULL OR s.repo_id = $1)
          AND ($2::text IS NULL OR s.source_type = $2)
        ORDER BY s.embedding <=> $3
        LIMIT $4
        "#,
    )
    .bind(repo_id)
    .bind(source_type)
    .bind(embedding)
    .bind(limit)
    .fetch_all(executor)
    .await
}
