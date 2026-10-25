use std::collections::HashMap;

use crate::fill;
use serde::Deserialize;
use serde_json::Value;
use tokio::fs;

#[derive(Debug, Default, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Overrides {
    pub username: Option<String>,
    pub avatar_url: Option<String>,
    pub tts: Option<bool>,
    pub allowed_mentions: Option<Value>,
    pub flags: Option<u64>,
    pub thread_name: Option<String>,
    pub applied_tags: Option<Vec<String>>,
}

impl Overrides {
    pub fn or(mut self, fallback: &Overrides) -> Overrides {
        fill(&mut self.username, &fallback.username);
        fill(&mut self.avatar_url, &fallback.avatar_url);
        fill(&mut self.tts, &fallback.tts);
        fill(&mut self.allowed_mentions, &fallback.allowed_mentions);
        fill(&mut self.flags, &fallback.flags);
        fill(&mut self.thread_name, &fallback.thread_name);
        fill(&mut self.applied_tags, &fallback.applied_tags);
        self
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Profile {
    pub url: Option<String>,
    pub id: Option<u64>,
    pub token: Option<String>,
    #[serde(flatten)]
    pub overrides: Overrides,
}

impl Profile {
    pub fn get_url(&self, name: &str) -> Result<Option<String>, String> {
        match (&self.url, self.id, &self.token) {
            (Some(url), None, None) => Ok(Some(url.clone())),
            (None, Some(id), Some(token)) => Ok(Some(format!(
                "https://discord.com/api/webhooks/{}/{}",
                id, token
            ))),
            (None, None, None) => Ok(None),
            _ => Err(format!(
                "[{name}] must set either `url` or both `id` and `token`"
            )),
        }
    }
}

#[derive(Debug)]
pub struct Target {
    pub url: String,
    pub overrides: Overrides,
}

#[derive(Debug)]
pub struct Config {
    pub targets: HashMap<String, Target>,
}

impl Config {
    pub async fn from_path(path: &str) -> Result<Self, String> {
        let text = fs::read_to_string(path)
            .await
            .map_err(|e| format!("cannot read {path}: {e}"))?;
        Self::parse(&text)
    }

    pub fn parse(text: &str) -> Result<Self, String> {
        let mut profiles: HashMap<String, Profile> =
            toml::from_str(text).map_err(|e| e.to_string())?;

        let default = profiles
            .remove("default")
            .ok_or("missing [default] profile")?;

        let default_url = default
            .get_url("default")?
            .ok_or("[default] must set `url` or both `id` and `token`")?;

        let mut targets = HashMap::new();
        for (name, profile) in profiles {
            let url = profile
                .get_url(&name)?
                .unwrap_or_else(|| default_url.clone());

            let overrides = profile.overrides.or(&default.overrides);
            targets.insert(name, Target { url, overrides });
        }

        targets.insert(
            "default".into(),
            Target {
                url: default_url,
                overrides: default.overrides,
            },
        );

        Ok(Self { targets })
    }

    pub fn get(&self, name: &str) -> Option<&Target> {
        self.targets.get(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_falls_back_to_default() {
        let cfg = Config::parse(
            r#"
            [default]
            url = "https://discord.com/api/webhooks/1/a"
            username = "qp"

            [grafana]
            avatar_url = "https://example.com/grafana.png"

            [backups]
            id = 2
            token = "b"
            username = "backups"
            "#,
        )
        .unwrap();

        let grafana = cfg.get("grafana").unwrap();
        assert_eq!(grafana.url, "https://discord.com/api/webhooks/1/a");
        assert_eq!(grafana.overrides.username, Some("qp".into()));
        assert_eq!(
            grafana.overrides.avatar_url.as_deref(),
            Some("https://example.com/grafana.png")
        );

        let backups = cfg.get("backups").unwrap();
        assert_eq!(backups.url, "https://discord.com/api/webhooks/2/b");
        assert_eq!(backups.overrides.username, Some("backups".into()));
        assert_eq!(backups.overrides.avatar_url.as_deref(), None);
    }

    #[test]
    fn reject_incomplete_credentials() {
        let err = Config::parse("[default]\nid = 1\n").unwrap_err();
        assert!(err.contains("[default]"), "{err}");

        let err = Config::parse("[default]\nurl = \"url\"\n\n[x]\ntoken = \"t\"\n").unwrap_err();
        assert!(err.contains("[x]"), "{err}");
    }

    #[test]
    fn rejects_unknown_keys() {
        let err = Config::parse("[default]\nurl = \"url\"\n\n[x]\ntypo = \"x\"\n").unwrap_err();
        assert!(err.contains("typo"), "{err}");
    }

    #[test]
    fn rejects_missing_default() {
        let err =
            Config::parse("[grafana]\nurl = \"url\"\n\nusername = \"grafana\"\n").unwrap_err();
        assert!(err.contains("[default]"), "{err}");
    }
}
