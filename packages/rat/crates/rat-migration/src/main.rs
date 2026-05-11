// SPDX-License-Identifier: MIT

use anyhow::Result;
use lambda_runtime::{service_fn, Error, LambdaEvent};
use rat_core::db;
use rat_core::rds_secret::RdsSecret;
use serde::Deserialize;
use tracing::{info, warn};

#[derive(Deserialize)]
struct Config {
    db_secret_arn: String,
    rds_proxy_endpoint: String,
}

#[derive(Deserialize, Default)]
struct Payload {
    #[serde(default)]
    reset: bool,
}

async fn handler(event: LambdaEvent<serde_json::Value>) -> Result<serde_json::Value, Error> {
    let payload: Payload = serde_json::from_value(event.payload).unwrap_or_default();
    let config: Config = envy::from_env()?;

    let aws_config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
    let secrets_client = aws_sdk_secretsmanager::Client::new(&aws_config);
    let rds_secret = RdsSecret::from_secret_arn(&secrets_client, &config.db_secret_arn).await?;

    let conn_str = rds_secret.connection_string_via(&config.rds_proxy_endpoint);
    info!(host = %config.rds_proxy_endpoint, db = %rds_secret.dbname, "Connecting via RDS Proxy");

    let pool = db::create_pool(&conn_str).await?;

    if payload.reset {
        warn!("RESET requested — dropping existing schema");
        sqlx::query(
            "DROP TABLE IF EXISTS snippets, files, repos, _sqlx_migrations CASCADE",
        )
        .execute(&pool)
        .await?;
        sqlx::query("DROP EXTENSION IF EXISTS vector")
            .execute(&pool)
            .await?;
        warn!("Schema reset complete");
    }

    db::run_migrations(&pool).await?;

    pool.close().await;
    info!("Migration completed successfully");

    Ok(serde_json::json!({ "status": "ok", "reset": payload.reset }))
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt()
        .json()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .without_time()
        .init();

    lambda_runtime::run(service_fn(handler)).await
}
