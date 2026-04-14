use lambda_runtime::{service_fn, Error, LambdaEvent};
use rat_core::db;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use tracing::{error, info};

mod actions;

#[derive(Deserialize)]
struct Config {
    rds_proxy_endpoint: String,
    db_secret_arn: String,
}

pub struct AppState {
    pool: PgPool,
    bedrock: aws_sdk_bedrockruntime::Client,
}

#[derive(Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
enum ApiRequest {
    Search(actions::search::SearchRequest),
    List(actions::list::ListRequest),
    Purge(actions::purge::PurgeRequest),
    RepoCreate(actions::repo_create::RepoCreateRequest),
}

#[derive(Serialize)]
#[serde(untagged)]
enum ApiResponse {
    Search(actions::search::SearchResponse),
    List(actions::list::ListResponse),
    Purge(actions::purge::PurgeResponse),
    RepoCreate(actions::repo_create::RepoCreateResponse),
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
    Ok(AppState { pool, bedrock })
}

async fn handler(
    state: &AppState,
    event: LambdaEvent<ApiRequest>,
) -> Result<ApiResponse, Error> {
    match event.payload {
        ApiRequest::Search(req) => actions::search::handle_search(state, req).await.map(ApiResponse::Search),
        ApiRequest::List(req) => actions::list::handle_list(state, req).await.map(ApiResponse::List),
        ApiRequest::Purge(req) => actions::purge::handle_purge(state, req).await.map(ApiResponse::Purge),
        ApiRequest::RepoCreate(req) => actions::repo_create::handle_repo_create(state, req).await.map(ApiResponse::RepoCreate),
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
