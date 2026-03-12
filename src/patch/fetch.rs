use crate::api::JulesClient;
use anyhow::Result;

/// Fetch the latest patch for a session from Jules artifacts API.
/// Returns None if Jules hasn't produced any changes yet.
pub async fn fetch_patch(client: &JulesClient, session_id: &str) -> Result<Option<String>> {
    client.get_latest_patch(session_id).await
}
