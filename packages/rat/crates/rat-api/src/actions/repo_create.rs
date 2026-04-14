use lambda_runtime::Error;
use rat_core::queries;
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::AppState;

#[derive(Deserialize)]
pub struct RepoCreateRequest {
    repo_id: String,
    branch: String,
    commit_id: String,
}

#[derive(Serialize)]
pub struct RepoCreateResponse {
    repo_id: String,
}

pub async fn handle_repo_create(
    state: &AppState,
    req: RepoCreateRequest,
) -> Result<RepoCreateResponse, Error> {
    info!(repo_id = %req.repo_id, branch = %req.branch, "Repo create request");

    queries::upsert_repo(&state.pool, &req.repo_id, &req.branch, &req.commit_id).await?;

    Ok(RepoCreateResponse {
        repo_id: req.repo_id,
    })
}
