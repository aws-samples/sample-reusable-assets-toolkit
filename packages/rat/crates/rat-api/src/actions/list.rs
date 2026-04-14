use lambda_runtime::Error;
use rat_core::queries::{self, RepoRow};
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::AppState;

#[derive(Deserialize)]
pub struct ListRequest {}

#[derive(Serialize)]
pub struct ListResponse {
    repos: Vec<RepoRow>,
}

pub async fn handle_list(state: &AppState, _req: ListRequest) -> Result<ListResponse, Error> {
    info!("List repos request");

    let repos = queries::list_repos(&state.pool).await?;

    Ok(ListResponse { repos })
}
