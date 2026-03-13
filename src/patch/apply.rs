use anyhow::{Context, Result};
use std::io::Write;
use std::path::Path;
use std::process::Command;
use tempfile::NamedTempFile;

#[derive(Debug)]
pub enum ApplyResult {
    Success,
    Conflict(ConflictInfo),
}

#[derive(Debug)]
pub struct ConflictInfo {
    pub conflicting_files: Vec<String>,
    pub patch_content: String,
    pub _error_output: String,
}

/// Dry-run first, then apply if clean.
/// Returns Conflict info if git apply --check fails.
pub fn apply_patch(repo_path: &Path, patch_content: &str) -> Result<ApplyResult> {
    // Write patch to temp file
    let mut tmp = NamedTempFile::new().context("Failed to create temp file")?;
    tmp.write_all(patch_content.as_bytes())
        .context("Failed to write patch to temp file")?;
    let tmp_path = tmp.path().to_path_buf();

    // Dry run
    let check = Command::new("git")
        .args(["apply", "--check", tmp_path.to_str().unwrap()])
        .current_dir(repo_path)
        .output()
        .context("Failed to run git apply --check")?;

    if !check.status.success() {
        let stderr = String::from_utf8_lossy(&check.stderr).to_string();
        let conflicting_files = parse_conflict_files(&stderr);
        return Ok(ApplyResult::Conflict(ConflictInfo {
            conflicting_files,
            patch_content: patch_content.to_string(),
            _error_output: stderr,
        }));
    }

    // Actually apply
    let apply = Command::new("git")
        .args(["apply", tmp_path.to_str().unwrap()])
        .current_dir(repo_path)
        .output()
        .context("Failed to run git apply")?;

    if !apply.status.success() {
        let stderr = String::from_utf8_lossy(&apply.stderr).to_string();
        anyhow::bail!("git apply failed unexpectedly after passing --check:\n{stderr}");
    }

    // Stage all changes
    Command::new("git")
        .args(["add", "-A"])
        .current_dir(repo_path)
        .status()
        .context("Failed to git add")?;

    Ok(ApplyResult::Success)
}

/// Commit staged changes with a message.
pub fn commit(repo_path: &Path, message: &str) -> Result<()> {
    let status = Command::new("git")
        .args(["commit", "-m", message])
        .current_dir(repo_path)
        .status()
        .context("Failed to git commit")?;
    if !status.success() {
        anyhow::bail!("git commit failed");
    }
    Ok(())
}

fn parse_conflict_files(stderr: &str) -> Vec<String> {
    // git apply --check stderr format:
    // "error: patch failed: src/GameScreen.kt:45"
    stderr
        .lines()
        .filter(|l| l.contains("patch failed:"))
        .map(|l| {
            l.split("patch failed:")
                .nth(1)
                .unwrap_or("")
                .trim()
                .split(':')
                .next()
                .unwrap_or("")
                .trim()
                .to_string()
        })
        .filter(|s| !s.is_empty())
        .collect()
}
