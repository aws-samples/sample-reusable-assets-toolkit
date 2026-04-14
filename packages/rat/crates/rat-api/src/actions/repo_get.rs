use lambda_runtime::Error;
use rat_core::api::{RepoGetRequest, RepoGetResponse};
use rat_core::queries;
use tracing::info;

use crate::AppState;

pub async fn handle_repo_get(
    state: &AppState,
    req: RepoGetRequest,
) -> Result<RepoGetResponse, Error> {
    info!(repo_id = %req.repo_id, "Repo get request");

    let repo = queries::get_repo(&state.pool, &req.repo_id).await?;

    Ok(RepoGetResponse { repo })
}
