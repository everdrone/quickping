use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{config::Overrides, fill};

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Message {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tts: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embeds: Option<Vec<Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_mentions: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub components: Option<Vec<Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flags: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thread_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub applied_tags: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub poll: Option<Value>,
}

impl Message {
    pub fn apply(&mut self, overrides: &Overrides) {
        fill(&mut self.username, &overrides.username);
        fill(&mut self.avatar_url, &overrides.avatar_url);
        fill(&mut self.tts, &overrides.tts);
        fill(&mut self.allowed_mentions, &overrides.allowed_mentions);
        fill(&mut self.flags, &overrides.flags);
        fill(&mut self.thread_name, &overrides.thread_name);
        fill(&mut self.applied_tags, &overrides.applied_tags);
    }
}

/// Accept a bare string or passthrough discord Webhook Content
pub fn parse(body: String) -> Message {
    serde_json::from_str(&body).unwrap_or_else(|_| Message {
        content: Some(body.trim().to_string()),
        ..Default::default()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn message_fields_win_over_overrides() {
        let mut msg = Message {
            content: Some("hi".into()),
            username: Some("from_request".into()),
            ..Default::default()
        };
        msg.apply(&Overrides {
            username: Some("from_profile".into()),
            avatar_url: Some("https://a/b.png".into()),
            ..Default::default()
        });

        assert_eq!(msg.username.as_deref(), Some("from_request"));
        assert_eq!(msg.avatar_url.as_deref(), Some("https://a/b.png"));
        assert_eq!(msg.tts, None);
    }

    #[test]
    fn bare_string_becomes_content() {
        let json = serde_json::to_value(parse("   it happened \n".into())).unwrap();
        assert_eq!(json, json!({ "content": "it happened" }));
    }

    #[test]
    fn params_pass_through() {
        let body = r#"{"content": "hi", "username": "bot", "flags": 4096,"embeds": [{ "title": "embed", "color": 5763719 }]}"#;
        let json = serde_json::to_value(parse(body.into())).unwrap();
        assert_eq!(
            json,
            json!({
                "content": "hi", "username": "bot", "flags": 4096,
                "embeds": [{ "title": "embed", "color": 5763719 }]
            })
        );
    }
}
