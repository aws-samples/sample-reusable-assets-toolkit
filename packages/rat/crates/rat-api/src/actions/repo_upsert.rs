use lambda_runtime::Error;
use pgvector::Vector;
use rat_core::api::{RepoUpsertRequest, RepoUpsertResponse};
use rat_core::{embedding, queries, summary};
use tracing::{info, warn};

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

    if let Some(readme) = req.readme.as_deref() {
        if !readme.trim().is_empty() {
            if let Err(e) = generate_and_store_description(state, &req.repo_id, readme).await {
                warn!(error = %e, repo_id = %req.repo_id, "failed to generate repo description");
            }
        }
    }

    Ok(RepoUpsertResponse {
        repo_id: req.repo_id,
    })
}

async fn generate_and_store_description(
    state: &AppState,
    repo_id: &str,
    readme: &str,
) -> Result<(), Error> {
    info!(repo_id, readme_len = readme.len(), "Generating repo description");

    let description = summary::generate_repo_description(
        &state.bedrock,
        &state.summary_model_id,
        repo_id,
        readme,
    )
    .await?;

    if description.is_empty() {
        warn!(repo_id, "empty description generated; skipping");
        return Ok(());
    }

    let emb = embedding::generate_embedding(&state.bedrock, &description, "GENERIC_INDEX").await?;

    queries::update_repo_description(&state.pool, repo_id, &description, Vector::from(emb))
        .await?;
    info!(repo_id, description_len = description.len(), "Repo description stored");
    Ok(())
}
