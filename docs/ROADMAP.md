# julesctl Roadmap & Architectural Manifesto

## Core Philosophy & Manifesto

`julesctl` is NOT a simple terminal chatbot. It completely abandons the traditional conversational AI model. Instead, it adopts a strict **Git-first Workflow paradigm**.

*We are building a highly polished, professional developer tool acting as a visual orchestrator to map, branch, merge, cherry-pick, and track AI-generated code across multiple parallel sessions.*

This document acts as an index to the comprehensive architectural manifesto. Every feature, interaction, and architectural decision listed here must be implemented.

**Explore the modules:**

1.  **Architecture:**
    *   [Core Philosophy](architecture/core-philosophy.md)
    *   [Hybrid Git Engine & Optimizations](architecture/hybrid-git-engine.md)
    *   [Data Management & Ahenk P2P Sync](architecture/data-management-ahenk.md)
2.  **User Interface (TUI):**
    *   [Layout & Navigation](ui/layout-and-navigation.md)
    *   [Scoped Chat Integration & `cli-chat-rs`](ui/scoped-chat-integration.md)
    *   [Settings & Configuration UI](ui/settings-ui.md)
3.  **Git Workflow & Operations:**
    *   [Branching, Synchronization & Safety](git-workflow/branching-and-sync.md)
    *   [The Visual Patch Stack (Catalog Shopping)](git-workflow/visual-patch-stack.md)
4.  **Conflict Resolution:**
    *   [The Conflict Resolution Framework](conflict-resolution/framework.md)
5.  **Universal Local AI Orchestration:**
    *   [Local CLI Encapsulation via Worktrees](architecture/local-orchestration.md)

## Execution Plan & Tasks

Based on the architectural manifesto, the following sequential tasks outline the development roadmap:

- [x] **Task 1:** Setup the project structure, Rust dependencies (Tokio, Reqwest, Clap, Ratatui, gix, diffy, nucleo, ignore, arboard, tui-tree-widget), and decouple the generic `cli-chat-rs` crate as a workspace member.
- [x] **Task 2:** Implement Data Management & Global State storage in `~/.config/julesctl/`, securely storing credentials via OS keyring.
- [x] **Task 3:** Implement Ahenk P2P sync for syncing workflow state and API cache across devices.
- [x] **Task 4:** Develop the core Hybrid Git Engine using `gitoxide` for non-blocking reads and `tokio::process::Command` for writes/network operations.
- [x] **Task 5:** Build the Responsive 2-Pane TUI layout foundation (Side-by-side for PC, Stacked for Termux) with Ratatui, implementing touch-first mouse event support.
- [x] **Task 6:** Implement the Top-Level Tabs (Web Browser Style) for managing multiple independent Workflows simultaneously.
- [x] **Task 7:** Implement the Viewer/Navigator Left Pane: Branch View using `tui-tree-widget`.
- [x] **Task 8:** Implement the Viewer/Navigator Left Pane: Commit Graph View with custom emoji nodes (🐱, 🦑, 💻).
- [x] **Task 9:** Implement the Details/Patch Preview Right Pane using `diffy` for blazing-fast in-memory red/green diff generation.
- [x] **Task 10:** Implement global professional keybindings (Vim movement, `Tab`, `v`, `s`, `a`, `r`, `n`, `b`, `e`, `q`, `/`).
- [x] **Task 11:** Implement Fuzzy Search navigation using `nucleo` and `ignore`.
- [x] **Task 12:** Integrate cross-platform Clipboard fallbacks (`arboard`, OSC 52).
- [x] **Task 13:** Implement the `JulesAdapter` and integrate `cli-chat-rs` as a full-screen modal overlay for Scoped Chat with the 7-message limit.
- [x] **Task 14:** Implement tree/list rendering for Jules "Plan/Todo list" activities within the chat interface.
- [x] **Task 15:** Implement Contextual Auto-Sync: Jules API Refresh (polling) active only when chat is open, and Git Sync (code fetching) on the main view.
- [x] **Task 16:** Implement the New Session Initialization flow (`n` keybind, prompt input, `create_session` parameter formatting, and dynamic rule injection).
- [x] **Task 17:** Implement Strict Branch Protection (auto-checkout of local branches when interacting with AI `🦑` branches).
- [x] **Task 18:** Implement the Keyboard-Driven Visual Patch Stack for catalog shopping (Interactive cherry-picking, reordering, squashing `s`, dropping `d`).
- [x] **Task 19:** Implement Dual-Patching functionality: `a` for cherry-picking Git commits, and raw `/artifacts` patching via API.
- [x] **Task 20:** Implement Read-Only Observer Mode (`v` or `b`) with visual cues and disabled actions.
- [x] **Task 21:** Implement External `$EDITOR` Fallback (`e` keybind) for manual review/modification of patches.
- [x] **Task 22:** Implement Tier 1 Conflict Resolution Framework: The Conflict Resolution Modal (`[O]` Keep Ours, `[T]` Keep Theirs, `[U]` Undo/Abort, `[M]` Manual Resolve via IDE).
- [x] **Task 23:** Implement Tier 3 Conflict Resolution: "Magic Wand" Auto-Merge for non-conflicting changes using `diffy`, and `git rerere` integration.
- [x] **Task 24:** Implement Tier 2 Conflict Resolution: AI-Assisted Auto-Resolution with structured XML prompt generation and session targeting.
- [x] **Task 25:** Implement Tier 4 Conflict Resolution: Isolated parallel testing support via `git worktree`.
- [x] **Task 26:** Implement the Settings & Configuration UI Overlay to manage global config, defaults, AI rules, and sync statuses.
- [ ] **Task 27:** Implement the Universal AI Git Orchestrator: PTY/Terminal Emulation within Ratatui to host external local CLIs (`claude-code`, `opencode`).
- [ ] **Task 28:** Implement the Isolated Worktree Encapsulation: Automatically spawn `git worktree` sandboxes for local CLI processes, capture file changes, and convert them to commit patches within the Visual Patch Stack.