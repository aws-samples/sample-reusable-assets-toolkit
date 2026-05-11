// SPDX-License-Identifier: MIT

use anyhow::{Context, Result};
use aws_sdk_sqs::Client as SqsClient;

use rat_core::message::FileMessage;

pub async fn send_file_message(
    sqs: &SqsClient,
    queue_url: &str,
    msg: &FileMessage,
) -> Result<()> {
    let body = serde_json::to_string(msg)?;
    sqs.send_message()
        .queue_url(queue_url)
        .message_body(body)
        .send()
        .await
        .context("failed to send SQS message")?;
    Ok(())
}
