use anyhow::{Context, Result};
use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone, PartialEq)]
pub enum BranchType {
    Local,
    RemoteMain,
    JulesSession(String), // Contains session_id
}

#[derive(Debug, Clone)]
pub struct GitCommit {
    pub sha: String,
    pub short_sha: String,
    pub title: String,
    pub branch_type: BranchType,
    pub refs: Vec<String>,
}

pub fn fetch_origin(repo_path: &Path) -> Result<()> {
    let output = Command::new("git")
        .current_dir(repo_path)
        .args(["fetch", "origin"])
        .output()
        .context("Failed to run git fetch")?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow::anyhow!("git fetch failed: {}", err));
    }
    Ok(())
}

pub fn get_workflow_commits(repo_path: &Path) -> Result<Vec<GitCommit>> {
    // Format: SHA\x00ShortSHA\x00Subject\x00Refs
    let output = Command::new("git")
        .current_dir(repo_path)
        .args([
            "log",
            "--all",
            "--date-order",
            "--format=%H%x00%h%x00%s%x00%D",
            "-n",
            "100", // Limit to last 100 commits for UI performance
        ])
        .output()
        .context("Failed to run git log")?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow::anyhow!("git log failed: {}", err));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut commits = Vec::new();

    for line in stdout.lines() {
        if line.trim().is_empty() {
            continue;
        }

        let parts: Vec<&str> = line.split('\x00').collect();
        if parts.len() < 4 {
            continue;
        }

        let sha = parts[0].to_string();
        let short_sha = parts[1].to_string();
        let title = parts[2].to_string();

        // Parse refs to determine branch type (e.g., "HEAD -> main, origin/main", "origin/jules/task-1234")
        let refs_str = parts[3];
        let refs: Vec<String> = refs_str.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();

        let mut branch_type = BranchType::Local;

        for r in &refs {
            if r.contains("origin/jules/task-") || r.contains("jules/task-") {
                // Extract session ID
                let parts: Vec<&str> = r.split("task-").collect();
                if parts.len() > 1 {
                    let id = parts[1].split(&[' ', ','][..]).next().unwrap_or("").to_string();
                    branch_type = BranchType::JulesSession(id);
                    break; // Jules branch takes precedence for UI emoji
                }
            } else if r.contains("origin/") {
                branch_type = BranchType::RemoteMain;
            }
        }

        commits.push(GitCommit {
            sha,
            short_sha,
            title,
            branch_type,
            refs,
        });
    }

    Ok(commits)
}

pub fn get_commit_diff(repo_path: &Path, sha: &str) -> Result<String> {
    let output = Command::new("git")
        .current_dir(repo_path)
        .args(["show", "--color=always", sha])
        .output()?;

    if !output.status.success() {
        return Ok("Failed to get diff for this commit.".to_string());
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

pub fn apply_cherry_pick(repo_path: &Path, sha: &str) -> Result<String> {
    let output = Command::new("git")
        .current_dir(repo_path)
        .args(["cherry-pick", sha])
        .output()?;

    if output.status.success() {
        Ok(format!("Successfully applied commit {}", sha))
    } else {
        // Abort failed cherry-pick
        let _ = Command::new("git")
            .current_dir(repo_path)
            .args(["cherry-pick", "--abort"])
            .output();

        let err = String::from_utf8_lossy(&output.stderr);
        Err(anyhow::anyhow!("Conflict applying {}:\n{}", sha, err))
    }
}

pub fn revert_commit(repo_path: &Path, sha: &str) -> Result<String> {
    let output = Command::new("git")
        .current_dir(repo_path)
        .args(["revert", "--no-edit", sha])
        .output()?;

    if output.status.success() {
        Ok(format!("Successfully reverted commit {}", sha))
    } else {
        let _ = Command::new("git")
            .current_dir(repo_path)
            .args(["revert", "--abort"])
            .output();

        let err = String::from_utf8_lossy(&output.stderr);
        Err(anyhow::anyhow!("Failed to revert {}:\n{}", sha, err))
    }
}
