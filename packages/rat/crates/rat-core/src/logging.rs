// SPDX-License-Identifier: MIT

/// Lambda용 JSON tracing 초기화.
pub fn init_lambda_tracing() {
    tracing_subscriber::fmt()
        .json()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .without_time()
        .init();
}
