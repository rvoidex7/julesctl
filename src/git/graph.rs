use anyhow::{Context, Result};
use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone)]
pub struct GitCommit {
    pub sha: String,
    pub short_sha: String,
    pub title: String,
    pub branch: Option<String>,
    pub is_jules: bool,
    pub is_remote: bool,
}

pub fn get_workflow_commits(repo_path: &Path) -> Result<Vec<GitCommit>> {
    // Basic implementation that will get replaced with real Git parser.
    // For now we just return a dummy node list as a placeholder for the Dashboard TUI.
    Ok(vec![GitCommit {
        sha: "abcdef123456789".to_string(),
        short_sha: "abcdef1".to_string(),
        title: "Initial commit".to_string(),
        branch: Some("main".to_string()),
        is_jules: false,
        is_remote: true,
    }])
}

pub fn fetch_origin(repo_path: &Path) -> Result<()> {
    Command::new("git")
        .current_dir(repo_path)
        .args(["fetch", "origin"])
        .output()
        .context("Failed to run git fetch")?;
    Ok(())
}
