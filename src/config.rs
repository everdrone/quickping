use std::collections::HashMap;

use serde::Deserialize;
use tokio::fs;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Credentials {
    pub id: u64,
    pub token: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Remote {
    pub url: String,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum Entry {
    Credentials(Credentials),
    Url(Remote),
}

#[derive(Debug, Deserialize)]
pub struct Config {
    pub default: Entry,
    #[serde(flatten)]
    pub others: HashMap<String, Entry>,
}

impl Config {
    pub async fn from_path(path: &str) -> Result<Self, String> {
        let text = fs::read_to_string(path)
            .await
            .map_err(|e| format!("cannot read {path}: {e}"))?;
        toml::from_str(&text).map_err(|e| format!("invalid {path}:\n{e}"))
    }

    pub fn get(&self, name: &str) -> Option<&Entry> {
        self.others.get(name)
    }
}
