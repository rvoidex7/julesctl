use super::JulesClient;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Session {
    pub name: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub create_time: String,
    #[serde(default)]
    pub state: String,
}

impl Session {
    pub fn id(&self) -> &str {
        // name format: "sessions/14550388554331055113"
        self.name.split('/').last().unwrap_or(&self.name)
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ListSessionsResponse {
    #[serde(default)]
    sessions: Vec<Session>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CreateSessionRequest {
    prompt: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    source_context: Option<SourceContext>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SourceContext {
    repository: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    branch: String,
}

impl JulesClient {
    pub async fn list_sessions(&self) -> Result<Vec<Session>> {
        let url = format!("{}/sessions", Self::base_url());
        let resp = self
            .http
            .get(&url)
            .query(&[("key", self.api_key.as_str())])
            .send()
            .await
            .context("HTTP request to list sessions failed")?;

        Self::check_status(&resp, "list_sessions")?;
        let body: ListSessionsResponse = resp.json().await?;
        Ok(body.sessions)
    }

    pub async fn create_session(
        &self,
        prompt: &str,
        title: &str,
        github_url: Option<&str>,
        branch: Option<&str>,
    ) -> Result<Session> {
        let url = format!("{}/sessions", Self::base_url());

        let body = CreateSessionRequest {
            prompt: prompt.to_string(),
            title: title.to_string(),
            source_context: github_url
                .filter(|u| !u.trim().is_empty())
                .map(|u| SourceContext {
                    repository: u.to_string(),
                    branch: branch.unwrap_or("").to_string(),
                }),
        };

        let resp = self
            .http
            .post(&url)
            .query(&[("key", self.api_key.as_str())])
            .json(&body)
            .send()
            .await
            .context("HTTP request to create session failed")?;

        Self::check_status(&resp, "create_session")?;
        let session: Session = resp.json().await?;
        Ok(session)
    }
}
