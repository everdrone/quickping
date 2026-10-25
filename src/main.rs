use std::{error::Error, sync::Arc};

use axum::{
    Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
};
use quickping::{config::Config, parse_msg, send, shutdown_signal, state::AppState};

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn Error>> {
    // install rustls provider
    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("crypto provider");

    // initialize tracing
    tracing_subscriber::fmt::init();

    let config_path = std::env::var("QP_CONFIG").unwrap_or("/etc/quickping/config.toml".into());

    let config = Config::from_path(&config_path).await?;

    let state = Arc::new(AppState {
        config,
        http: reqwest::Client::new(),
    });

    let app = Router::new()
        .route("/healthz", get(healtz))
        .route("/", post(post_default))
        .route("/{name}", post(post_with_name))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    tracing::info!("serving http://0.0.0.0:3000");
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}

async fn healtz() -> StatusCode {
    StatusCode::OK
}

async fn post_default(State(state): State<Arc<AppState>>, body: String) -> StatusCode {
    send(&state, "default", &parse_msg(body)).await
}

async fn post_with_name(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
    body: String,
) -> StatusCode {
    send(&state, &name, &parse_msg(body)).await
}
