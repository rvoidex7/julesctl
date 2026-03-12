use super::JulesClient;
use anyhow::{Context, Result};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Artifact {
    pub _name: String,
    #[serde(default)]
    pub _create_time: String,
    #[serde(default)]
    pub patch_content: String,
    #[serde(default)]
    pub _artifact_type: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ListArtifactsResponse {
    #[serde(default)]
    artifacts: Vec<Artifact>,
}

impl JulesClient {
    /// Fetch all artifacts (patches) for a session.
    /// Returns the latest patch content if available.
    pub async fn get_artifacts(&self, session_id: &str) -> Result<Vec<Artifact>> {
        let url = format!("{}/sessions/{session_id}/artifacts", Self::base_url());
        let resp = self
            .http
            .get(&url)
            .query(&[("key", self.api_key.as_str())])
            .send()
            .await
            .context("HTTP request to artifacts endpoint failed")?;

        Self::check_status(&resp, "get_artifacts")?;

        // Try to parse as structured response
        let text = resp.text().await?;

        // The artifacts API may return raw patch content or structured JSON
        // Try JSON first, fall back to treating entire response as patch
        if let Ok(body) = serde_json::from_str::<ListArtifactsResponse>(&text) {
            return Ok(body.artifacts);
        }

        // If not JSON, the response itself might be a raw git patch
        if text.starts_with("diff --git") || text.starts_with("---") {
            return Ok(vec![Artifact {
                _name: format!("sessions/{session_id}/artifacts/patch"),
                _create_time: String::new(),
                patch_content: text,
                _artifact_type: "git_patch".to_string(),
            }]);
        }

        Ok(vec![])
    }

    /// Get the latest patch content for a session.
    /// Returns None if no patch available yet.
    pub async fn get_latest_patch(&self, session_id: &str) -> Result<Option<String>> {
        let artifacts = self.get_artifacts(session_id).await?;
        let patch = artifacts
            .into_iter()
            .filter(|a| !a.patch_content.is_empty())
            .last()
            .map(|a| a.patch_content);
        Ok(patch)
    }
}
