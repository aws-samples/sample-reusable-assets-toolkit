use aws_lambda_events::sqs::SqsEvent;
use aws_sdk_secretsmanager::Client as SecretsClient;
use lambda_runtime::{service_fn, Error, LambdaEvent};
use rat_core::db::RdsSecret;
use serde::Deserialize;
use sqlx::PgPool;
use tracing::{error, info};

#[derive(Deserialize)]
struct Config {
    rds_proxy_endpoint: String,
    db_secret_arn: String,
}

struct AppState {
    pool: PgPool,
}

async fn init() -> Result<AppState, Error> {
    tracing_subscriber::fmt()
        .json()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .without_time() // Lambda adds timestamp
        .init();

    info!("Initializing Lambda");

    let config: Config = envy::from_env()?;

    let aws_config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
    let secrets_client = SecretsClient::new(&aws_config);

    // Fetch DB secret
    info!(secret_arn = %config.db_secret_arn, "Fetching DB secret");
    let secret_value = secrets_client
        .get_secret_value()
        .secret_id(&config.db_secret_arn)
        .send()
        .await?;
    let secret_str = secret_value.secret_string().unwrap_or_default();
    let rds_secret: RdsSecret = serde_json::from_str(secret_str)?;

    // Create connection pool (use proxy endpoint instead of direct host)
    info!(proxy = %config.rds_proxy_endpoint, db = %rds_secret.dbname, "Connecting to DB via RDS Proxy");
    let conn_str = format!(
        "postgres://{}:{}@{}:{}/{}",
        rds_secret.username,
        rds_secret.password,
        config.rds_proxy_endpoint,
        rds_secret.port,
        rds_secret.dbname,
    );
    let pool = PgPool::connect(&conn_str).await?;

    // Ensure pgvector extension
    sqlx::query("CREATE EXTENSION IF NOT EXISTS vector")
        .execute(&pool)
        .await?;
    info!("pgvector extension ready");

    Ok(AppState { pool })
}

async fn handler(state: &AppState, event: LambdaEvent<SqsEvent>) -> Result<(), Error> {
    let record_count = event.payload.records.len();
    info!(records = record_count, "Processing SQS batch");

    for record in &event.payload.records {
        if let Some(body) = &record.body {
            info!(message_id = ?record.message_id, "Processing record");
            // TODO: parse chunk, generate description, embed, insert
            let _ = body;
        }
    }

    let _ = &state.pool;
    info!(records = record_count, "Batch complete");
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let state = match init().await {
        Ok(s) => s,
        Err(e) => {
            error!(error = %e, "Failed to initialize");
            return Err(e);
        }
    };

    lambda_runtime::run(service_fn(|event| handler(&state, event))).await
}
