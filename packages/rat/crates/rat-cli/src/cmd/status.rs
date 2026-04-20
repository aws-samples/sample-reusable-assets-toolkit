use anyhow::{Context, Result};
use aws_sdk_sqs::types::QueueAttributeName;

use crate::aws;
use crate::config;

pub async fn handle(profile_name: Option<&str>) -> Result<()> {
    let cfg = config::load_config()?
        .context("No configuration found. Run `rat configure` first.")?;
    let mut profile = config::resolve_profile(&cfg, profile_name)
        .context("Profile not found")?;
    let token = config::load_valid_token(&profile, profile_name)
        .await?
        .context("Not logged in. Run `rat login` first.")?;

    let aws_config = aws::load_aws_config(&profile, &token).await?;
    let ssm = aws_sdk_ssm::Client::new(&aws_config);
    aws::resolve_ssm_values(profile_name, &mut profile, &ssm).await?;

    anyhow::ensure!(
        !profile.sqs_queue_url.is_empty(),
        "sqs_queue_url not configured"
    );

    let sqs = aws_sdk_sqs::Client::new(&aws_config);
    let attrs = sqs
        .get_queue_attributes()
        .queue_url(&profile.sqs_queue_url)
        .attribute_names(QueueAttributeName::ApproximateNumberOfMessages)
        .attribute_names(QueueAttributeName::ApproximateNumberOfMessagesNotVisible)
        .attribute_names(QueueAttributeName::ApproximateNumberOfMessagesDelayed)
        .send()
        .await
        .context("failed to get SQS queue attributes")?;

    let get = |name: QueueAttributeName| -> &str {
        attrs
            .attributes()
            .and_then(|m| m.get(&name))
            .map(String::as_str)
            .unwrap_or("-")
    };

    println!("Queue: {}", profile.sqs_queue_url);
    println!(
        "  Visible   : {}",
        get(QueueAttributeName::ApproximateNumberOfMessages)
    );
    println!(
        "  In-flight : {}",
        get(QueueAttributeName::ApproximateNumberOfMessagesNotVisible)
    );
    println!(
        "  Delayed   : {}",
        get(QueueAttributeName::ApproximateNumberOfMessagesDelayed)
    );

    Ok(())
}
