// SPDX-License-Identifier: MIT

use aws_sdk_lambda::Client as LambdaClient;
use lambda_runtime::{Error, LambdaEvent, run, service_fn};
use rat_client::ops;
use serde::Deserialize;
use serde_json::{Value, json};
use tracing::info;

#[derive(Deserialize)]
struct Config {
    api_function_arn: String,
}

struct AppState {
    lambda: LambdaClient,
    api_function_arn: String,
}

async fn init() -> Result<AppState, Error> {
    rat_core::logging::init_lambda_tracing();

    let config: Config = envy::from_env()?;
    let aws_config = aws_config::defaults(aws_config::BehaviorVersion::latest())
        .load()
        .await;
    Ok(AppState {
        lambda: LambdaClient::new(&aws_config),
        api_function_arn: config.api_function_arn,
    })
}

#[derive(Deserialize)]
struct SearchParams {
    query: String,
    #[serde(default)]
    repo_id: Option<String>,
    #[serde(default)]
    source_type: Option<String>,
    #[serde(default)]
    limit: Option<i64>,
}

#[derive(Deserialize)]
struct RepoSearchParams {
    query: String,
    #[serde(default)]
    limit: Option<i64>,
}

#[derive(Deserialize)]
struct FileGetParams {
    repo_id: String,
    source_path: String,
}

async fn handler(state: &AppState, event: LambdaEvent<Value>) -> Result<Value, Error> {
    let (payload, ctx) = event.into_parts();

    let tool_name = ctx
        .client_context
        .as_ref()
        .and_then(|c| c.custom.get("bedrockAgentCoreToolName"))
        .cloned()
        .unwrap_or_default();

    let action = tool_name
        .rsplit_once("___")
        .map(|(_, a)| a)
        .unwrap_or(&tool_name);

    info!(tool = %tool_name, action = %action, "MCP tool invocation");

    match action {
        "search" => {
            let p: SearchParams = serde_json::from_value(payload)?;
            let results = ops::search(
                &state.lambda,
                &state.api_function_arn,
                &p.query,
                p.repo_id.as_deref(),
                p.source_type.as_deref().unwrap_or("code"),
                p.limit.unwrap_or(3),
            )
            .await?;
            Ok(json!({ "results": results }))
        }
        "search_repos" => {
            let p: RepoSearchParams = serde_json::from_value(payload)?;
            let results = ops::repo_search(
                &state.lambda,
                &state.api_function_arn,
                &p.query,
                p.limit.unwrap_or(5),
            )
            .await?;
            Ok(json!({ "results": results }))
        }
        "list_repos" => {
            let repos = ops::list(&state.lambda, &state.api_function_arn).await?;
            Ok(json!({ "repos": repos }))
        }
        "file_get" => {
            let p: FileGetParams = serde_json::from_value(payload)?;
            let file = ops::file_get(
                &state.lambda,
                &state.api_function_arn,
                &p.repo_id,
                &p.source_path,
            )
            .await?;
            Ok(json!({ "file": file }))
        }
        _ => Err(format!("Unknown tool: {tool_name}").into()),
    }
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let state = init().await?;
    run(service_fn(|event| handler(&state, event))).await
}
