use anyhow::{Context, Result};
use std::path::Path;
use std::process::Command;

pub fn pull(repo_path: &Path) -> Result<bool> {
    let output = Command::new("git")
        .args(["pull", "--ff-only"])
        .current_dir(repo_path)
        .output()
        .context("Failed to run git pull")?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if !output.status.success() {
        anyhow::bail!("git pull failed:\n{stderr}");
    }

    Ok(!stdout.contains("Already up to date"))
}
