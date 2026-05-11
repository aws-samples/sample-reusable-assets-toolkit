// SPDX-License-Identifier: MIT

use aws_lambda_events::sqs::SqsEvent;
use aws_sdk_bedrockruntime::Client as BedrockClient;
use lambda_runtime::{service_fn, Error, LambdaEvent};
use pgvector::Vector;
use rat_core::{db, embedding, queries, summary, summary::SummaryContext};
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

    let aws_config = aws_config::defaults(aws_config::BehaviorVersion::latest())
        .region(aws_config::Region::new("us-east-1"))
        .load()
        .await;
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

        info!(action = ?msg.action, repo_id = %msg.repo_id, source_path = %msg.source_path, "Processing message");

        let result = match msg.action {
            Action::Upsert => handle_upsert(state, &msg).await,
            Action::Delete => handle_delete(state, &msg).await,
        };

        if let Err(e) = result {
            error!(error = %e, action = ?msg.action, repo_id = %msg.repo_id, source_path = %msg.source_path, "Failed to process message");
        }
    }

    info!(records = record_count, "Batch complete");
    Ok(())
}

async fn handle_upsert(state: &AppState, msg: &FileMessage) -> Result<(), Error> {
    let file_rec = build_file_record(msg)?;
    let snippet_recs = build_snippet_records(msg);

    let mut tx = state.pool.begin().await?;

    if queries::get_repo(&mut *tx, &msg.repo_id).await?.is_none() {
        warn!(repo_id = %msg.repo_id, source_path = %msg.source_path, "repo not found, skipping upsert");
        drop(tx);
        return Ok(());
    }

    let file_id = queries::upsert_file(
        &mut *tx,
        file_rec.repo_id,
        file_rec.source_path,
        file_rec.content,
        file_rec.language,
    )
    .await?;

    queries::delete_snippets_by_file(&mut *tx, file_id).await?;

    info!(file_id, chunks = snippet_recs.len(), "Inserting snippets");

    for rec in &snippet_recs {
        let ctx = SummaryContext {
            source_path: file_rec.source_path,
            language: file_rec.language,
            source_type: rec.source_type,
        };
        let description = summary::generate_summary(
            &state.bedrock,
            &state.summary_model_id,
            rec.content,
            &ctx,
        )
        .await
        .map_err(|e| format!("summary error: {e}"))?;

        let emb = embedding::generate_embedding(&state.bedrock, &description, "GENERIC_INDEX")
            .await
            .map_err(|e| format!("embedding error: {e}"))?;

        queries::insert_snippet(
            &mut *tx,
            file_id,
            rec.repo_id,
            rec.content,
            &description,
            Vector::from(emb),
            rec.source_type,
            rec.start_line,
            rec.end_line,
        )
        .await?;
    }

    tx.commit().await?;

    info!(file_id, "Upsert complete");
    Ok(())
}

async fn handle_delete(state: &AppState, msg: &FileMessage) -> Result<(), Error> {
    if queries::get_repo(&state.pool, &msg.repo_id).await?.is_none() {
        warn!(repo_id = %msg.repo_id, source_path = %msg.source_path, "repo not found, skipping delete");
        return Ok(());
    }

    let rows = queries::delete_file(&state.pool, &msg.repo_id, &msg.source_path).await?;

    info!(rows, repo_id = %msg.repo_id, source_path = %msg.source_path, "Delete complete");
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
