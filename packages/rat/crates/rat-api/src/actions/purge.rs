use lambda_runtime::Error;
use rat_core::queries;
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
    found: bool,
    deleted_files: i64,
    deleted_snippets: i64,
}

pub async fn handle_purge(state: &AppState, req: PurgeRequest) -> Result<PurgeResponse, Error> {
    warn!(repo_id = %req.repo_id, "Purge repo request");

    let mut tx = state.pool.begin().await?;

    let counts = queries::count_repo_contents(&mut *tx, &req.repo_id).await?;
    let (deleted_files, deleted_snippets, found) = match counts {
        Some(c) => (c.deleted_files, c.deleted_snippets, true),
        None => (0, 0, false),
    };

    if found {
        queries::delete_repo(&mut *tx, &req.repo_id).await?;
    }

    tx.commit().await?;

    warn!(repo_id = %req.repo_id, deleted_files, deleted_snippets, found, "Purge complete");

    Ok(PurgeResponse {
        repo_id: req.repo_id,
        found,
        deleted_files,
        deleted_snippets,
    })
}
