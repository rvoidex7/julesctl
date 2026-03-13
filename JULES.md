# JULES.md — julesctl Implementation Specification

> This document is the authoritative implementation guide for `julesctl`,
> a Rust CLI tool that orchestrates one or multiple Jules AI coding agent
> sessions across one or multiple repositories.
>
> Jules: read this document fully before writing any code.

---

## Project Overview

**Name:** `julesctl`
**Language:** Rust (2021 edition)
**Purpose:** Replace the Jules web UI for power users who run multiple parallel
Jules sessions and need automated patch application, conflict resolution, and
session orchestration — all from the terminal.

**Core philosophy:**
- Jules does the coding. `julesctl` does the orchestration.
- The user should never touch a file manually due to merge conflicts.
- The user should never switch git branches manually.
- Everything Jules does is consumed via Jules API (artifacts/patch endpoint),
  never via `git checkout` to Jules-owned branches.

---

## Repository Structure

```
julesctl/
├── src/
│   ├── main.rs              — CLI entrypoint, clap commands, mode dispatch
│   ├── api/
│   │   ├── mod.rs           — Jules API client (reqwest)
│   │   ├── sessions.rs      — Session CRUD, list, send message
│   │   ├── activities.rs    — Activity polling, incremental fetch
│   │   └── artifacts.rs     — Patch/artifact fetch from sessions
│   ├── modes/
│   │   ├── mod.rs           — Mode trait and dispatch
│   │   ├── single.rs        — Mode 1: Single session workflow
│   │   ├── orchestrated.rs  — Mode 2: Manager session orchestration
│   │   └── manual.rs        — Mode 3: Manual multi-session
│   ├── patch/
│   │   ├── mod.rs           — Patch apply pipeline
│   │   ├── fetch.rs         — Fetch patch from Jules artifacts API
│   │   ├── apply.rs         — Apply patch via `git apply`
│   │   ├── conflict.rs      — Conflict detection and resolution
│   │   └── queue.rs         — FIFO patch queue across sessions
│   ├── git/
│   │   ├── mod.rs           — Git operations
│   │   ├── branch.rs        — Branch management (local orchestrator branch)
│   │   └── pull.rs          — git pull, fetch, status
│   ├── config/
│   │   ├── mod.rs           — Config loading
│   │   └── schema.rs        — Config structs (TOML)
│   ├── orchestrator/
│   │   ├── mod.rs           — Orchestrator core loop
│   │   ├── manager.rs       — Manager session communication protocol
│   │   ├── task_file.rs     — .julesctl-tasks.json read/write/watch
│   │   └── prompts.rs       — Built-in prompt templates (manager bootstrap)
│   ├── poller.rs            — Generic Jules API polling loop
│   ├── display.rs           — Terminal output (colored, no full TUI)
│   └── adapter/
│       └── cli_chat_rs.rs   — cli-chat-rs MessagingAdapter blueprint
├── JULES.md                 — This file
├── README.md                — User-facing documentation
├── Cargo.toml
└── .gitignore
```

---

## Configuration

**Location:** `~/.config/julesctl/config.toml`

**Schema:**

```toml
api_key = "YOUR_JULES_API_KEY"

# Optional: used for conflict resolution (sends conflict to manager session)
# No external LLM needed — conflicts go to the Jules manager session.

[[repos]]
path         = "/home/user/projects/my-app"   # absolute path to repo root
display_name = "My App"                        # shown in terminal header
mode         = "single"                        # "single" | "orchestrated" | "manual"
post_pull    = "cargo build"                   # optional: run after patch apply

# Mode 1 (single) fields:
single_session_id = "14550388554331055113"

# Mode 2 (orchestrated) fields:
manager_session_id = ""          # filled automatically after manager session is created
task_file = ".julesctl-tasks.json"  # file Jules manager writes tasks to

# Mode 3 (manual) fields:
# sessions added dynamically via CLI: julesctl session add <session_id> <name>
```

---

## Operating Modes

### Mode 1 — Single Session

**What it does:**
- Polls Jules session for new activities
- When Jules pushes new commits → fetch patch from artifacts API → `git apply` to local orchestrator branch
- Runs `post_pull` script after successful apply
- Allows sending messages to Jules from CLI

**Local git state:**
- User stays on `jules-orchestrator` branch (created automatically if not exists)
- Never touches Jules-owned remote branch
- Patches applied directly via `git apply`

**CLI:**
```
julesctl watch                    # start polling loop
julesctl send "your message"      # send prompt to active session
julesctl status                   # show last N activities
```

---

### Mode 2 — Orchestrated (Manager Session)

**What it does:**
- Creates one Jules manager session automatically
- Sends a carefully crafted bootstrap prompt (see Manager Bootstrap Prompt section)
- Watches `.julesctl-tasks.json` in the repo root for new task entries written by manager
- When manager writes a new task → `julesctl` opens that Jules session automatically
- Each worker session produces patches → queued → applied in order to `jules-orchestrator` branch
- Conflicts → sent back to manager session for resolution → new patch applied

**Local git state:**
- Single local branch: `jules-orchestrator`
- All patches from all sessions applied here in FIFO order
- No branch switching ever

**CLI:**
```
julesctl orchestrate start "Build a user authentication system with login, register, and profile pages"
julesctl orchestrate status       # show all sessions and their states
julesctl orchestrate logs         # show full activity stream across all sessions
```

**Task file format (`.julesctl-tasks.json`):**
Jules manager session must write this file in this exact format:

```json
{
  "schema_version": "1",
  "tasks": [
    {
      "id": "task-001",
      "title": "Implement login screen",
      "description": "Create LoginScreen.kt with email/password fields and validation",
      "status": "pending",
      "session_id": "",
      "depends_on": []
    },
    {
      "id": "task-002",
      "title": "Implement register screen",
      "description": "Create RegisterScreen.kt with full form validation",
      "status": "pending",
      "session_id": "",
      "depends_on": []
    },
    {
      "id": "task-003",
      "title": "Implement profile page",
      "description": "Create ProfileScreen.kt, depends on login being complete",
      "status": "pending",
      "session_id": "",
      "depends_on": ["task-001"]
    }
  ],
  "conflict_resolution_requests": [],
  "messages_to_julesctl": []
}
```

**`messages_to_julesctl` protocol:**
Manager session communicates with `julesctl` by appending to this array:

```json
{
  "messages_to_julesctl": [
    {
      "id": "msg-001",
      "type": "open_session",
      "payload": {
        "task_id": "task-001",
        "prompt": "Implement login screen as described in task-001"
      }
    },
    {
      "id": "msg-002",
      "type": "resolve_conflict",
      "payload": {
        "affected_task_ids": ["task-001", "task-002"],
        "resolution_patch": "--- a/src/LoginScreen.kt\n+++ ..."
      }
    },
    {
      "id": "msg-003",
      "type": "reorder_queue",
      "payload": {
        "new_order": ["task-003", "task-001", "task-002"]
      }
    }
  ]
}
```

`julesctl` polls this file every 10 seconds. When a new message appears:
- `open_session` → create Jules session with given prompt, update `session_id` in task entry
- `resolve_conflict` → apply the resolution patch, continue queue
- `reorder_queue` → reorder the patch application queue

---

### Mode 3 — Manual Multi-Session

**What it does:**
- User adds sessions manually via CLI
- Each session is tracked independently
- Patches from all sessions queued in order user defined
- Conflicts → sent to a designated resolver session (user picks which one)

**CLI:**
```
julesctl session add <session_id> <name>
julesctl session list
julesctl session remove <session_id>
julesctl run                          # start polling all sessions, apply patches in queue order
julesctl queue reorder                # interactive reorder of patch queue
julesctl queue status                 # show pending patches
```

**Local git state:**
- Local branch: `jules-orchestrator` (same as other modes)
- Created automatically on first run

---

## Patch System

### Fetching Patches

Patches are fetched exclusively from the Jules artifacts API endpoint:

```
GET /v1alpha/sessions/{sessionId}/artifacts
```

Expected response contains git patch format strings. Each artifact represents
a set of file changes Jules has made.

**Do not use:**
- `git fetch origin jules/task-*`
- `git merge`
- `git checkout` to Jules branches

### Applying Patches

```rust
// Pseudocode for patch application pipeline
fn apply_patch(repo_path: &Path, patch: &str) -> PatchResult {
    // 1. Write patch to temp file
    // 2. Run: git apply --check <tempfile>  (dry run first)
    // 3. If dry run passes: git apply <tempfile>
    // 4. If dry run fails: conflict detected → route to conflict resolver
    // 5. Cleanup temp file
}
```

`git apply --check` runs a dry-run without modifying files. If it fails,
we know exactly which files conflict before touching anything.

### Conflict Resolution

**Never ask the user.** Conflicts are always resolved automatically.

**Resolution flow:**

```
patch apply fails (conflict detected)
  → extract conflicting file paths from git apply error output
  → read current file content from disk
  → read conflicting patch hunks
  → send to Jules manager session (Mode 2) or designated resolver session (Mode 3):

    "JULESCTL_CONFLICT_RESOLUTION_REQUEST
     Task: {task_id}
     Conflicting file: {filepath}
     Current file content:
     {file_content}
     
     Patch that failed to apply:
     {patch_content}
     
     Please produce a new complete patch that incorporates both the current
     file state and the intended changes from the failed patch.
     Respond ONLY with a valid git patch in unified diff format, nothing else."

  → wait for Jules response
  → parse patch from response
  → apply resolved patch
  → continue queue
```

**Important:** The manager session has full repo context and sees both sides
of the conflict. This is sufficient for Jules to produce a correct resolution.

---

## Manager Bootstrap Prompt

When Mode 2 starts, `julesctl` sends this prompt to the manager session.
This prompt is hardcoded in `src/orchestrator/prompts.rs`.

```
You are the orchestration manager for a julesctl multi-session workflow.

Your job:
1. Analyze the user's goal below
2. Break it into independent parallel tasks where possible
3. Write the task list to .julesctl-tasks.json in the repo root (exact schema below)
4. Communicate with julesctl by appending to messages_to_julesctl array in that file
5. When julesctl reports a conflict, provide a resolution patch

Communication rules:
- julesctl polls .julesctl-tasks.json every 10 seconds
- To open a new worker session: append open_session message to messages_to_julesctl
- To resolve a conflict: append resolve_conflict message with a valid git patch
- To reorder patch queue: append reorder_queue message
- After julesctl processes a message, it sets processed: true on that message
- Do NOT delete processed messages, only add new ones

Task file schema:
{TASK_FILE_SCHEMA}

Message types:
{MESSAGE_TYPES_SCHEMA}

User goal:
{USER_GOAL}

Start by writing .julesctl-tasks.json with your task breakdown.
Do not write any code yourself — delegate all coding to worker sessions via open_session messages.
```

The `{TASK_FILE_SCHEMA}`, `{MESSAGE_TYPES_SCHEMA}`, and `{USER_GOAL}` placeholders
are filled at runtime by `julesctl`.

---

## Jules API Reference

**Base URL:** `https://jules.googleapis.com/v1alpha`
**Auth:** `?key=API_KEY` query param on every request

### Endpoints Used

| Method | Endpoint | Purpose |
|--------|----------|---------|
| GET | `/sessions/{id}/activities` | Poll for new messages/events |
| POST | `/sessions/{id}:sendMessage` | Send prompt to session |
| GET | `/sessions/{id}/artifacts` | Fetch patches/file changes |
| POST | `/sessions` | Create new session (Mode 2/3) |
| GET | `/sessions` | List all sessions |

### Activity Types

```rust
pub enum ActivityKind {
    Message { author: String, text: String },   // "USER" or "JULES"
    Plan { description: String, status: String },
    GitHubPush { branch: String, commit_sha: String },
}
```

### Known Limitations

- No streaming/webhooks — polling only (default: 30s interval)
- No `cancelSession` or `deleteSession` in API — use web UI
- Jules cannot `git pull` from within a session if local commits exist on same branch
- Max `pageSize` for activities: 100
- No cross-session communication in Jules API — julesctl handles this via task file

---

## Local Git Branch Management

### Orchestrator Branch

On first run in any mode, `julesctl` creates a local branch:

```bash
git checkout -b jules-orchestrator
```

If branch already exists:
```bash
git checkout jules-orchestrator
```

All patches are applied to this branch. User reviews here, then merges to main
when satisfied.

### Branch Naming Convention

Jules creates its own remote branches (e.g. `jules/task-14550388554331055113`).
`julesctl` never checks out these branches. It fetches patches via API only.

---

## cli-chat-rs Integration

`src/adapter/cli_chat_rs.rs` contains a `MessagingAdapter` implementation
which interacts with the `cli-chat-rs` workspace member. The `cli-chat-rs` framework is now a generic `ratatui` based framework with mouse support that is decoupled from any Jules specific implementation.

| cli-chat-rs concept | julesctl concept |
|--------------------|-----------------|
| Chat / Room | Jules Session |
| Message | Activity (message type) |
| send_message() | POST :sendMessage |
| get_messages() | GET activities (filtered to message type) |
| Sidebar entry | Session with display_name |

When opening the TUI, it properly detects if the current directory is bound to project-specific sessions (single session, orchestrated manager, or multiple manual sessions). If so, it filters the sidebar to *only* show those relevant sessions instead of all globally fetched Jules API sessions.

The TUI polling loop and the patch application loop run as separate tokio tasks.

---

## Cargo.toml Dependencies

```toml
[dependencies]
tokio = { version = "1", features = ["full"] }
reqwest = { version = "0.12", features = ["json", "rustls-tls"], default-features = false }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
toml = "0.8"
anyhow = "1"
thiserror = "1"
clap = { version = "4", features = ["derive"] }
colored = "2"
async-trait = "0.1"
dirs = "5"
notify = "6"          # file system watcher for .julesctl-tasks.json
tempfile = "3"        # temp files for git apply
```

---

## Error Handling Philosophy

- All Jules API errors are retried with exponential backoff (max 3 attempts)
- HTTP 429 (rate limit): back off 60 seconds, log warning
- HTTP 401: exit immediately with clear message about API key
- Patch apply failure: route to conflict resolver, never crash
- Task file parse error: log error, skip that entry, continue polling
- Git errors: log full stderr, surface to user, do not silently continue

---

## Implementation Order

Jules should implement in this order:

1. `Cargo.toml` — dependencies
2. `src/config/` — config loading and schema
3. `src/api/` — Jules API client (sessions, activities, artifacts)
4. `src/git/` — git branch and pull operations
5. `src/patch/` — fetch, apply, conflict detection
6. `src/modes/single.rs` — Mode 1 complete
7. `src/poller.rs` — generic polling loop
8. `src/display.rs` — terminal output
9. `src/main.rs` — CLI entrypoint with all commands
10. `src/orchestrator/` — Mode 2 (manager session, task file, protocol)
11. `src/modes/orchestrated.rs` — Mode 2 complete
12. `src/modes/manual.rs` — Mode 3
13. `src/adapter/cli_chat_rs.rs` — TUI adapter blueprint
14. `README.md` — user documentation

---

## What julesctl Does NOT Do

- Does not run any LLM other than Jules
- Does not open a browser or web UI
- Does not modify Jules-owned remote branches
- Does not force-push or rebase remote branches
- Does not create GitHub PRs automatically (user decides when to merge orchestrator branch)
- Does not store API keys anywhere other than config file
