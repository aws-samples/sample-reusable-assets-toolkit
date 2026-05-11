// SPDX-License-Identifier: MIT

use anyhow::{bail, Context, Result};
use aws_sdk_lambda::primitives::Blob;
use aws_sdk_lambda::Client as LambdaClient;

use rat_core::api::{ApiRequest, RepoGetRequest, RepoGetResponse, RepoUpsertRequest};
use rat_core::queries::RepoRow;

pub async fn invoke_api(
    lambda: &LambdaClient,
    function_arn: &str,
    request: &ApiRequest,
) -> Result<Vec<u8>> {
    let payload = serde_json::to_vec(request)?;
    let response = lambda
        .invoke()
        .function_name(function_arn)
        .payload(Blob::new(payload))
        .send()
        .await
        .context("failed to invoke API Lambda")?;

    if let Some(err) = response.function_error() {
        let body = response
            .payload()
            .map(|p| String::from_utf8_lossy(p.as_ref()).to_string())
            .unwrap_or_default();
        bail!("Lambda error ({}): {}", err, body);
    }

    let payload = response
        .payload()
        .context("no response payload from Lambda")?;
    Ok(payload.as_ref().to_vec())
}

pub async fn fetch_repo(
    lambda: &LambdaClient,
    function_arn: &str,
    repo_id: &str,
) -> Result<Option<RepoRow>> {
    let bytes = invoke_api(
        lambda,
        function_arn,
        &ApiRequest::RepoGet(RepoGetRequest {
            repo_id: repo_id.to_string(),
        }),
    )
    .await?;
    let parsed: RepoGetResponse =
        serde_json::from_slice(&bytes).context("failed to parse repo_get response")?;
    Ok(parsed.repo)
}

pub async fn upsert_repo(
    lambda: &LambdaClient,
    function_arn: &str,
    repo_id: &str,
    branch: &str,
    commit_id: Option<&str>,
    readme: Option<&str>,
) -> Result<()> {
    invoke_api(
        lambda,
        function_arn,
        &ApiRequest::RepoUpsert(RepoUpsertRequest {
            repo_id: repo_id.to_string(),
            branch: branch.to_string(),
            commit_id: commit_id.map(|s| s.to_string()),
            readme: readme.map(|s| s.to_string()),
        }),
    )
    .await?;
    Ok(())
}
