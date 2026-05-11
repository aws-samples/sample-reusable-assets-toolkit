// SPDX-License-Identifier: MIT

use anyhow::{bail, Context, Result};
use aws_sdk_lambda::primitives::Blob;
use dialoguer::theme::ColorfulTheme;
use dialoguer::Select;

use crate::session::CliSession;

pub async fn handle(reset: bool, profile_name: Option<&str>) -> Result<()> {
    let session = CliSession::init(profile_name).await?;
    let function_arn = &session.profile.migration_function_arn;

    if reset {
        let theme = ColorfulTheme::default();
        let selection = Select::with_theme(&theme)
            .with_prompt("This will DROP ALL TABLES and re-run migrations. Continue?")
            .items(&["No", "Yes"])
            .default(0)
            .interact()?;
        if selection != 1 {
            eprintln!("Aborted.");
            return Ok(());
        }
    }

    eprintln!("Invoking migration Lambda (reset={})...", reset);

    let payload = serde_json::json!({ "reset": reset });
    let lambda_client = aws_sdk_lambda::Client::new(&session.aws_config);
    let response = lambda_client
        .invoke()
        .function_name(function_arn)
        .payload(Blob::new(serde_json::to_vec(&payload)?))
        .send()
        .await
        .context("failed to invoke migration Lambda")?;

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
    let text = String::from_utf8_lossy(payload.as_ref());
    println!("Migration complete: {}", text);

    Ok(())
}
