use anyhow::{Context, Result};
use aws_config::Region;
use aws_sdk_cognitoidentity::config::Credentials;
use aws_sdk_ssm::Client as SsmClient;

use crate::config::{self, Profile, TokenSet};

const SSM_INGEST_QUEUE_URL: &str = "/idp-code/ingest/queue-url";
const SSM_API_FUNCTION_ARN: &str = "/idp-code/api/function-arn";
const SSM_MIGRATION_FUNCTION_ARN: &str = "/idp-code/migration/function-arn";

/// Cognito Identity Pool의 id_token으로 AWS credentials를 얻어 SdkConfig를 반환한다.
pub async fn load_aws_config(profile: &Profile, token: &TokenSet) -> Result<aws_config::SdkConfig> {
    let region = Region::new(profile.aws_region.clone());

    let cognito_client = aws_sdk_cognitoidentity::Client::new(
        &aws_config::defaults(aws_config::BehaviorVersion::latest())
            .region(region.clone())
            .no_credentials()
            .load()
            .await,
    );

    let provider_name = format!(
        "cognito-idp.{}.amazonaws.com/{}",
        profile.aws_region, profile.cognito_user_pool_id,
    );

    let get_id_output = cognito_client
        .get_id()
        .identity_pool_id(&profile.cognito_identity_pool_id)
        .logins(&provider_name, &token.id_token)
        .send()
        .await
        .context("failed to get Cognito identity ID")?;

    let identity_id = get_id_output
        .identity_id()
        .context("no identity ID returned")?;

    let creds_output = cognito_client
        .get_credentials_for_identity()
        .identity_id(identity_id)
        .logins(&provider_name, &token.id_token)
        .send()
        .await
        .context("failed to get AWS credentials from Cognito")?;

    let creds = creds_output
        .credentials()
        .context("no credentials returned")?;

    let access_key = creds.access_key_id().context("no access key ID")?;
    let secret_key = creds.secret_key().context("no secret key")?;
    let session_token = creds.session_token().context("no session token")?;

    let sdk_config = aws_config::defaults(aws_config::BehaviorVersion::latest())
        .region(region)
        .credentials_provider(Credentials::new(
            access_key,
            secret_key,
            Some(session_token.to_string()),
            None,
            "cognito-identity",
        ))
        .load()
        .await;

    Ok(sdk_config)
}

/// Profile의 SSM 기반 값(sqs_queue_url, api_function_arn)이 없으면 SSM에서 가져와 config에 저장한다.
pub async fn resolve_ssm_values(
    profile_name: Option<&str>,
    profile: &mut Profile,
    ssm: &SsmClient,
) -> Result<()> {
    let mut updated = false;

    if profile.sqs_queue_url.is_empty() {
        profile.sqs_queue_url = get_ssm_parameter(ssm, SSM_INGEST_QUEUE_URL).await?;
        updated = true;
    }

    if profile.api_function_arn.is_empty() {
        profile.api_function_arn = get_ssm_parameter(ssm, SSM_API_FUNCTION_ARN).await?;
        updated = true;
    }

    if profile.migration_function_arn.is_empty() {
        profile.migration_function_arn =
            get_ssm_parameter(ssm, SSM_MIGRATION_FUNCTION_ARN).await?;
        updated = true;
    }

    if updated {
        let mut cfg = config::load_config()?.context("config not found")?;
        match profile_name {
            None | Some("default") => cfg.default = profile.clone(),
            Some(name) => {
                cfg.profiles.insert(name.to_string(), profile.clone());
            }
        }
        config::save_config(&cfg)?;
    }

    Ok(())
}

async fn get_ssm_parameter(ssm: &SsmClient, name: &str) -> Result<String> {
    let output = ssm
        .get_parameter()
        .name(name)
        .send()
        .await
        .with_context(|| format!("failed to get SSM parameter: {}", name))?;

    let value = output
        .parameter()
        .and_then(|p| p.value())
        .with_context(|| format!("SSM parameter {} has no value", name))?;

    Ok(value.to_string())
}
