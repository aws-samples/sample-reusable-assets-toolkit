use lambda_runtime::Error;
use rat_core::api::{SnippetListRequest, SnippetListResponse};
use rat_core::queries;
use tracing::info;

use crate::AppState;

pub async fn handle_snippet_list(
    state: &AppState,
    req: SnippetListRequest,
) -> Result<SnippetListResponse, Error> {
    info!(
        repo_id = %req.repo_id,
        source_path = %req.source_path,
        "Snippet list request"
    );

    let snippets = queries::list_snippets_by_file(
        &state.pool,
        &req.repo_id,
        &req.source_path,
    )
    .await?;

    Ok(SnippetListResponse { snippets })
}
