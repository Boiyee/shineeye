use std::{sync::Arc, time::Duration};

use axum::{
    routing::{get, post},
    Router,
};
use shineeye::{check, rules::Rules, AppState, Config};
use tower_http::services::ServeDir;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    let cfg = Config::from_env();
    if cfg.llm_api_key.is_none() && cfg.llm_base_url.contains("groq.com") {
        eprintln!("warning: LLM_API_KEY not set, running rules-only");
    }

    let state = Arc::new(AppState {
        rules: Rules::load("patterns.json")?,
        http: reqwest::Client::builder()
            .timeout(Duration::from_secs(20))
            .build()?,
        cfg,
    });

    let app = Router::new()
        .route("/health", get(|| async { "ok" }))
        .route("/api/check", post(check::handler))
        .fallback_service(ServeDir::new("static"))
        .with_state(state);

    let port = std::env::var("PORT").unwrap_or_else(|_| "3000".into());
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}")).await?;
    println!("ShineEye on http://localhost:{port}");
    axum::serve(listener, app).await?;
    Ok(())
}
