use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct Config {
    pub api_key: String,
    #[serde(default)]
    pub repos: Vec<RepoConfig>,

    // Global active tabs for Workflow view
    #[serde(default)]
    pub active_tabs: Vec<usize>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct RepoConfig {
    pub path: String,
    pub display_name: String,

    #[serde(default)]
    pub github_url: String, // Useful for the API context

    #[serde(default)]
    pub post_pull: String,

    // We store sessions here just for UI labeling purposes and hierarchy
    #[serde(default)]
    pub sessions: Vec<JulesSession>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct JulesSession {
    pub session_id: String,
    pub label: String,

    // Hierarchical mapping of Jules sessions (Session A spawned Session B)
    #[serde(default)]
    pub parent_id: Option<String>,
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
