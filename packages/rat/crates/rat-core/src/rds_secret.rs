// SPDX-License-Identifier: MIT

use anyhow::Result;
use aws_sdk_secretsmanager::Client as SecretsClient;
use serde::Deserialize;
use tracing::info;

/// RDS Secrets Manager에 저장되는 DB 시크릿 JSON 구조.
/// Aurora가 자동 생성하는 포맷과 동일.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RdsSecret {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub dbname: String,
    pub engine: String,
    pub db_cluster_identifier: String,
}

impl RdsSecret {
    /// Secrets Manager에서 시크릿을 가져와 파싱.
    pub async fn from_secret_arn(client: &SecretsClient, secret_arn: &str) -> Result<Self> {
        info!(secret_arn = %secret_arn, "Fetching DB secret");
        let secret_value = client
            .get_secret_value()
            .secret_id(secret_arn)
            .send()
            .await?;
        let secret_str = secret_value.secret_string().unwrap_or_default();
        let secret: Self = serde_json::from_str(secret_str)?;
        Ok(secret)
    }

    pub fn connection_string(&self) -> String {
        format!(
            "postgres://{}:{}@{}:{}/{}",
            self.username, self.password, self.host, self.port, self.dbname
        )
    }

    /// 지정된 호스트(예: RDS Proxy 엔드포인트)를 사용하는 connection string 생성.
    pub fn connection_string_via(&self, host: &str) -> String {
        format!(
            "postgres://{}:{}@{}:{}/{}",
            self.username, self.password, host, self.port, self.dbname
        )
    }
}
