use lambda_runtime::Error;
use rat_core::api::{FileListRequest, FileListResponse};
use rat_core::queries;
use tracing::info;

use crate::AppState;

pub async fn handle_file_list(
    state: &AppState,
    req: FileListRequest,
) -> Result<FileListResponse, Error> {
    info!(repo_id = %req.repo_id, "File list request");

    let files = queries::list_files_by_repo(&state.pool, &req.repo_id).await?;

    Ok(FileListResponse { files })
}
