# julesctl Roadmap & Architectural Manifesto

## Core Philosophy & Manifesto
`julesctl` is NOT a simple terminal chatbot. It completely abandons the traditional conversational AI model. Instead, it adopts a strict **Git-first Workflow paradigm**.

*We are building a highly polished, professional developer tool acting as a visual orchestrator to map, branch, merge, cherry-pick, and track AI-generated code across multiple parallel sessions.*

This document serves as the absolute, comprehensive source of truth for developing the `julesctl` application. Every feature, interaction, and architectural decision listed here must be implemented.

### The Paradigm Shift & Core Workflow
1. **Death of "Single Mode":** There is absolutely no "single session" or "manual chat" mode in the traditional sense. Everything revolves around a **Workflow**. A single local repository directory can host multiple independent Workflows simultaneously. The rigid `config.toml` structure that restricted 1 workflow per directory is deprecated.
2. **Jules Sessions are Git Branches:** Every AI session is simply a Git branch. `julesctl` tracks the origin, the divergence, and the patches.
   - *Note on "Ghost Commits":* Unlike other CLI tools (like Codex CLI) that use local hidden stashes/ghost commits to protect user state, `julesctl`'s native AI branch isolation makes this unnecessary. The AI commits to its own remote branch (e.g., `jules/task-...` 🦑), inherently protecting local uncommitted work.
3. **Infinite Parallelism & User Initiative:** Orchestration is entirely manual and at the user's complete discretion. There are no forced automated paths.
   - *Example Use Case:* A user *could* start one initial "Planner" Jules session, and from that session's branch, manually spawn 10 parallel "Worker" Jules sessions, each assigned a different module. However, this is just an example; users have absolute freedom to branch, checkout, and create sessions wherever and however they want.
4. **Patch Picking vs Copy-Pasting:** Instead of copy-pasting code from an AI chat, users visually inspect AI commits within the Git graph and seamlessly cherry-pick, merge, or rebase them into their local working branch.

---

## TUI Architecture & Layout

The TUI must be native, fast, feature-rich regarding keyboard/mouse interactions, and fully **Responsive** for mobile/Android (Termux) environments.

### 1. Responsive & Touch-First Design
- **Adaptive Layout:** The UI automatically adapts based on screen width: **Side-by-side (Horizontal)** panes on Desktop PC, and **Stacked (Vertical)** panes on narrow screens (Android/Termux).
- **Touch-First Interactions:** Every interactive element (Tabs, List items, Action buttons) must natively support Ratatui Mouse events. This ensures seamless tapping on mobile touchscreens and clicking on desktop mice.

### 2. Top-Level Tabs (Web Browser Style)
- The very top of the screen features a tabbed interface representing active Workflows.
- Users can seamlessly switch between entirely different Workflows (even across different projects) instantly, akin to switching tabs in a web browser. The old, useless "Title Pane" is removed.

### 3. The 2-Pane Dashboard Layout (No 3rd Pane Clutter)
To avoid cramping the screen (especially critical for Termux/mobile), we strictly use 2 Panes. Complex views like Chat open as full-screen overlays/modals over the dashboard.

#### Pane 1 (Viewer/Navigator - Left/Top):
This pane contains internal toggles to switch between different data scopes and visual representations.
- **Scope Toggle:**
  - **Workflow-scoped:** Shows only the AI sessions, branches, and commits created specifically for the currently active Workflow.
  - **Global-scoped:** Shows the entire repository's raw git data (all branches, all commits).
- **View Toggle:**
  - **Branch View:** A hierarchical, folder-like structure. We will specifically implement the `tui-tree-widget` library here to display the nested relationships of branches cleanly and natively.
  - **Commit Graph View:** A custom-coded native ASCII git graph parsing implementation (similar to `git log --graph`).
    - **The Working Branch Line:** In this graph, the **leftmost continuous vertical line** strictly represents the active local working branch (the stable integration ground).
    - **Emoji Graph Nodes:** The standard Git `*` nodes in the graph are explicitly replaced by identity emojis for instant visual recognition:
      - 🐱 (Remote/GitHub commits)
      - 🦑 (Jules AI session branches)
      - 💻 (Local branches/commits)

#### Pane 2 (Details/Patch Preview - Right/Bottom):
- Dynamically updates based on Pane 1's selection.
- Displays the exact patch/diff preview, commit message, or branch details.
- *Implementation Detail:* We will use the `diffy` crate for in-memory, blazing-fast red/green diff generation when previewing API patches, avoiding heavy, slow disk-based Git commands for UI previews.

---

## The Scoped Chat Interface & `cli-chat-rs` Decoupling Manifesto

A critical, non-negotiable architectural rule is the absolute decoupling of the `cli-chat-rs` component from `julesctl`.

### 1. The Agnostic UI Framework (`cli-chat-rs`)
- `cli-chat-rs` is a standalone, generic Ratatui messaging UI library.
- It knows absolutely nothing about "Jules API", "Git", "Branches", or "Workflows".
- It must remain generic enough to be plugged into WhatsApp, Telegram, Discord, or any other CLI messaging tool.

### 2. The `JulesAdapter` Integration
- `julesctl` will implement a specific `JulesAdapter` that translates Jules API `Activity` payloads into the generic `cli-chat-rs` message types.

### 3. Scoped Chat Access & Limitations
- **Launch Point:** The Chat Interface (`cli-chat-rs`) is explicitly launched **ONLY from the Branch View** when a Jules branch (🦑) is highlighted.
- **Overlay Rendering:** It opens as a full-screen overlay/modal over the dashboard (not as a cramped 3rd pane).
- **Performance Limit:** It only loads and displays the **last 7 messages** by default to ensure maximum performance and UI responsiveness.

### 4. Chat Layout Rules (Codex-rs Inspired)
Within the `cli-chat-rs` UI, messages must be formatted based on their source/type:
- **AI Messages:** Left-aligned message bubbles.
- **User Messages:** Right-aligned message bubbles.
- **System/Action Logs:** Centered, full-width blocks (e.g., "Terminal outputs", "File modifications", status updates).
- **Special Exception (Jules Plans):** When the Jules API sends a "Plan/Todo list" activity, the adapter will parse it, and `cli-chat-rs` will render it as a structured **Tree/List layout**, distinct from normal text bubbles.

---

## Professional Keybindings & Navigation Standards

To ensure a seamless, native terminal experience, the following universal keybinding map is strictly enforced:

### Navigational Fallbacks
- Native **Vim movement keys** (`j` down, `k` up, `h` collapse left, `l` expand right, `g` top, `G` bottom) are fully supported across all lists and scrollable components.
- Standard **Arrow Keys** remain universally active for users unfamiliar with Vim.

### Action Keybindings (First-Letter Mnemonic)
- **`Tab`**: Switch between active top-level Workflows.
- **`v`**: View toggle (Switch Left Pane between Branch Tree and Commit Graph).
- **`s`**: Scope toggle (Switch Left Pane between Workflow-only and Global repo data).
- **`c` or `Enter`**: Open the Chat (`cli-chat-rs`) overlay for the selected Jules branch.
- **`a`**: Apply / Cherry-pick the currently previewed patch/commit into the active working branch.
- **`r`**: Revert / Undo the selected commit or patch.
- **`n`**: Initialize a **New Session** from the currently highlighted branch.
- **`b`**: Enter **Read-Only Observer Mode** (Inspect a branch's commits/files without checking out or altering the local working environment).
- **`e`**: Open the currently previewed patch or payload in the external `$EDITOR` (e.g., vim/nano) for manual review/modification.
- **`q` or `Esc`**: Close modal/overlay or quit the application.
- **`/`**: Open the fuzzy finder search bar.

---

## Core Mechanisms, Safety, and Synchronization

### 1. New Session Initialization Flow (`n`)
- Pressing `n` on a highlighted branch triggers a specific TUI modal flow.
- The UI prompts the user for the session goal (e.g., "Fix login bug").
- `julesctl` gathers the required `create_session` parameters:
  - `source_context`: Safely formatted as a remote GitHub URL (e.g., `github.com/owner/repo`) to prevent `HTTP 400 Bad Request` errors.
  - `source_branch`: The branch highlighted by the user in the UI.
  - `manager_prompt`: The user's goal input + dynamically injected rules from `.gsd/context.md`, `AGENTS.md`, and global `~/.config/julesctl/rules/`.
- The API is called, and Jules spins up a new remote branch (`jules/task-...`).

### 2. Strict Branch Protection (Auto-Checkout)
- **Rule:** Direct commits by humans to a Jules AI branch (🦑) are strictly forbidden, as it breaks the AI's ability to pull/sync cleanly.
- **Mechanism:** If a user selects a Jules branch and attempts to apply local patches or develop on it, `julesctl` must instantly and automatically prompt/create a new local working branch stemming from that commit.

### 3. Dynamic Auto-Sync and Polling Mechanisms
Because Jules AI operates remotely and asynchronously, `julesctl` must keep the UI updated via two distinct synchronization channels:

1. **Jules API Refresh (Message Polling):**
   - **What it does:** Fetches lightweight JSON `Activity` logs from the Jules API. It checks if the AI has sent a new message, finished thinking, or generated a plan. It does *not* download large code files.
   - **UI Mechanism:** A customizable countdown button (e.g., `[ Refresh Jules (59s) ]`). The countdown ticks dynamically (`58s`, `57s`...). When it hits 0, it auto-refreshes the chat logs.
   - **Manual Override:** The user can click the button or trigger the sync key at any time (e.g., at `30s` remaining) to force an immediate API refresh.

2. **Git Sync (Code Fetching):**
   - **What it does:** Executes heavy `git fetch origin` or `git pull` commands. When the API indicates Jules has pushed new code, this sync actually downloads the commits (🦑 nodes) so they appear in the local TUI graph for cherry-picking.
   - **UI Mechanism:** A customizable countdown button (e.g., `[ Git Sync (5m) ]`). Similar to the API refresh, it ticks down and auto-fetches, but can be manually triggered at any point.

*Both timers can be configured by the user (or set to "Manual Only" to disable auto-polling) depending on network/API rate limits.*
