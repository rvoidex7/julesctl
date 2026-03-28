use anyhow::{Context, Result};
use std::path::Path;
use tokio::process::Command;

#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]
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
    pub graph_prefix: String,
}

/// Heavy network operations remain shell commands
pub async fn fetch_origin(repo_path: &Path) -> Result<()> {
    let output = Command::new("git")
        .current_dir(repo_path)
        .args(["fetch", "origin"])
        .output()
        .await
        .context("Failed to run git fetch")?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow::anyhow!("git fetch failed: {}", err));
    }
    Ok(())
}

pub async fn get_workflow_commits(repo_path: &Path, workflow_only: bool) -> Result<Vec<GitCommit>> {
    // Check if git is initialized and has commits
    let check = Command::new("git")
        .current_dir(repo_path)
        .args(["rev-parse", "HEAD"])
        .output()
        .await;

    if check.is_err() || !check.unwrap().status.success() {
        return Ok(Vec::new()); // No commits or not a git repo
    }

    let mut args = vec![
        "log",
        "--date-order",
        "--graph",
        "--format=%H%x00%h%x00%s%x00%D",
        "-n",
        "100",
    ];

    if workflow_only {
        // Only show branches that are jules/task or main/HEAD (The direct workflow context)
        // We do this by pointing to HEAD, and branches matching jules/*
        args.push("HEAD");
        args.push("--branches=jules/*");
        args.push("--remotes=origin/jules/*");
    } else {
        args.push("--all");
    }

    // Format: %H%x00%h%x00%s%x00%D  but we add --graph so git prepends ascii lines like "| * | "
    let output = Command::new("git")
        .current_dir(repo_path)
        .args(&args)
        .output()
        .await
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

        // Because we use --graph, the line starts with ascii art, then our %H etc...
        // So we need to split at the first 40 character SHA (which will be preceded by ascii graph).
        // To make it easy, we split by the first \x00. The graph + SHA will be in the first part.
        let parts: Vec<&str> = line.split('\x00').collect();
        if parts.len() < 4 {
            // It might just be a graph connecting line with no commit data (e.g., "| |")
            commits.push(GitCommit {
                sha: String::new(),
                short_sha: String::new(),
                title: String::new(),
                branch_type: BranchType::Local,
                graph_prefix: line.to_string(), // Just keep the art
            });
            continue;
        }

        // Part 0 contains: "GRAPH_ART SHA"
        // Let's separate the graph from the SHA
        let graph_and_sha = parts[0];
        // Git SHA is 40 chars. Let's find where it starts.
        let mut graph_prefix = String::new();
        let sha;

        if graph_and_sha.len() >= 40 {
            let split_idx = graph_and_sha.len() - 40;
            graph_prefix = graph_and_sha[..split_idx].to_string();
            sha = graph_and_sha[split_idx..].to_string();
        } else {
            sha = graph_and_sha.to_string();
        }

        let short_sha = parts[1].to_string();
        let title = parts[2].to_string();

        // Parse refs to determine branch type (e.g., "HEAD -> main, origin/main", "origin/jules/task-1234")
        let refs_str = parts[3];
        let refs: Vec<String> = refs_str
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        let mut branch_type = BranchType::Local;

        for r in &refs {
            if r.contains("origin/jules/task-") || r.contains("jules/task-") {
                // Extract session ID
                let parts: Vec<&str> = r.split("task-").collect();
                if parts.len() > 1 {
                    let id = parts[1]
                        .split(&[' ', ','][..])
                        .next()
                        .unwrap_or("")
                        .to_string();
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
            graph_prefix,
        });
    }

    Ok(commits)
}

pub async fn get_commit_diff(repo_path: &Path, sha: &str) -> Result<String> {
    // We use standard git show for full metadata, but the diff generation logic
    // can be optimized via diffy in future iterations for purely local text changes.
    // For now, this is incredibly fast asynchronously.
    let output = Command::new("git")
        .current_dir(repo_path)
        .args(["show", "--color=always", sha])
        .output()
        .await?;

    if !output.status.success() {
        return Ok("Failed to get diff for this commit.".to_string());
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Blazing fast in-memory red/green diff generation using the diffy crate.
/// This fulfills Task 9 by providing a way to generate patch previews entirely in memory
/// without needing heavy `git diff` disk interactions for raw text comparisons (e.g. from API artifacts).
pub fn generate_memory_diff(original: &str, modified: &str) -> String {
    let patch = diffy::create_patch(original, modified);
    let f = diffy::PatchFormatter::new().with_color();
    let x = f.fmt_patch(&patch).to_string();
    x
}

pub async fn apply_cherry_pick(repo_path: &Path, sha: &str) -> Result<String> {
    let output = Command::new("git")
        .current_dir(repo_path)
        .args(["cherry-pick", sha])
        .output()
        .await?;

    if output.status.success() {
        Ok(format!("Successfully applied commit {}", sha))
    } else {
        // Abort failed cherry-pick
        let _ = Command::new("git")
            .current_dir(repo_path)
            .args(["cherry-pick", "--abort"])
            .output()
            .await;

        let err = String::from_utf8_lossy(&output.stderr);
        Err(anyhow::anyhow!("Conflict applying {}:\n{}", sha, err))
    }
}

pub async fn revert_commit(repo_path: &Path, sha: &str) -> Result<String> {
    let output = Command::new("git")
        .current_dir(repo_path)
        .args(["revert", "--no-edit", sha])
        .output()
        .await?;

    if output.status.success() {
        Ok(format!("Successfully reverted commit {}", sha))
    } else {
        let _ = Command::new("git")
            .current_dir(repo_path)
            .args(["revert", "--abort"])
            .output()
            .await;

        let err = String::from_utf8_lossy(&output.stderr);
        Err(anyhow::anyhow!("Failed to revert {}:\n{}", sha, err))
    }
}

pub async fn checkout_branch(repo_path: &Path, branch_name_or_sha: &str) -> Result<String> {
    let output = Command::new("git")
        .current_dir(repo_path)
        .args(["checkout", branch_name_or_sha])
        .output()
        .await?;

    if output.status.success() {
        Ok(format!("Successfully checked out {}", branch_name_or_sha))
    } else {
        let err = String::from_utf8_lossy(&output.stderr);
        Err(anyhow::anyhow!("Checkout failed:\n{}", err))
    }
}
