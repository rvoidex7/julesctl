use crate::api::JulesClient;
use crate::config::RepoConfig;
use crate::git::{ensure_orchestrator_branch, run_hook};
use crate::orchestrator::{
    manager_bootstrap_prompt, parse_open_session, parse_reorder_queue, parse_resolve_conflict,
    TaskFile, TaskStatus,
};
use crate::patch::{apply_patch, commit, ApplyResult, PatchQueue};
use anyhow::Result;
use colored::Colorize;
use std::path::PathBuf;
use tokio::time::{interval, Duration};

pub async fn run_orchestrated(client: JulesClient, repo: &RepoConfig, user_goal: &str) -> Result<()> {
    let repo_path = PathBuf::from(&repo.path);
    let task_file_path = repo_path.join(&repo.task_file);

    // Ensure we're on the orchestrator branch
    ensure_orchestrator_branch(&repo_path)?;

    println!("\n{} {}", "julesctl".cyan().bold(), "orchestrated mode".dimmed());
    println!("  project : {}", repo.display_name.yellow());
    println!("  goal    : {}", user_goal.white());
    println!("{}", "─".repeat(60).dimmed());

    // Create or reuse manager session
    let manager_session_id = if !repo.manager_session_id.is_empty() {
        println!("  {} Using existing manager session {}", "→".cyan(), repo.manager_session_id.dimmed());
        repo.manager_session_id.clone()
    } else {
        println!("  {} Creating manager session…", "→".cyan());
        let session = client
            .create_session(
                &manager_bootstrap_prompt(user_goal, &repo.task_file),
                &format!("julesctl manager: {}", &user_goal[..user_goal.len().min(50)]),
                None,
                None,
            )
            .await?;
        let id = session.id().to_string();
        println!("  {} Manager session: {}", "✓".green(), id.dimmed());
        id
    };

    println!("\n{} Waiting for Jules to create task file…\n", "◉".green());

    // Patch queue across all worker sessions
    let mut queue = PatchQueue::new();

    // Poll loop
    let mut ticker = interval(Duration::from_secs(10));
    ticker.tick().await;

    loop {
        ticker.tick().await;

        // Check if task file exists and has new messages
        if task_file_path.exists() {
            match TaskFile::load(&task_file_path) {
                Ok(mut tf) => {
                    process_messages(&client, &mut tf, &task_file_path, &mut queue, repo).await?;
                }
                Err(e) => {
                    eprintln!("{} Task file parse error: {e}", "⚠".yellow());
                }
            }
        }

        // Try to apply next ready patch in queue
        process_queue(&client, &mut queue, &repo_path, &manager_session_id, repo).await?;
    }
}

async fn process_messages(
    client: &JulesClient,
    tf: &mut TaskFile,
    task_file_path: &PathBuf,
    queue: &mut PatchQueue,
    _repo: &RepoConfig,
) -> Result<()> {
    let pending: Vec<_> = tf
        .messages_to_julesctl
        .iter()
        .filter(|m| !m.processed)
        .map(|m| (m.id.clone(), m.msg_type.clone(), m.payload.clone()))
        .collect();

    for (msg_id, msg_type, payload) in pending {
        use crate::orchestrator::MessageType;

        match msg_type {
            MessageType::OpenSession => {
                if let Some((task_id, prompt)) = parse_open_session(&payload) {
                    println!("  {} Opening session for task {}", "→".cyan(), task_id.yellow());
                    match client.create_session(&prompt, &task_id, None, None).await {
                        Ok(session) => {
                            let sid = session.id().to_string();
                            println!("  {} Session {} opened for {}", "✓".green(), sid.dimmed(), task_id.yellow());
                            tf.update_task_session(&task_id, &sid, TaskStatus::Running);
                            queue.push(&sid, &task_id);
                        }
                        Err(e) => eprintln!("{} Failed to create session: {e}", "✗".red()),
                    }
                    tf.mark_processed(&msg_id);
                    tf.save(task_file_path)?;
                }
            }

            MessageType::ResolveConflict => {
                if let Some((session_id, patch)) = parse_resolve_conflict(&payload) {
                    println!("  {} Applying conflict resolution for {}", "→".cyan(), session_id.dimmed());
                    queue.resolve_conflict(&session_id, patch);
                    tf.mark_processed(&msg_id);
                    tf.save(task_file_path)?;
                }
            }

            MessageType::ReorderQueue => {
                if let Some(new_order) = parse_reorder_queue(&payload) {
                    println!("  {} Reordering patch queue", "→".cyan());
                    queue.reorder(&new_order);
                    tf.mark_processed(&msg_id);
                    tf.save(task_file_path)?;
                }
            }
        }
    }

    Ok(())
}

async fn process_queue(
    client: &JulesClient,
    queue: &mut PatchQueue,
    repo_path: &PathBuf,
    manager_session_id: &str,
    repo: &RepoConfig,
) -> Result<()> {
    // First, try to fetch patches for pending entries
    let pending_ids: Vec<(String, String)> = queue
        .all()
        .iter()
        .filter(|e| matches!(e.status, crate::patch::EntryStatus::Pending))
        .map(|e| (e.session_id.clone(), e.task_label.clone()))
        .collect();

    for (session_id, _label) in pending_ids {
        if let Ok(Some(patch)) = client.get_latest_patch(&session_id).await {
            println!("  {} Patch ready for session {}", "↓".cyan(), session_id.dimmed());
            queue.set_patch(&session_id, patch);
        }
    }

    // Apply next ready patch
    let next = queue
        .all()
        .iter()
        .find(|e| e.status == crate::patch::EntryStatus::Ready)
        .map(|e| (e.session_id.clone(), e.task_label.clone(), e.patch_content.clone()));

    if let Some((session_id, task_label, Some(patch_content))) = next {
        println!(
            "\n  {} Applying patch for {} ({})…",
            "→".cyan(),
            task_label.yellow(),
            session_id.dimmed()
        );

        match apply_patch(repo_path, &patch_content)? {
            ApplyResult::Success => {
                commit(
                    repo_path,
                    &format!("julesctl: apply {} ({})", task_label, &session_id[..8.min(session_id.len())]),
                )?;
                queue.mark_applied(&session_id);
                println!("  {} Applied: {}", "✓".green().bold(), task_label.yellow());

                if !repo.post_pull.is_empty() {
                    run_hook(repo_path, &repo.post_pull)?;
                }
            }

            ApplyResult::Conflict(conflict) => {
                println!(
                    "  {} Conflict in {:?} — sending to manager…",
                    "⚡".yellow(),
                    conflict.conflicting_files
                );
                queue.mark_conflicted(&session_id);

                // Build file contexts
                let mut file_contexts = String::new();
                for file in &conflict.conflicting_files {
                    let full_path = repo_path.join(file);
                    if let Ok(content) = std::fs::read_to_string(&full_path) {
                        file_contexts.push_str(&format!(
                            "--- Current content of {file} ---\n{content}\n\n"
                        ));
                    }
                }

                let prompt = crate::orchestrator::conflict_resolution_prompt(
                    &task_label,
                    &session_id,
                    &conflict.conflicting_files,
                    &file_contexts,
                    &conflict.patch_content,
                );

                client.send_message(manager_session_id, &prompt).await?;
                println!(
                    "  {} Conflict sent to manager. Waiting for resolution…",
                    "◉".yellow()
                );
                // Resolution will come via messages_to_julesctl in next poll cycle
            }
        }
    }

    Ok(())
}
