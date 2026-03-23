# Branching, Synchronization & Safety

`julesctl` operates as a Git-first workflow manager. Understanding how sessions initialize, how branches are protected, and how data syncs is crucial for development.

## 1. New Session Initialization Flow (`n`)

*   Pressing `n` on a highlighted branch triggers a specific TUI modal flow.
*   The UI prompts the user for the session goal (e.g., "Fix login bug").
*   `julesctl` gathers the required `create_session` parameters:
    *   `source_context`: Safely formatted as a remote GitHub URL (e.g., `github.com/owner/repo`) to prevent `HTTP 400 Bad Request` errors.
    *   `source_branch`: The branch highlighted by the user in the UI.
    *   `manager_prompt`: The user's goal input + dynamically injected rules from `.gsd/context.md`, `AGENTS.md`, and global `~/.config/julesctl/rules/`.
*   The API is called, and Jules spins up a new remote branch (`jules/task-...`).

## 2. Strict Branch Protection (Auto-Checkout)

*   **Rule:** Direct commits by humans to a Jules AI branch (🦑) are strictly forbidden, as it breaks the AI's ability to pull/sync cleanly.
*   **Mechanism:** If a user selects a Jules branch and attempts to apply local patches or develop on it, `julesctl` must instantly and automatically prompt/create a new local working branch stemming from that commit.

## 3. Contextual Auto-Sync and Polling Mechanisms

To avoid overengineering (like WebHooks) while providing a real-time feel, `julesctl` uses two distinct, context-aware synchronization polling channels:

### 3.1. Jules API Refresh (Message/Activity Polling)

*   **What it does:** Fetches lightweight JSON `Activity` logs from the Jules API to detect new messages, plans, or status updates.
*   **Contextual Execution:** This polling loop is **ONLY ACTIVE when the `cli-chat-rs` overlay is open** for a specific session. There is no reason to poll the API for messages if the user is just looking at the Workflow Git graph.
*   **UI Mechanism:** A customizable countdown button inside the chat UI (e.g., `[ Refresh Chat (15s) ]`). The user can manually click it or press `r` inside the chat to force an instant refresh.

### 3.2. Git Sync (Code/Commit Fetching)

*   **What it does:** Executes a heavy `git fetch origin` command to download actual code commits (🦑 nodes) into the local TUI graph for cherry-picking.
*   **Contextual Execution:** Runs periodically on the main Workflow View (e.g., every 3-5 minutes) or triggered manually.
*   **UI Mechanism:** A countdown button on the main Workflow View (e.g., `[ Git Sync (5m) ]`).
