use reqwest::StatusCode;
use tokio::signal;

use crate::{config::Config, message::Message};

pub mod config;
pub mod message;

pub struct AppState {
    pub config: Config,
    pub http: reqwest::Client,
}

pub fn fill<T: Clone>(slot: &mut Option<T>, value: &Option<T>) {
    if slot.is_none() {
        *slot = value.clone();
    }
}

pub async fn send(state: &AppState, name: &str, mut msg: Message) -> StatusCode {
    let Some(target) = state.config.get(name) else {
        tracing::error!("no target named '{name}'");
        return StatusCode::NOT_FOUND;
    };

    msg.apply(&target.overrides);

    tracing::debug!("sending to '{name}'");

    let result = state
        .http
        .post(&target.url)
        .json(&msg)
        .send()
        .await
        .and_then(|r| r.error_for_status());

    match result {
        Ok(_) => StatusCode::OK,
        Err(e) => {
            tracing::error!("failed to send to {name}: {e}");
            StatusCode::BAD_GATEWAY
        }
    }
}

pub async fn shutdown_signal() {
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
