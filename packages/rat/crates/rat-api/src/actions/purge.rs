use lambda_runtime::Error;
use rat_core::api::{PurgeRequest, PurgeResponse};
use rat_core::queries;
use tracing::warn;

use crate::AppState;

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
