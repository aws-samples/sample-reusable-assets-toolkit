use lambda_runtime::{service_fn, Error, LambdaEvent};
use rat_core::api::{ApiRequest, ApiResponse};
use rat_core::db;
use serde::Deserialize;
use sqlx::PgPool;
use tracing::{error, info};

mod actions;

#[derive(Deserialize)]
struct Config {
    rds_proxy_endpoint: String,
    db_secret_arn: String,
    summary_model_id: String,
}

pub struct AppState {
    pub pool: PgPool,
    pub bedrock: aws_sdk_bedrockruntime::Client,
    pub summary_model_id: String,
}

async fn init() -> Result<AppState, Error> {
    rat_core::logging::init_lambda_tracing();

    info!("Initializing API Lambda");
    let config: Config = envy::from_env()?;
    let pool =
        db::create_pool_from_secret(&config.db_secret_arn, &config.rds_proxy_endpoint).await?;

    let bedrock_config = aws_config::defaults(aws_config::BehaviorVersion::latest())
        .region(aws_config::Region::new("us-east-1"))
        .load()
        .await;
    let bedrock = aws_sdk_bedrockruntime::Client::new(&bedrock_config);

    info!("API Lambda ready");
    Ok(AppState {
        pool,
        bedrock,
        summary_model_id: config.summary_model_id,
    })
}

async fn handler(
    state: &AppState,
    event: LambdaEvent<ApiRequest>,
) -> Result<ApiResponse, Error> {
    match event.payload {
        ApiRequest::Search(req) => actions::search::handle_search(state, req).await.map(ApiResponse::Search),
        ApiRequest::List(req) => actions::list::handle_list(state, req).await.map(ApiResponse::List),
        ApiRequest::Purge(req) => actions::purge::handle_purge(state, req).await.map(ApiResponse::Purge),
        ApiRequest::RepoUpsert(req) => actions::repo_upsert::handle_repo_upsert(state, req).await.map(ApiResponse::RepoUpsert),
        ApiRequest::RepoGet(req) => actions::repo_get::handle_repo_get(state, req).await.map(ApiResponse::RepoGet),
        ApiRequest::RepoSearch(req) => actions::repo_search::handle_repo_search(state, req).await.map(ApiResponse::RepoSearch),
        ApiRequest::FileGet(req) => actions::file_get::handle_file_get(state, req).await.map(ApiResponse::FileGet),
    }
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
