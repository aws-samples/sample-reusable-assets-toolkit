use lambda_runtime::Error;
use rat_core::api::{FileGetRequest, FileGetResponse};
use rat_core::queries;
use tracing::info;

use crate::AppState;

pub async fn handle_file_get(
    state: &AppState,
    req: FileGetRequest,
) -> Result<FileGetResponse, Error> {
    info!(
        repo_id = %req.repo_id,
        source_path = %req.source_path,
        "File get request"
    );

    let file = queries::get_file(&state.pool, &req.repo_id, &req.source_path).await?;

    Ok(FileGetResponse { file })
}
