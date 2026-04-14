use anyhow::{Context, Result};
use aws_config::SdkConfig;

use crate::aws;
use crate::config::{self, Profile};

/// 공통 CLI 세션 초기화 결과.
///
/// 1. `~/.config/rat/config.toml`에서 프로필 로드
/// 2. OAuth 토큰 로드 (만료 시 refresh)
/// 3. Cognito credentials로 AWS SdkConfig 구성
/// 4. SSM에서 프로필의 런타임 값(sqs_queue_url / api_function_arn / migration_function_arn)
///    이 비어있으면 자동 resolve 후 저장
pub struct CliSession {
    pub profile: Profile,
    pub aws_config: SdkConfig,
}

impl CliSession {
    pub async fn init(profile_name: Option<&str>) -> Result<Self> {
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

        Ok(Self { profile, aws_config })
    }
}
