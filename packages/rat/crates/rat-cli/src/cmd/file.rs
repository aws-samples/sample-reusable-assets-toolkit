// SPDX-License-Identifier: MIT

use anyhow::Result;

use crate::session::CliSession;
use rat_client::ops;
use rat_core::queries::FileRow;

pub async fn run_file_get(
    repo_id: &str,
    source_path: &str,
    profile_name: Option<&str>,
) -> Result<Option<FileRow>> {
    let session = CliSession::init(profile_name).await?;
    let lambda = aws_sdk_lambda::Client::new(&session.aws_config);
    ops::file_get(
        &lambda,
        &session.profile.api_function_arn,
        repo_id,
        source_path,
    )
    .await
}
