// SPDX-License-Identifier: MIT

use anyhow::Result;
use sqlx::PgPool;
use tracing::info;

use crate::rds_secret::RdsSecret;

/// PgPool 생성.
pub async fn create_pool(connection_string: &str) -> Result<PgPool> {
    let pool = PgPool::connect(connection_string).await?;
    Ok(pool)
}

/// Secrets Manager에서 DB 시크릿을 가져와 지정된 호스트로 PgPool 생성.
pub async fn create_pool_from_secret(db_secret_arn: &str, host: &str) -> Result<PgPool> {
    let aws_config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
    let secrets_client = aws_sdk_secretsmanager::Client::new(&aws_config);
    let rds_secret = RdsSecret::from_secret_arn(&secrets_client, db_secret_arn).await?;

    let conn_str = rds_secret.connection_string_via(host);
    info!(host = %host, db = %rds_secret.dbname, "Connecting to DB");

    create_pool(&conn_str).await
}

/// 임베디드 migration 실행.
/// migration 파일은 workspace root의 `migrations/` 디렉토리에서 컴파일 타임에 포함됨.
pub async fn run_migrations(pool: &PgPool) -> Result<()> {
    info!("Running database migrations");
    sqlx::migrate!("../../migrations").run(pool).await?;
    info!("Migrations completed successfully");
    Ok(())
}
