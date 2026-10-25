use reqwest::StatusCode;
use tokio::signal;

use crate::{
    config::{Credentials, Entry, Remote},
    message::Message,
    state::AppState,
};

pub mod config;
pub mod message;
pub mod state;

/// Accept a bare string or passthrough discord Webhook Content
pub fn parse_msg(body: String) -> Message {
    serde_json::from_str(&body).unwrap_or_else(|_| Message {
        content: Some(body.trim().to_string()),
        ..Default::default()
    })
}

pub async fn send(state: &AppState, name: &str, msg: &Message) -> StatusCode {
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

async fn webhook(http: &reqwest::Client, r: &Remote, msg: &Message) -> reqwest::Result<()> {
    http.post(&r.url)
        .json(msg)
        .send()
        .await?
        .error_for_status()?;
    Ok(())
}

async fn id_token(http: &reqwest::Client, c: &Credentials, msg: &Message) -> reqwest::Result<()> {
    http.post(format!(
        "https://discord.com/api/webhooks/{}/{}",
        c.id, c.token
    ))
    .json(msg)
    .send()
    .await?
    .error_for_status()?;
    Ok(())
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

#[cfg(test)]
mod tests {
    use crate::parse_msg;
    use serde_json::json;

    #[test]
    fn bare_string_becomes_content() {
        let json = serde_json::to_value(parse_msg("   it happened \n".into())).unwrap();
        assert_eq!(json, json!({ "content": "it happened" }));
    }

    #[test]
    fn params_pass_through() {
        let body = r#"{"content": "hi", "username": "bot", "flags": 4096,"embeds": [{ "title": "embed", "color": 5763719 }]}"#;
        let json = serde_json::to_value(parse_msg(body.into())).unwrap();
        assert_eq!(
            json,
            json!({
                "content": "hi", "username": "bot", "flags": 4096,
                "embeds": [{ "title": "embed", "color": 5763719 }]
            })
        );
    }
}
