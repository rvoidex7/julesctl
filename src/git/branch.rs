use anyhow::{Context, Result};
use std::path::Path;
use std::process::Command;

pub const ORCHESTRATOR_BRANCH: &str = "jules-orchestrator";

/// Ensure the orchestrator branch exists and check it out.
pub fn ensure_orchestrator_branch(repo_path: &Path) -> Result<()> {
    let branches = Command::new("git")
        .args(["branch", "--list", ORCHESTRATOR_BRANCH])
        .current_dir(repo_path)
        .output()
        .context("Failed to list git branches")?;

    let exists = !String::from_utf8_lossy(&branches.stdout).trim().is_empty();

    if exists {
        // Already exists — just check it out
        let status = Command::new("git")
            .args(["checkout", ORCHESTRATOR_BRANCH])
            .current_dir(repo_path)
            .status()?;
        if !status.success() {
            anyhow::bail!("Failed to checkout {ORCHESTRATOR_BRANCH}");
        }
    } else {
        // Create from current HEAD
        let status = Command::new("git")
            .args(["checkout", "-b", ORCHESTRATOR_BRANCH])
            .current_dir(repo_path)
            .status()?;
        if !status.success() {
            anyhow::bail!("Failed to create {ORCHESTRATOR_BRANCH}");
        }
    }

    Ok(())
}

pub fn current_branch(repo_path: &Path) -> Result<String> {
    let output = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(repo_path)
        .output()
        .context("Failed to get current branch")?;
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

pub fn head_sha(repo_path: &Path) -> Result<String> {
    let output = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .current_dir(repo_path)
        .output()
        .context("Failed to get HEAD sha")?;
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

pub fn run_hook(repo_path: &Path, cmd: &str) -> Result<()> {
    if cmd.is_empty() {
        return Ok(());
    }
    let status = Command::new("sh")
        .args(["-c", cmd])
        .current_dir(repo_path)
        .status()
        .with_context(|| format!("Failed to run hook: {cmd}"))?;
    if !status.success() {
        eprintln!("⚠ Hook exited with {}", status.code().unwrap_or(-1));
    }
    Ok(())
}
