use aws_lambda_events::sqs::SqsEvent;
use aws_sdk_bedrockruntime::Client as BedrockClient;
use lambda_runtime::{service_fn, Error, LambdaEvent};
use pgvector::Vector;
use rat_core::{db, embedding, summary};
use rat_lambda::{build_file_record, build_snippet_records, Action, FileMessage};
use serde::Deserialize;
use sqlx::PgPool;
use tracing::{error, info, warn};

#[derive(Deserialize)]
struct Config {
    rds_proxy_endpoint: String,
    db_secret_arn: String,
    summary_model_id: String,
}

struct AppState {
    pool: PgPool,
    bedrock: BedrockClient,
    summary_model_id: String,
}

async fn init() -> Result<AppState, Error> {
    rat_core::logging::init_lambda_tracing();

    info!("Initializing Lambda");
    let config: Config = envy::from_env()?;
    let pool =
        db::create_pool_from_secret(&config.db_secret_arn, &config.rds_proxy_endpoint).await?;
    info!("DB pool ready");

    let aws_config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
    let bedrock = BedrockClient::new(&aws_config);

    Ok(AppState {
        pool,
        bedrock,
        summary_model_id: config.summary_model_id,
    })
}

async fn handler(state: &AppState, event: LambdaEvent<SqsEvent>) -> Result<(), Error> {
    let record_count = event.payload.records.len();
    info!(records = record_count, "Processing SQS batch");

    for record in &event.payload.records {
        let body = match &record.body {
            Some(b) => b,
            None => continue,
        };

        let msg: FileMessage = match serde_json::from_str(body) {
            Ok(m) => m,
            Err(e) => {
                error!(error = %e, message_id = ?record.message_id, "Failed to parse message");
                continue;
            }
        };

        info!(action = ?msg.action, repo_id = %msg.repo_id, source_path = ?msg.source_path, "Processing message");

        let result = match msg.action {
            Action::Upsert => handle_upsert(state, &msg).await,
            Action::Delete => handle_delete(state, &msg).await,
            Action::Purge => handle_purge(state, &msg).await,
        };

        if let Err(e) = result {
            error!(error = %e, action = ?msg.action, repo_id = %msg.repo_id, source_path = ?msg.source_path, "Failed to process message");
        }
    }

    info!(records = record_count, "Batch complete");
    Ok(())
}

async fn handle_upsert(state: &AppState, msg: &FileMessage) -> Result<(), Error> {
    let file_rec = build_file_record(msg)?;
    let snippet_recs = build_snippet_records(msg);

    let mut tx = state.pool.begin().await?;

    let file_id: i64 = sqlx::query_scalar(
        "INSERT INTO files (repo_id, source_path, commit_id, content, language)
         VALUES ($1, $2, $3, $4, $5)
         ON CONFLICT (repo_id, source_path) DO UPDATE
         SET commit_id = EXCLUDED.commit_id, content = EXCLUDED.content, language = EXCLUDED.language
         RETURNING id",
    )
    .bind(file_rec.repo_id)
    .bind(file_rec.source_path)
    .bind(file_rec.commit_id)
    .bind(file_rec.content)
    .bind(file_rec.language)
    .fetch_one(&mut *tx)
    .await?;

    sqlx::query("DELETE FROM snippets WHERE file_id = $1")
        .bind(file_id)
        .execute(&mut *tx)
        .await?;

    info!(file_id, chunks = snippet_recs.len(), "Inserting snippets");

    for rec in &snippet_recs {
        let description =
            summary::generate_summary(&state.bedrock, &state.summary_model_id, rec.content)
                .await
                .map_err(|e| format!("summary error: {e}"))?;

        let emb = embedding::generate_embedding(&state.bedrock, &description)
            .await
            .map_err(|e| format!("embedding error: {e}"))?;

        sqlx::query(
            "INSERT INTO snippets (file_id, repo_id, content, description, embedding, source_type, start_line, end_line)
             VALUES ($1, $2, $3, $4, $5::vector, $6, $7, $8)",
        )
        .bind(file_id)
        .bind(rec.repo_id)
        .bind(rec.content)
        .bind(&description)
        .bind(Vector::from(emb))
        .bind(rec.source_type)
        .bind(rec.start_line)
        .bind(rec.end_line)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;

    info!(file_id, "Upsert complete");
    Ok(())
}

async fn handle_delete(state: &AppState, msg: &FileMessage) -> Result<(), Error> {
    let source_path = msg.source_path.as_deref().ok_or("delete requires source_path")?;

    let result = sqlx::query("DELETE FROM files WHERE repo_id = $1 AND source_path = $2")
        .bind(&msg.repo_id)
        .bind(source_path)
        .execute(&state.pool)
        .await?;

    info!(rows = result.rows_affected(), repo_id = %msg.repo_id, source_path, "Delete complete");
    Ok(())
}

async fn handle_purge(state: &AppState, msg: &FileMessage) -> Result<(), Error> {
    let result = sqlx::query("DELETE FROM files WHERE repo_id = $1")
        .bind(&msg.repo_id)
        .execute(&state.pool)
        .await?;

    warn!(rows = result.rows_affected(), repo_id = %msg.repo_id, "Purge complete");
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
