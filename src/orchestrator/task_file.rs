use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TaskFile {
    pub schema_version: String,
    pub tasks: Vec<Task>,
    #[serde(default)]
    pub messages_to_julesctl: Vec<JulesMessage>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Task {
    pub id: String,
    pub title: String,
    pub description: String,
    pub status: TaskStatus,
    #[serde(default)]
    pub session_id: String,
    #[serde(default)]
    pub depends_on: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct JulesMessage {
    pub id: String,
    #[serde(rename = "type")]
    pub msg_type: MessageType,
    #[serde(default)]
    pub processed: bool,
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum MessageType {
    OpenSession,
    ResolveConflict,
    ReorderQueue,
}

impl TaskFile {
    pub fn load(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Cannot read task file at {}", path.display()))?;
        let tf: TaskFile = serde_json::from_str(&content)
            .with_context(|| format!("Invalid JSON in task file {}", path.display()))?;
        Ok(tf)
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Return unprocessed messages in order.
    pub fn _pending_messages(&self) -> Vec<&JulesMessage> {
        self.messages_to_julesctl
            .iter()
            .filter(|m| !m.processed)
            .collect()
    }

    /// Mark a message as processed by id.
    pub fn mark_processed(&mut self, msg_id: &str) {
        if let Some(m) = self
            .messages_to_julesctl
            .iter_mut()
            .find(|m| m.id == msg_id)
        {
            m.processed = true;
        }
    }

    /// Update task session_id and status.
    pub fn update_task_session(&mut self, task_id: &str, session_id: &str, status: TaskStatus) {
        if let Some(t) = self.tasks.iter_mut().find(|t| t.id == task_id) {
            t.session_id = session_id.to_string();
            t.status = status;
        }
    }
}

/// Parse open_session payload.
pub fn parse_open_session(payload: &serde_json::Value) -> Option<(String, String)> {
    let task_id = payload.get("task_id")?.as_str()?.to_string();
    let prompt = payload.get("prompt")?.as_str()?.to_string();
    Some((task_id, prompt))
}

/// Parse resolve_conflict payload.
pub fn parse_resolve_conflict(payload: &serde_json::Value) -> Option<(String, String)> {
    let session_id = payload.get("affected_session_id")?.as_str()?.to_string();
    let patch = payload.get("resolution_patch")?.as_str()?.to_string();
    Some((session_id, patch))
}

/// Parse reorder_queue payload.
pub fn parse_reorder_queue(payload: &serde_json::Value) -> Option<Vec<String>> {
    let arr = payload.get("new_order")?.as_array()?;
    Some(
        arr.iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect(),
    )
}
