use aws_lambda_events::sqs::SqsEvent;
use lambda_runtime::{service_fn, Error, LambdaEvent};
use rat_core::db;
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
    rat_core::logging::init_lambda_tracing();

    info!("Initializing Lambda");
    let config: Config = envy::from_env()?;
    let pool =
        db::create_pool_from_secret(&config.db_secret_arn, &config.rds_proxy_endpoint).await?;
    info!("DB pool ready");

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
