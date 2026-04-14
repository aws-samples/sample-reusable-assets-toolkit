use anyhow::{bail, Context, Result};
use aws_sdk_lambda::primitives::Blob;
use dialoguer::theme::ColorfulTheme;
use dialoguer::Select;

use rat_cli::aws;
use rat_cli::config;

pub async fn handle(reset: bool, profile_name: Option<&str>) -> Result<()> {
    let cfg = config::load_config()?.context("No configuration found. Run `rat configure` first.")?;
    let mut profile = config::resolve_profile(&cfg, profile_name)
        .context("Profile not found")?;
    let token = config::load_valid_token(&profile, profile_name).await?
        .context("Not logged in. Run `rat login` first.")?;

    let aws_config = aws::load_aws_config(&profile, &token).await?;
    let ssm = aws_sdk_ssm::Client::new(&aws_config);
    aws::resolve_ssm_values(profile_name, &mut profile, &ssm).await?;

    anyhow::ensure!(
        !profile.migration_function_arn.is_empty(),
        "migration_function_arn not configured"
    );
    let function_arn = &profile.migration_function_arn;

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
    let lambda_client = aws_sdk_lambda::Client::new(&aws_config);
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
