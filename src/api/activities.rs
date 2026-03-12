use super::JulesClient;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Activity {
    pub name: String,
    #[serde(default)]
    pub create_time: String,
    #[serde(default)]
    pub message: Option<MessagePayload>,
    #[serde(default)]
    pub plan: Option<PlanPayload>,
    #[serde(default)]
    pub github_push: Option<GitHubPushPayload>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MessagePayload {
    #[serde(default)]
    pub author: String,
    pub text: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanPayload {
    pub description: String,
    #[serde(default)]
    pub status: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitHubPushPayload {
    #[serde(default)]
    pub branch: String,
    #[serde(default)]
    pub commit_sha: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ListActivitiesResponse {
    #[serde(default)]
    activities: Vec<Activity>,
}

impl JulesClient {
    pub async fn get_activities(&self, session_id: &str, page_size: u32) -> Result<Vec<Activity>> {
        let url = format!("{}/sessions/{session_id}/activities", Self::base_url());
        let resp = self
            .http
            .get(&url)
            .query(&[
                ("key", self.api_key.as_str()),
                ("pageSize", &page_size.to_string()),
                ("orderBy", "create_time desc"),
            ])
            .send()
            .await
            .context("HTTP request to activities endpoint failed")?;

        Self::check_status(&resp, "get_activities")?;
        let body: ListActivitiesResponse = resp.json().await?;
        let mut acts = body.activities;
        acts.reverse();
        Ok(acts)
    }

    pub async fn get_activities_after(
        &self,
        session_id: &str,
        after_name: &str,
        page_size: u32,
    ) -> Result<Vec<Activity>> {
        let acts = self.get_activities(session_id, page_size).await?;
        let new: Vec<Activity> = acts
            .into_iter()
            .skip_while(|a| a.name != after_name)
            .skip(1)
            .collect();
        Ok(new)
    }

    pub async fn send_message(&self, session_id: &str, text: &str) -> Result<()> {
        let url = format!("{}/sessions/{session_id}:sendMessage", Self::base_url());
        #[derive(serde::Serialize)]
        struct Body<'a> {
            prompt: &'a str,
        }
        let resp = self
            .http
            .post(&url)
            .query(&[("key", self.api_key.as_str())])
            .json(&Body { prompt: text })
            .send()
            .await
            .context("HTTP request to sendMessage failed")?;
        Self::check_status(&resp, "send_message")?;
        Ok(())
    }
}
