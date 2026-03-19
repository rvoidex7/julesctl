# julesctl Roadmap & Architectural Manifesto

## Core Philosophy & Manifesto
`julesctl` is NOT a simple terminal chatbot. It completely abandons the traditional conversational AI model. Instead, it adopts a strict **Git-first Workflow paradigm**.

*We are building a highly polished, professional developer tool acting as a visual orchestrator to map, branch, merge, cherry-pick, and track AI-generated code across multiple parallel sessions.*

### The Paradigm Shift
1. **Death of "Single Mode":** There is no "single session" or "manual chat" mode. Everything revolves around a **Workflow**. A single repository directory can host multiple independent Workflows simultaneously.
2. **Jules Sessions are Git Branches:** Every AI session is simply a Git branch. `julesctl` tracks the origin, the divergence, and the patches.
3. **Infinite Parallelism (User Initiative):** Orchestration is entirely manual and at the user's discretion.
   - *Example Use Case:* A user *could* start one initial "Planner" Jules session, and from that session's branch, manually spawn 10 parallel "Worker" Jules sessions, each assigned a different module. However, this is just an example; users have absolute freedom to branch, checkout, and create sessions wherever and however they want.
4. **Patch Picking vs Copy-Pasting:** Instead of copy-pasting code from an AI chat, users visually inspect AI commits and seamlessly cherry-pick or rebase them into their local working branch.

## TUI Architecture & Layout

The TUI must be native, fast, feature-rich keyboard/mouse interactions, and fully **Responsive** for mobile/Android (Termux) environments.

1. **Responsive & Touch-First UI:**
   - The UI automatically adapts: **Side-by-side (Horizontal)** panes on Desktop, and **Stacked (Vertical)** panes on narrow screens (Android/Termux).
   - Every interactive element (Tabs, List items, Action buttons) must natively support Ratatui Mouse events to allow seamless tapping on touchscreens and clicking on desktops.

2. **Top-Level Tabs (Web Browser Style):**
   - The top of the screen features tabs representing active Workflows.
   - Users can seamlessly switch between entirely different Workflows (even across different projects) instantly.

3. **The 2-Pane Dashboard Layout (No 3rd Pane Clutter):**
   - To avoid cramping the screen (especially on mobile), we strictly use 2 Panes. Complex views like Chat open as full-screen overlays/modals over the dashboard.
   - **Pane 1 (Viewer/Navigator - Left/Top):**
     - Toggle between **Branch View** (tree of branches) and **Commit View** (Git graph of commits).
     - **The Working Branch Line:** In the Commit Graph, the leftmost continuous vertical line always represents the active local working branch (the stable integration ground).
     - **Emoji Graph Nodes:** The standard Git `*` nodes in the graph are explicitly replaced by identity emojis: 🐱 (Remote/GitHub), 🦑 (Jules AI session branch), 💻 (Local branch/commit).
   - **Pane 2 (Details/Patch Preview - Right/Bottom):**
     - Dynamically updates based on Pane 1's selection.
     - Displays the exact patch/diff preview, commit message, or branch details. Use `diffy` for in-memory red/green diff generation.

4. **Scoped Chat Interface (Overlay):**
   - Pressing `C` (or tapping "Open Chat") on a Jules branch/commit strictly opens the decoupled `cli-chat-rs` interface as a full-screen overlay scoped *only* to that specific branch.

## Professional CLI/TUI Enhancements (Inspirations)

To elevate `julesctl` to a top-tier developer tool, we adopt the following technical standards:

1. **Event-Driven UI (AppEvent Bus):**
   - The TUI must not block on API calls. Implement a centralized `AppEvent` channel (e.g., `AppEvent::SubmitPrompt`, `AppEvent::PatchFetched`).
2. **Fuzzy Finding & Navigation:**
   - Integrate `nucleo` and `ignore` for lightning-fast, `.gitignore`-aware fuzzy searching when switching Workflows or selecting files.
3. **Native Vim Keybindings & Fallbacks:**
   - Implement native Vim navigation (`hjkl`, `g`, `G`, `/`) across all list and scrollable Ratatui components.
   - Implement an `$EDITOR` fallback (e.g., pressing `e` to open a massive AI-generated patch directly in Vim/Nano).
4. **Advanced Clipboard Support:**
   - Use `arboard` coupled with OSC 52 escape sequences to guarantee copy-pasting works flawlessly even over SSH or within WSL/Termux environments.

## Critical Mechanisms & Safety Rules

1. **Strict Branch Protection (Auto-Checkout):**
   - **Rule:** Direct commits by humans to a Jules AI branch (🦑) are strictly forbidden.
   - **Mechanism:** If a user selects a Jules branch to apply local patches or develop on it, `julesctl` must instantly and automatically create a new local working branch stemming from that commit.

2. **API Context Safety:**
   - Safely format local absolute paths into `github.com/owner/repo` formats for the `source_context` to avoid Jules API HTTP 400 errors.

3. **Meta-Prompting Moddability:**
   - Initial prompts sent to Jules will automatically append context read from `~/.config/julesctl/rules/`, local `.julesctl/rules.md`, `AGENTS.md`, and `.gsd/context.md`.
