use crate::config::rules;
use std::path::Path;

/// Bootstrap prompt sent to the Jules manager session when Mode 2 starts.
/// Placeholders are filled at runtime.
pub fn manager_bootstrap_prompt(user_goal: &str, task_file_path: &str, repo_root: &Path) -> String {
    let template = rules::get_global_manager_prompt().unwrap_or_else(|| {
        r#"You are the orchestration manager for a julesctl multi-session workflow.

Your responsibilities:
1. Analyze the user goal below
2. Break it into independent parallel tasks where possible
3. Write the task breakdown to {task_file_path} in this exact JSON format
4. Communicate with julesctl by appending messages to the messages_to_julesctl array in that file
5. When julesctl reports a conflict, produce a resolution patch

## Task file format ({task_file_path}):

```json
{
  "schema_version": "1",
  "tasks": [
    {
      "id": "task-001",
      "title": "Short task title",
      "description": "Full description of what this task should implement",
      "status": "pending",
      "session_id": "",
      "depends_on": []
    }
  ],
  "messages_to_julesctl": []
}
```

## Communication protocol (messages_to_julesctl):

To open a new worker session for a task:
```json
{
  "id": "msg-001",
  "type": "open_session",
  "processed": false,
  "payload": {
    "task_id": "task-001",
    "prompt": "Full prompt to send to the worker session"
  }
}
```

To resolve a conflict julesctl reported:
```json
{
  "id": "msg-002",
  "type": "resolve_conflict",
  "processed": false,
  "payload": {
    "affected_session_id": "14550388554331055113",
    "resolution_patch": "diff --git a/src/File.kt b/src/File.kt\n..."
  }
}
```

To reorder the patch application queue:
```json
{
  "id": "msg-003",
  "type": "reorder_queue",
  "processed": false,
  "payload": {
    "new_order": ["session-id-1", "session-id-2", "session-id-3"]
  }
}
```

## Rules:
- julesctl polls {task_file_path} every 10 seconds
- After julesctl processes a message, it sets processed: true - do NOT delete processed messages
- Only append new messages, never modify existing ones
- Do NOT write code yourself - delegate all coding to worker sessions via open_session messages
- Tasks with no depends_on can be opened in parallel immediately
- Tasks with depends_on should wait until dependency tasks are marked "completed" in the task file
- For conflict resolution: julesctl will send you a JULESCTL_CONFLICT_RESOLUTION_REQUEST message - respond with a resolve_conflict message

## User goal:

{user_goal}

Start now: write {task_file_path} with your task breakdown, then send open_session messages for all tasks that have no dependencies."#.to_string()
    });

    let filled = template
        .replace("{task_file_path}", task_file_path)
        .replace("{user_goal}", user_goal);

    rules::build_session_prompt(&filled, Some(repo_root))
}

/// Conflict resolution request prompt sent to manager session.
pub fn conflict_resolution_prompt(
    task_label: &str,
    session_id: &str,
    conflicting_files: &[String],
    file_contexts: &str,
    patch_content: &str,
) -> String {
    format!(
        r#"JULESCTL_CONFLICT_RESOLUTION_REQUEST

Task: {task_label}
Session: {session_id}
Conflicting files: {files}

{file_contexts}

--- Patch that failed to apply ---
{patch_content}

Please produce a new complete patch that incorporates both the current file state
and the intended changes from the failed patch.

Respond by appending a resolve_conflict message to messages_to_julesctl in {task_file} with:
- affected_session_id: "{session_id}"
- resolution_patch: the complete unified diff patch

The patch must start with "diff --git" and be valid input for `git apply`."#,
        task_label = task_label,
        session_id = session_id,
        files = conflicting_files.join(", "),
        file_contexts = file_contexts,
        patch_content = patch_content,
        task_file = ".julesctl-tasks.json",
    )
}
