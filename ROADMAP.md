# julesctl Roadmap & Architectural Manifesto

## Core Philosophy & Manifesto
`julesctl` is NOT a simple terminal chatbot. It completely abandons the traditional conversational AI model. Instead, it adopts a strict **Git-first Workflow paradigm**.

*We are building a highly polished, professional developer tool acting as a visual orchestrator to map, branch, merge, cherry-pick, and track AI-generated code across multiple parallel sessions.*

### The Paradigm Shift
1. **Death of "Single Mode":** There is no "single session" or "manual chat" mode. Everything revolves around a **Workflow**. A single repository directory can host multiple independent Workflows simultaneously.
2. **Jules Sessions are Git Branches:** Every AI session is simply a Git branch. `julesctl` tracks the origin, the divergence, and the patches.
3. **Infinite Parallelism (User Initiative):** Orchestration is entirely manual and at the user's discretion.

## TUI Architecture & Layout

The TUI must be native, fast, feature-rich keyboard/mouse interactions, and fully **Responsive** for mobile/Android (Termux) environments.

1. **Responsive & Touch-First UI:**
   - The UI automatically adapts: **Side-by-side (Horizontal)** panes on Desktop, and **Stacked (Vertical)** panes on narrow screens (Android/Termux).
   - Every interactive element must natively support Ratatui Mouse events.

2. **The 2-Pane Dashboard Layout:**
   - **Pane 1 (Viewer/Navigator - Left/Top):**
     - Toggle between two distinct views:
       - **Branch View:** A hierarchical folder-like structure. We will implement `tui-tree-widget` here to display the nested relationships of branches cleanly.
       - **Commit View (Custom ASCII):** A custom-coded native git graph parsing implementation. The leftmost continuous vertical line always represents the active local working branch. The standard Git `*` nodes in the graph are explicitly replaced by identity emojis: 🐱 (Remote), 🦑 (Jules), 💻 (Local).
   - **Pane 2 (Details/Patch Preview - Right/Bottom):**
     - Dynamically updates based on Pane 1's selection. Displays patch/diff previews using `diffy`.

3. **Scoped Chat Interface (cli-chat-rs Overlay):**
   - The Chat Interface is explicitly launched **from the Branch View** when a Jules branch (🦑) is highlighted.
   - It opens as a full-screen overlay/modal over the dashboard.
   - It only loads the last 7 messages by default to ensure maximum performance and responsiveness.

## The cli-chat-rs Decoupling Manifesto

A critical architectural rule is the absolute decoupling of `cli-chat-rs` from `julesctl`.

1. **Agnostic UI Framework:** `cli-chat-rs` is a standalone, generic Ratatui messaging UI library. It knows absolutely nothing about "Jules", "Git", or "Workflows". It must remain generic enough to be plugged into WhatsApp, Telegram, or any other CLI messaging tool.
2. **Julesctl Adapter:** `julesctl` will implement a specific `JulesAdapter` that translates Jules API `Activity` payloads into generic `cli-chat-rs` message types.
3. **Chat Layout Rules (Codex-rs Inspired):**
   - **AI Messages:** Left-aligned message bubbles.
   - **User Messages:** Right-aligned message bubbles.
   - **System/Action Logs:** Centered, full-width blocks (e.g., "Terminal outputs", "File modifications").
   - **Special Exception (Jules Plans):** When Jules sends a "Plan/Todo list" activity, the adapter will parse it and `cli-chat-rs` will render it as a structured Tree/List layout, distinct from normal text bubbles.

## Professional CLI/TUI Enhancements
- **Event-Driven UI (AppEvent Bus):** Centralized `AppEvent` channel for non-blocking UI.
- **Fuzzy Finding:** Integrate `nucleo` and `ignore` for blazing fast Workflow switching.
- **Vim Keybindings & Editor Fallback:** `hjkl`, `g`, `G`, `/` navigation, and an `$EDITOR` fallback for massive patches.
- **Clipboard Support:** `arboard` + OSC 52.

## Critical Mechanisms & Safety Rules
1. **Strict Branch Protection (Auto-Checkout):** Direct commits by humans to a Jules AI branch (🦑) are strictly forbidden. `julesctl` must instantly auto-branch off it.
2. **API Context Safety:** Safely format local absolute paths into `github.com/owner/repo` formats for `source_context` to avoid Jules API HTTP 400 errors.
