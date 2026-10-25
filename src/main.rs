use std::{error::Error, sync::Arc};

use axum::{
    Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
};
use quickping::{
    config::{Config, Credentials, Entry, Remote},
    state::AppState,
};
use serde::Deserialize;
use tokio::signal;

#[derive(Debug, Deserialize)]
struct Msg {
    msg: String,
}

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

/// Accept `{"msg": "..."}` or a bare string: anything else is used verbatim
fn parse_msg(body: String) -> String {
    match serde_json::from_str::<Msg>(&body) {
        Ok(m) => m.msg,
        Err(_) => body.trim().to_string(),
    }
}

async fn send(state: &AppState, name: &str, msg: &str) -> StatusCode {
    let result = if name == "default" {
        tracing::debug!("sending to 'default'");
        match &state.config.default {
            Entry::Credentials(c) => id_token(&state.http, c, msg).await,
            Entry::Url(r) => webhook(&state.http, r, msg).await,
        }
    } else {
        tracing::debug!("sending to '{name}'");
        match state.config.others.get(name) {
            Some(Entry::Credentials(c)) => id_token(&state.http, c, msg).await,
            Some(Entry::Url(r)) => webhook(&state.http, r, msg).await,
            None => {
                tracing::error!("no target named '{name}'");
                return StatusCode::NOT_FOUND;
            }
        }
    };

    match result {
        Ok(()) => StatusCode::OK,
        Err(e) => {
            tracing::error!("failed to send to {name}: {e}");
            StatusCode::BAD_GATEWAY
        }
    }
}

async fn webhook(http: &reqwest::Client, r: &Remote, msg: &str) -> reqwest::Result<()> {
    http.post(&r.url)
        .json(&serde_json::json!({ "content": msg }))
        .send()
        .await?
        .error_for_status()?;
    Ok(())
}

async fn id_token(http: &reqwest::Client, c: &Credentials, msg: &str) -> reqwest::Result<()> {
    http.post(format!(
        "https://discord.com/api/webhooks/{}/{}",
        c.id, c.token
    ))
    .json(&serde_json::json!({ "content": msg }))
    .send()
    .await?
    .error_for_status()?;
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c().await.expect("failed to listen to ctrl+c");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to listen to SIGTERM")
            .recv()
            .await;
    };

    #[cfg(windows)]
    let terminate = async {
        let mut close = signal::windows::ctrl_close().expect("failed to listen to CTRL_CLOSE");
        let mut shutdown =
            signal::windows::ctrl_shutdown().expect("failed to listen to CTRL_SHUTDOWN");
        tokio::select! {
            _ = close.recv() => {},
            _ = shutdown.recv() => {},
        }
    };

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    tracing::info!("shutdown signal received");
}
