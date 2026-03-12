use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct Config {
    pub api_key: String,
    #[serde(default)]
    pub repos: Vec<RepoConfig>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct RepoConfig {
    pub path: String,
    pub display_name: String,
    pub mode: RepoMode,
    #[serde(default)]
    pub post_pull: String,

    // Mode 1
    #[serde(default)]
    pub single_session_id: String,

    // Mode 2
    #[serde(default)]
    pub manager_session_id: String,
    #[serde(default = "default_task_file")]
    pub task_file: String,

    // Mode 3: sessions added dynamically, stored here
    #[serde(default)]
    pub manual_sessions: Vec<ManualSession>,
}

fn default_task_file() -> String {
    ".julesctl-tasks.json".to_string()
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum RepoMode {
    Single,
    Orchestrated,
    Manual,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ManualSession {
    pub session_id: String,
    pub label: String,
    pub queue_position: usize,
}

impl Config {
    pub fn find_repo(&self, cwd: &Path) -> Option<&RepoConfig> {
        self.repos.iter().find(|r| {
            let rp = PathBuf::from(&r.path);
            cwd.starts_with(&rp) || cwd == rp
        })
    }

    pub fn find_repo_mut(&mut self, cwd: &Path) -> Option<&mut RepoConfig> {
        self.repos.iter_mut().find(|r| {
            let rp = PathBuf::from(&r.path);
            cwd.starts_with(&rp) || cwd == rp
        })
    }
}
