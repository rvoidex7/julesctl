use crate::api::JulesClient;
use crate::config::RepoConfig;
use crate::display;
use crate::git::{current_branch, ensure_orchestrator_branch, head_sha, run_hook};
use crate::patch::{apply_patch, commit, fetch_patch, ApplyResult};
use anyhow::Result;
use colored::Colorize;
use std::path::PathBuf;
use tokio::time::{interval, Duration};

pub async fn run_single(client: JulesClient, repo: &RepoConfig, poll_secs: u64, initial_count: u32) -> Result<()> {
    let repo_path = PathBuf::from(&repo.path);
    let session_id = &repo.single_session_id;

    if session_id.is_empty() {
        anyhow::bail!("single_session_id is not set in config for this repo");
    }

    // Ensure orchestrator branch
    ensure_orchestrator_branch(&repo_path)?;

    println!("\n{} {}", "julesctl".cyan().bold(), "single mode".dimmed());
    println!("  project  : {}", repo.display_name.yellow());
    println!("  session  : {}", session_id.dimmed());
    println!("  branch   : {}", current_branch(&repo_path).unwrap_or_default().green());
    println!("  sha      : {}", head_sha(&repo_path).unwrap_or_default().dimmed());
    println!("  interval : {}s", poll_secs);
    println!("{}", "─".repeat(60).dimmed());

    // Show recent activities on startup
    println!("\n{} Loading last {} activities…\n", "↓".cyan(), initial_count);
    let activities = client.get_activities(session_id, initial_count).await?;
    display::print_activities(&activities);

    let mut last_seen: Option<String> = activities.last().map(|a| a.name.clone());
    let mut last_patch_sha: Option<String> = None;

    println!("\n{} Watching (Ctrl-C to quit)…\n", "◉".green());

    let mut ticker = interval(Duration::from_secs(poll_secs));
    ticker.tick().await;

    loop {
        ticker.tick().await;

        // Poll new activities
        let new_activities = match &last_seen {
            Some(name) => client.get_activities_after(session_id, name, 20).await,
            None => Ok(vec![]),
        };

        match new_activities {
            Ok(acts) if !acts.is_empty() => {
                println!();
                for activity in &acts {
                    display::print_activity(activity);

                    // If Jules pushed to GitHub, try to fetch patch
                    if activity.github_push.is_some() {
                        try_fetch_and_apply(
                            &client,
                            session_id,
                            &repo_path,
                            repo,
                            &mut last_patch_sha,
                        )
                        .await;
                    }

                    last_seen = Some(activity.name.clone());
                }
            }
            Ok(_) => {
                // No new activities — also try patch fetch in case Jules pushed
                // without a detectable activity (API quirk)
                try_fetch_and_apply(
                    &client,
                    session_id,
                    &repo_path,
                    repo,
                    &mut last_patch_sha,
                )
                .await;

                let ts = utc_time();
                print!("\r{} last checked {ts}", "·".dimmed());
                use std::io::Write;
                let _ = std::io::stdout().flush();
            }
            Err(e) => {
                eprintln!("{} Poll error: {e}", "⚠".yellow());
            }
        }
    }
}

async fn try_fetch_and_apply(
    client: &JulesClient,
    session_id: &str,
    repo_path: &PathBuf,
    repo: &RepoConfig,
    last_patch_sha: &mut Option<String>,
) {
    match fetch_patch(client, session_id).await {
        Ok(Some(patch)) => {
            // Deduplicate: skip if same patch as last time
            let patch_hash = format!("{:x}", md5_simple(&patch));
            if last_patch_sha.as_deref() == Some(&patch_hash) {
                return;
            }

            println!("\n  {} New patch from Jules…", "↓".cyan().bold());

            match apply_patch(repo_path, &patch) {
                Ok(ApplyResult::Success) => {
                    if let Err(e) = commit(repo_path, &format!("julesctl: apply jules/{session_id}")) {
                        eprintln!("{} Commit failed: {e}", "✗".red());
                        return;
                    }
                    *last_patch_sha = Some(patch_hash);
                    println!("  {} Patch applied and committed.", "✓".green().bold());

                    if !repo.post_pull.is_empty() {
                        if let Err(e) = run_hook(repo_path, &repo.post_pull) {
                            eprintln!("{} Hook error: {e}", "✗".red());
                        }
                    }
                }
                Ok(ApplyResult::Conflict(c)) => {
                    eprintln!(
                        "  {} Conflict in {:?} — Mode 1 does not have a resolver session.",
                        "⚡".yellow(),
                        c.conflicting_files
                    );
                    eprintln!("  Consider using orchestrated mode (Mode 2) for conflict resolution.");
                }
                Err(e) => {
                    eprintln!("  {} Patch apply error: {e}", "✗".red());
                }
            }
        }
        Ok(None) => {} // No patch yet
        Err(e) => {
            eprintln!("{} Artifact fetch error: {e}", "⚠".yellow());
        }
    }
}

fn utc_time() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let h = (secs % 86400) / 3600;
    let m = (secs % 3600) / 60;
    let s = secs % 60;
    format!("{h:02}:{m:02}:{s:02} UTC")
}

fn md5_simple(s: &str) -> u64 {
    // Simple non-crypto hash for deduplication
    let mut h: u64 = 14695981039346656037;
    for b in s.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(1099511628211);
    }
    h
}
