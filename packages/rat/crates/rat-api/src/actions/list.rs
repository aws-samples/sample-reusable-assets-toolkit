use lambda_runtime::Error;
use rat_core::api::{ListRequest, ListResponse};
use rat_core::queries;
use tracing::info;

use crate::AppState;

pub async fn handle_list(state: &AppState, _req: ListRequest) -> Result<ListResponse, Error> {
    info!("List repos request");

    let repos = queries::list_repos(&state.pool).await?;

    Ok(ListResponse { repos })
}
