use lambda_runtime::Error;
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::AppState;

#[derive(Deserialize)]
pub struct ListRequest {}

#[derive(Serialize)]
pub struct ListResponse {
    repos: Vec<RepoRow>,
}

#[derive(Serialize, sqlx::FromRow)]
struct RepoRow {
    repo_id: String,
    file_count: i64,
    snippet_count: i64,
}

pub async fn handle_list(state: &AppState, _req: ListRequest) -> Result<ListResponse, Error> {
    info!("List repos request");

    let repos = sqlx::query_as::<_, RepoRow>(
        r#"
        SELECT f.repo_id,
               COUNT(DISTINCT f.id) AS file_count,
               COUNT(s.id) AS snippet_count
        FROM files f
        LEFT JOIN snippets s ON s.file_id = f.id
        GROUP BY f.repo_id
        ORDER BY f.repo_id
        "#,
    )
    .fetch_all(&state.pool)
    .await?;

    Ok(ListResponse { repos })
}
