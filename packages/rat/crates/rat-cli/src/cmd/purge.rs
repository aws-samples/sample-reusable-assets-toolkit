// SPDX-License-Identifier: MIT

use anyhow::{Context, Result};

use rat_client::api_client;
use crate::session::CliSession;
use rat_core::api::{ApiRequest, PurgeRequest, PurgeResponse};

pub async fn handle(repo_id: &str, profile_name: Option<&str>) -> Result<()> {
    let session = CliSession::init(profile_name).await?;
    let lambda = aws_sdk_lambda::Client::new(&session.aws_config);

    let bytes = api_client::invoke_api(
        &lambda,
        &session.profile.api_function_arn,
        &ApiRequest::Purge(PurgeRequest {
            repo_id: repo_id.to_string(),
        }),
    )
    .await?;
    let response: PurgeResponse =
        serde_json::from_slice(&bytes).context("failed to parse purge response")?;

    if !response.found {
        println!("Repo '{}' not found.", response.repo_id);
    } else {
        println!(
            "Purged repo '{}': {} file(s), {} snippet(s) deleted.",
            response.repo_id, response.deleted_files, response.deleted_snippets
        );
    }

    Ok(())
}
