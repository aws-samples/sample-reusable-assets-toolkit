use lambda_runtime::Error;
use serde::{Deserialize, Serialize};
use tracing::warn;

use crate::AppState;

#[derive(Deserialize)]
pub struct PurgeRequest {
    repo_id: String,
}

#[derive(Serialize)]
pub struct PurgeResponse {
    repo_id: String,
    deleted_files: u64,
}

pub async fn handle_purge(state: &AppState, req: PurgeRequest) -> Result<PurgeResponse, Error> {
    warn!(repo_id = %req.repo_id, "Purge repo request");

    let result = sqlx::query("DELETE FROM files WHERE repo_id = $1")
        .bind(&req.repo_id)
        .execute(&state.pool)
        .await?;

    let deleted_files = result.rows_affected();
    warn!(repo_id = %req.repo_id, deleted_files, "Purge complete");

    Ok(PurgeResponse {
        repo_id: req.repo_id,
        deleted_files,
    })
}
