use serde::Deserialize;

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
    pub fn connection_string(&self) -> String {
        format!(
            "postgres://{}:{}@{}:{}/{}",
            self.username, self.password, self.host, self.port, self.dbname
        )
    }
}
