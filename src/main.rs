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

#[derive(Debug, Deserialize)]
struct Msg {
    msg: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
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
    axum::serve(listener, app).await?;
    Ok(())
}

async fn healtz() -> StatusCode {
    StatusCode::OK
}

async fn post_default(State(state): State<Arc<AppState>>, body: String) -> (StatusCode, String) {
    send(&state, "default", &parse_msg(body)).await
}

async fn post_with_name(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
    body: String,
) -> (StatusCode, String) {
    send(&state, &name, &parse_msg(body)).await
}

/// Accept `{"msg": "..."}` or a bare string: anything else is used verbatim
fn parse_msg(body: String) -> String {
    match serde_json::from_str::<Msg>(&body) {
        Ok(m) => m.msg,
        Err(_) => body.trim().to_string(),
    }
}

async fn send(state: &AppState, name: &str, msg: &str) -> (StatusCode, String) {
    let result = if name == "default" {
        match &state.config.default {
            Entry::Credentials(c) => id_token(&state.http, c, msg).await,
            Entry::Url(r) => webhook(&state.http, r, msg).await,
        }
    } else {
        match state.config.others.get(name) {
            Some(Entry::Credentials(c)) => id_token(&state.http, c, msg).await,
            Some(Entry::Url(r)) => webhook(&state.http, r, msg).await,
            None => {
                tracing::warn!("no target named '{name}'");
                return (StatusCode::NOT_FOUND, format!("no target named '{name}'\n"));
            }
        }
    };

    match result {
        Ok(()) => (StatusCode::OK, format!("sent to {name}\n")),
        Err(e) => (
            StatusCode::BAD_GATEWAY,
            format!("failed to send to {name}: {e}\n"),
        ),
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
