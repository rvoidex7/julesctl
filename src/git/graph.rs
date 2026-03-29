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
        // Optimization: Use iterators directly to avoid allocating a large Vec of Strings per line.
        let mut parts = line.split('\x00');

        let graph_and_sha = match parts.next() {
            Some(s) => s,
            None => continue,
        };

        let short_sha = parts.next().unwrap_or_default();
        let title = parts.next().unwrap_or_default();
        let refs_str = parts.next().unwrap_or_default();

        // If we didn't extract the basic fields, it's just an ASCII branch connecting line
        if short_sha.is_empty() && title.is_empty() {
            commits.push(GitCommit {
                sha: String::new(),
                short_sha: String::new(),
                title: String::new(),
                branch_type: BranchType::Local,
                graph_prefix: line.to_string(), // Keep the art
            });
            continue;
        }

        // Separate the graph from the 40 character SHA natively
        let (graph_prefix, sha) = if graph_and_sha.len() >= 40 {
            let split_idx = graph_and_sha.len() - 40;
            (&graph_and_sha[..split_idx], &graph_and_sha[split_idx..])
        } else {
            ("", graph_and_sha)
        };

        // Parse refs to determine branch type efficiently (e.g., "HEAD -> main, origin/main")
        let mut branch_type = BranchType::Local;

        if !refs_str.is_empty() {
            for r in refs_str.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()) {
                if r.contains("origin/jules/task-") || r.contains("jules/task-") {
                    let mut session_parts = r.split("task-");
                    let _ = session_parts.next(); // Skip prefix
                    if let Some(id_part) = session_parts.next() {
                        let id = id_part.split(&[' ', ','][..]).next().unwrap_or("").to_string();
                        branch_type = BranchType::JulesSession(id);
                        break; // Jules branch takes visual precedence
                    }
                } else if r.contains("origin/") {
                    branch_type = BranchType::RemoteMain;
                }
            }
        }

        commits.push(GitCommit {
            sha: sha.to_string(),
            short_sha: short_sha.to_string(),
            title: title.to_string(),
            branch_type,
            graph_prefix: graph_prefix.to_string(),
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

pub enum GitActionOutcome {
    Success(String),
    Conflict(String), // Returns the standard error string (conflict details) without aborting
}

pub async fn apply_cherry_pick(repo_path: &Path, sha: &str) -> Result<GitActionOutcome> {
    let output = Command::new("git")
        .current_dir(repo_path)
        .args(["cherry-pick", sha])
        .output()
        .await?;

    if output.status.success() {
        Ok(GitActionOutcome::Success(format!("Successfully applied commit {}", sha)))
    } else {
        // We no longer blindly abort. We enter conflict resolution state (Task 22).
        let err = String::from_utf8_lossy(&output.stdout); // Git often puts conflict data in stdout
        Ok(GitActionOutcome::Conflict(format!("Conflict applying {}:\n{}", sha, err)))
    }
}

pub async fn revert_commit(repo_path: &Path, sha: &str) -> Result<GitActionOutcome> {
    let output = Command::new("git")
        .current_dir(repo_path)
        .args(["revert", "--no-edit", sha])
        .output()
        .await?;

    if output.status.success() {
        Ok(GitActionOutcome::Success(format!("Successfully reverted commit {}", sha)))
    } else {
        let err = String::from_utf8_lossy(&output.stdout);
        Ok(GitActionOutcome::Conflict(format!("Failed to revert {}:\n{}", sha, err)))
    }
}

pub async fn abort_merge_or_cherry_pick(repo_path: &Path) -> Result<()> {
    // Determine if we are in a cherry-pick or revert or merge, and abort safely.
    // Easiest blunt force method: try both.
    let _ = Command::new("git").current_dir(repo_path).args(["cherry-pick", "--abort"]).output().await;
    let _ = Command::new("git").current_dir(repo_path).args(["revert", "--abort"]).output().await;
    let _ = Command::new("git").current_dir(repo_path).args(["merge", "--abort"]).output().await;
    Ok(())
}

/// Enables Git `rerere` (Reuse Recorded Resolution) cache for Tier 3 conflict resolutions.
pub async fn enable_git_rerere(repo_path: &Path) -> Result<()> {
    Command::new("git")
        .current_dir(repo_path)
        .args(["config", "rerere.enabled", "true"])
        .output()
        .await?;
    Ok(())
}

/// Magic Wand Placeholder: Leverages `diffy` in the background for non-overlapping chunk
/// merges that Git's default text merge fails on. Called before returning `GitActionOutcome::Conflict`.
#[allow(dead_code)]
pub async fn auto_merge_non_conflicting_chunks(_repo_path: &Path) -> Result<bool> {
    // TODO: Task 23 Magic Wand integration.
    // 1. Parse conflict markers (<<<<<<< HEAD)
    // 2. Identify disjoint hunks
    // 3. Diffy merge patch safely
    // 4. Return true if safely magically resolved
    Ok(false)
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
