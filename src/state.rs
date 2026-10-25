use crate::config::Config;

pub struct AppState {
    pub config: Config,
    pub http: reqwest::Client,
}
