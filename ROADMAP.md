# julesctl Roadmap & Architectural Manifesto

## Core Philosophy & Manifesto
`julesctl` is NOT a simple terminal chatbot. It completely abandons the traditional conversational AI model. Instead, it adopts a strict **Git-first Workflow paradigm**.

*We are building a highly polished, professional developer tool acting as a visual orchestrator to map, branch, merge, cherry-pick, and track AI-generated code across multiple parallel sessions.*

### The Paradigm Shift
1. **Death of "Single Mode":** There is no "single session" or "manual chat" mode. Everything revolves around a **Workflow**. A single repository directory can host multiple independent Workflows simultaneously.
2. **Jules Sessions are Git Branches:** Every AI session is simply a Git branch. `julesctl` tracks the origin, the divergence, and the patches.
   - *Note on "Ghost Commits": Unlike other CLI tools that use local hidden stashes/ghost commits to protect user state, `julesctl`'s native AI branch isolation makes this unnecessary. The AI commits to its own remote branch (🦑), inherently protecting local uncommitted work.*
3. **Infinite Parallelism:** A user can start one initial "Planner" Jules session. From that session's branch, the user can spawn 10 parallel "Worker" Jules sessions, each assigned a different module.
4. **Patch Picking vs Copy-Pasting:** Instead of copy-pasting code from an AI chat, users visually inspect AI commits and seamlessly cherry-pick or rebase them into their local working branch.

## TUI Architecture & Layout

The TUI must be native, fast, and feature rich keyboard/mouse interactions without feeling bloated (like `gitui`).

1. **Top-Level Tabs (Web Browser Style):**
   - The top of the screen features tabs representing active Workflows.
   - Users can seamlessly switch between entirely different Workflows (even across different projects) instantly.

2. **The 2-Pane Dashboard Layout:**
   - **Left Pane (Viewer/Navigator):**
     - Contains internal tab/toggle switches to alternate between **Branch View** (tree of branches) and **Commit View** (Git graph of commits).
     - Contains a scope toggle to alternate between **Workflow-scoped** (showing only AI sessions/branches created for this specific Workflow) and **Global-scoped** (showing the entire repository's raw git data).
     - Visual emojis strictly enforce identity: 🐱 (Remote/GitHub), 🦑 (Jules AI session branch), 💻 (Local branch/commit).
   - **Right Pane (Details/Patch Preview):**
     - Dynamically updates based on the Left Pane's selection.
     - Displays the exact patch/diff preview, commit message, or branch details.
     - *Implementation Detail:* Use `diffy` for in-memory, blazing-fast red/green diff generation when previewing API patches, avoiding heavy disk-based Git commands for UI previews.

3. **Scoped Chat Interface:**
   - Pressing `C` on a Jules branch/commit strictly opens the decoupled `cli-chat-rs` interface scoped *only* to that specific branch.

## Professional CLI/TUI Enhancements (Inspirations)

To elevate `julesctl` to a top-tier developer tool, we adopt the following technical standards:

1. **Event-Driven UI (AppEvent Bus):**
   - The TUI must not block on API calls. Implement a centralized `AppEvent` channel (e.g., `AppEvent::SubmitPrompt`, `AppEvent::PatchFetched`). Widgets emit events; the main event loop processes them asynchronously and updates the UI state.
2. **Fuzzy Finding & Navigation:**
   - Integrate `nucleo` and `ignore` for lightning-fast, `.gitignore`-aware fuzzy searching when switching Workflows or selecting files to add as context (e.g., `@src/main.rs`).
3. **Native Vim Keybindings & Fallbacks:**
   - Implement native Vim navigation (`hjkl`, `g`, `G`, `/`) across all list and scrollable Ratatui components.
   - Implement an `$EDITOR` fallback (e.g., pressing `e` to open a massive AI-generated patch or configuration file directly in Vim/Nano for editing before applying).
4. **Advanced Clipboard Support:**
   - Use `arboard` coupled with OSC 52 escape sequences to guarantee copy-pasting works flawlessly even over SSH or within WSL environments.

## Critical Mechanisms & Safety Rules

1. **Strict Branch Protection (Auto-Checkout):**
   - **Rule:** Direct commits by humans to a Jules AI branch (🦑) are strictly forbidden (it breaks the AI's ability to pull/sync).
   - **Mechanism:** If a user selects a Jules branch to apply local patches or develop on it, `julesctl` must instantly and automatically create a new local working branch stemming from that commit.

2. **API Context Safety:**
   - The `create_session` API call frequently fails with HTTP 400 Bad Request because it expects a valid remote GitHub URL format (e.g., `github.com/owner/repo`) in the `source_context`.
   - **Mechanism:** The CLI must rigorously format or safely omit local absolute paths before dispatching API requests.

3. **Meta-Prompting Moddability:**
   - `julesctl` will act as a host for external context-engineering tools like Get-Shit-Done (GSD).
   - Initial prompts sent to Jules will automatically append context read from `~/.config/julesctl/rules/`, local `.julesctl/rules.md`, `AGENTS.md`, and `.gsd/context.md`.

## Execution Priorities (For Next Session)
*(Reminder: This current session is strictly for documentation and planning. Do NOT code these features right now.)*

1. **Purge Old Logic:** Remove all legacy references to Single/Manual mode and the rigid `config.toml` structure that restricts 1 workflow per directory.
2. **Implement Top-Level Tabs:** Build the ratatui tabbed interface at the application root.
3. **Refine the Left Pane:** Code the Branch vs Commit and Workflow vs Global toggle views inside the left pane.
4. **Integrate Professional Enhancements:** Set up the `AppEvent` bus and add `nucleo`, `diffy`, and Vim keybindings to the core UI loop.
5. **Fix the 400 Error:** Patch the session creation API logic to ensure safe `source_context` formatting.
