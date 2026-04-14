use lambda_runtime::Error;
use rat_core::api::{RepoUpsertRequest, RepoUpsertResponse};
use rat_core::queries;
use tracing::info;

use crate::AppState;

pub async fn handle_repo_upsert(
    state: &AppState,
    req: RepoUpsertRequest,
) -> Result<RepoUpsertResponse, Error> {
    info!(repo_id = %req.repo_id, branch = %req.branch, "Repo upsert request");

    queries::upsert_repo(
        &state.pool,
        &req.repo_id,
        &req.branch,
        req.commit_id.as_deref(),
    )
    .await?;

    Ok(RepoUpsertResponse {
        repo_id: req.repo_id,
    })
}
