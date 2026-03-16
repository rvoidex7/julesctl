pub mod activities;
pub mod sessions;

use anyhow::{bail, Result};
use reqwest::{Client, Response, StatusCode};

const BASE_URL: &str = "https://jules.googleapis.com/v1alpha";

#[derive(Clone)]
pub struct JulesClient {
    pub http: Client,
    pub api_key: String,
}

impl JulesClient {
    pub fn new(api_key: &str) -> Self {
        let http = Client::builder()
            .user_agent("julesctl/0.1")
            .build()
            .expect("Failed to build HTTP client");
        Self {
            http,
            api_key: api_key.to_string(),
        }
    }

    pub fn base_url() -> &'static str {
        BASE_URL
    }

    pub fn check_status(resp: &Response, op: &str) -> Result<()> {
        match resp.status() {
            StatusCode::OK | StatusCode::CREATED | StatusCode::NO_CONTENT => Ok(()),
            StatusCode::UNAUTHORIZED => bail!("[{op}] 401 Unauthorized — check JULES_API_KEY"),
            StatusCode::NOT_FOUND => bail!("[{op}] 404 Not Found — session_id may be wrong"),
            StatusCode::TOO_MANY_REQUESTS => bail!("[{op}] 429 Rate limited — slow down polling"),
            s => bail!("[{op}] Unexpected HTTP {s}"),
        }
    }

    pub fn api_key(&self) -> &str {
        &self.api_key
    }
}
