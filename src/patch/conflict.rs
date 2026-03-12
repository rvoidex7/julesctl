use crate::api::JulesClient;
use crate::patch::apply::ConflictInfo;
use anyhow::Result;
use colored::Colorize;
use std::path::Path;

/// Send conflict info to the resolver session (manager or designated session).
/// Waits for Jules to respond with a resolved patch.
/// Returns the resolved patch content.
pub async fn resolve_conflict(
    client: &JulesClient,
    resolver_session_id: &str,
    repo_path: &Path,
    task_label: &str,
    conflict: &ConflictInfo,
) -> Result<String> {
    // Build context: current content of conflicting files
    let mut file_contexts = String::new();
    for file in &conflict.conflicting_files {
        let full_path = repo_path.join(file);
        if let Ok(content) = std::fs::read_to_string(&full_path) {
            file_contexts.push_str(&format!(
                "\n--- Current content of {file} ---\n{content}\n"
            ));
        }
    }

    let prompt = format!(
        "JULESCTL_CONFLICT_RESOLUTION_REQUEST\n\
         Task: {task_label}\n\
         Conflicting files: {files}\n\
         \n\
         {file_contexts}\n\
         --- Patch that failed to apply ---\n\
         {patch}\n\
         \n\
         Please produce a new complete patch that incorporates both the current \
         file state and the intended changes from the failed patch.\n\
         Respond ONLY with a valid git patch in unified diff format, nothing else.\n\
         Start your response with 'diff --git' and include nothing else.",
        files = conflict.conflicting_files.join(", "),
        patch = conflict.patch_content,
    );

    println!(
        "  {} Sending conflict to resolver session {}…",
        "⚡".yellow(),
        resolver_session_id.dimmed()
    );

    client.send_message(resolver_session_id, &prompt).await?;

    // Poll for Jules response containing a patch
    println!("  {} Waiting for conflict resolution…", "◉".yellow());
    let resolved_patch = poll_for_resolution(client, resolver_session_id).await?;

    println!("  {} Conflict resolved.", "✓".green().bold());
    Ok(resolved_patch)
}

/// Poll the resolver session activities until Jules responds with a patch.
async fn poll_for_resolution(client: &JulesClient, session_id: &str) -> Result<String> {
    let mut last_seen: Option<String> = None;
    let mut attempts = 0;
    const MAX_ATTEMPTS: u32 = 60; // 5 minutes max (5s interval)

    loop {
        attempts += 1;
        if attempts > MAX_ATTEMPTS {
            anyhow::bail!("Timed out waiting for conflict resolution from Jules");
        }

        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

        let activities = match &last_seen {
            Some(name) => client.get_activities_after(session_id, name, 10).await?,
            None => client.get_activities(session_id, 10).await?,
        };

        for activity in &activities {
            last_seen = Some(activity.name.clone());

            if let Some(msg) = &activity.message {
                if msg.author.to_uppercase() == "JULES" {
                    let text = msg.text.trim();
                    // Check if response looks like a git patch
                    if text.contains("diff --git") || text.starts_with("---") {
                        return Ok(extract_patch_from_text(text));
                    }
                }
            }
        }
    }
}

/// Extract the patch portion from Jules' response text.
/// Jules might include explanation text before/after the patch.
fn extract_patch_from_text(text: &str) -> String {
    // Find the start of the patch
    if let Some(start) = text.find("diff --git") {
        return text[start..].to_string();
    }
    if let Some(start) = text.find("--- a/") {
        return text[start..].to_string();
    }
    // Return as-is if we can't find clear boundaries
    text.to_string()
}
