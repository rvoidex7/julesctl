use crate::api::JulesClient;
use crate::config::RepoConfig;
use crate::git::{ensure_orchestrator_branch, run_hook};
use crate::patch::{apply_patch, commit, fetch_patch, ApplyResult, EntryStatus, PatchQueue};
use anyhow::Result;
use colored::Colorize;
use std::path::PathBuf;
use tokio::time::{interval, Duration};

pub async fn run_manual(client: JulesClient, repo: &RepoConfig, poll_secs: u64) -> Result<()> {
    let repo_path = PathBuf::from(&repo.path);

    if repo.manual_sessions.is_empty() {
        anyhow::bail!(
            "No sessions configured for manual mode.\n\
             Add sessions with: julesctl session add <session_id> <label>"
        );
    }

    ensure_orchestrator_branch(&repo_path)?;

    // Sort sessions by queue_position
    let mut sessions = repo.manual_sessions.clone();
    sessions.sort_by_key(|s| s.queue_position);

    println!(
        "\n{} {}",
        "julesctl".cyan().bold(),
        "manual multi-session mode".dimmed()
    );
    println!("  project  : {}", repo.display_name.yellow());
    println!("  sessions : {}", sessions.len());
    for s in &sessions {
        println!(
            "    [{}] {} ({})",
            s.queue_position,
            s.label.yellow(),
            s.session_id.dimmed()
        );
    }
    println!("{}", "─".repeat(60).dimmed());

    // Build patch queue in configured order
    let mut queue = PatchQueue::new();
    for s in &sessions {
        queue.push(&s.session_id, &s.label);
    }

    // Designate first session as conflict resolver
    let resolver_session_id = sessions[0].session_id.clone();

    println!(
        "\n{} Watching all sessions (Ctrl-C to quit)…\n",
        "◉".green()
    );

    let mut ticker = interval(Duration::from_secs(poll_secs));
    ticker.tick().await;

    loop {
        ticker.tick().await;

        // Try to fetch patches for pending queue entries
        let pending: Vec<(String, String)> = queue
            .all()
            .iter()
            .filter(|e| e.status == EntryStatus::Pending)
            .map(|e| (e.session_id.clone(), e.task_label.clone()))
            .collect();

        for (sid, label) in &pending {
            if let Ok(Some(patch)) = fetch_patch(&client, sid).await {
                println!("  {} Patch ready: {}", "↓".cyan(), label.yellow());
                queue.set_patch(sid, patch);
            }
        }

        // Apply next ready patch in queue
        let next = queue
            .all()
            .iter()
            .find(|e| e.status == EntryStatus::Ready)
            .map(|e| {
                (
                    e.session_id.clone(),
                    e.task_label.clone(),
                    e.patch_content.clone(),
                )
            });

        if let Some((sid, label, Some(patch))) = next {
            println!(
                "\n  {} Applying: {} ({})",
                "→".cyan(),
                label.yellow(),
                sid.dimmed()
            );

            match apply_patch(&repo_path, &patch)? {
                ApplyResult::Success => {
                    commit(
                        &repo_path,
                        &format!("julesctl: apply {} ({})", label, &sid[..8.min(sid.len())]),
                    )?;
                    queue.mark_applied(&sid);
                    println!("  {} Applied: {}", "✓".green().bold(), label.yellow());

                    if !repo.post_pull.is_empty() {
                        run_hook(&repo_path, &repo.post_pull)?;
                    }
                }

                ApplyResult::Conflict(conflict) => {
                    println!(
                        "  {} Conflict in {:?} — sending to resolver session {}…",
                        "⚡".yellow(),
                        conflict.conflicting_files,
                        resolver_session_id.dimmed()
                    );
                    queue.mark_conflicted(&sid);

                    // Build file contexts
                    let mut file_contexts = String::new();
                    for file in &conflict.conflicting_files {
                        let fp = repo_path.join(file);
                        if let Ok(content) = std::fs::read_to_string(&fp) {
                            file_contexts.push_str(&format!("--- {file} ---\n{content}\n\n"));
                        }
                    }

                    // Use conflict resolver from patch module
                    match crate::patch::resolve_conflict(
                        &client,
                        &resolver_session_id,
                        &repo_path,
                        &label,
                        &conflict,
                    )
                    .await
                    {
                        Ok(resolved_patch) => {
                            queue.resolve_conflict(&sid, resolved_patch);
                            println!(
                                "  {} Resolution received, will apply next cycle.",
                                "✓".green()
                            );
                        }
                        Err(e) => {
                            eprintln!("  {} Conflict resolution failed: {e}", "✗".red());
                        }
                    }
                }
            }
        } else {
            // Show heartbeat
            print!(
                "\r{} queue: {} pending",
                "·".dimmed(),
                queue.pending_count()
            );
            use std::io::Write;
            let _ = std::io::stdout().flush();
        }
    }
}
