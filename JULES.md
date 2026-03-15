# JULES.md — julesctl Implementation Specification

> This document is the authoritative implementation guide for `julesctl`.
>
> Jules: read this document fully before writing any code.

---

## Project Overview

**Name:** `julesctl`
**Language:** Rust (2021 edition)
**Purpose:** A visual, Git-first TUI for managing multiple parallel Jules AI sessions. It acts as a branch and patch manager, allowing users to preview, cherry-pick, and orchestrate AI-generated code without leaving their terminal.

**Core philosophy:**
- Jules does the coding. `julesctl` does the orchestration.
- The primary source of truth for code changes is **Git**, not the Jules API `artifacts` endpoint. We fetch branches and commits (`jules/task-...`) to get accurate history, commit messages, and diffs.
- The UI is a Dashboard (Node/Tree view) first, and a Chat UI second.

---

## Repository Structure

```
julesctl/
├── src/
│   ├── main.rs              — CLI entrypoint, launches Dashboard TUI
│   ├── tui_dashboard.rs     — Ratatui Dashboard (Git Graph, Patch Preview, Actions)
│   ├── api/
│   │   ├── mod.rs           — Jules API client (reqwest)
│   │   ├── sessions.rs      — Session CRUD
│   │   └── activities.rs    — Message history polling
│   ├── git/
│   │   ├── mod.rs           — Git operations (fetch, log, cherry-pick, reset)
│   │   └── graph.rs         — Logic to parse git log into UI tree nodes
│   ├── config/
│   │   ├── mod.rs           — Config loading
│   │   ├── schema.rs        — Config structs (TOML)
│   │   └── rules.rs         — Global/Local rule parsing (AGENTS.md, GSD)
│   └── adapter/
│       └── cli_chat_rs.rs   — cli-chat-rs MessagingAdapter for Jules API
├── cli-chat-rs/             — Workspace member (Generic Ratatui Chat TUI)
├── JULES.md                 — This file
├── README.md                — User-facing documentation
└── Cargo.toml
```

---

## The Workflow Paradigm

We group sessions under a **Workflow** (tied to the current repository).

1. **Initialization:** The user runs `julesctl` in a repo. If the local git state is behind or ahead of `origin`, the TUI warns the user to Sync (Push/Pull) so Jules starts from the correct codebase.
2. **Creating a Session:** The user presses `N` to start a new Jules session from the currently selected commit. The `julesctl` appends local context (`.gsd/context.md`, `AGENTS.md`) and global rules to the prompt, creates the session via API, and Jules creates a remote branch (`jules/task-id`).
3. **Graph Visualization:** `julesctl` runs `git fetch origin` in the background and parses `git log --graph`. It draws the tree in the Left Panel.
   - Commits on `origin/main` get 🐱.
   - Commits on `origin/jules/task-id` get 🦑.
   - Local commits have no emoji.
4. **Patch Preview:** Moving the cursor over a 🦑 commit runs `git show <commit>` and displays the diff and commit message in the Top Right panel.
5. **Action:** Pressing `A` (Apply) runs `git cherry-pick <commit>` or `git merge` into the user's current local branch.

---

## Extensibility (Moddability)

`julesctl` is fully moddable by external meta-prompting tools (like Get-Shit-Done / GSD).

- **Global Rules:** Read from `~/.config/julesctl/rules/system_prompt.md`.
- **Local Context:** Automatically appended if `.julesctl/rules.md`, `.gsd/context.md`, or `AGENTS.md` exist in the current working directory.

This allows third-party tools to inject behavior without modifying `julesctl` code.

---

## TUI Dashboard Architecture (Ratatui)

The primary interface (`src/tui_dashboard.rs`) is built with `ratatui`.

**Layout:**
- **Left (30%):** The Git Tree / Workflow View. A `List` or custom widget drawing the branch topology.
- **Top Right (70% width, 70% height):** Diff Preview (`Paragraph` or `Text` widget).
- **Bottom Right (70% width, 30% height):** Keybindings and Actions.

**State Management:**
- The Dashboard holds the `git log` state.
- When `C` (Chat) is pressed on a Jules session node, the Dashboard returns an action `OpenChat(session_id)`.
- `main.rs` then passes control to `cli-chat-rs::MessengerApp`.
- When `cli-chat-rs` is closed, control returns to the Dashboard.

---

## cli-chat-rs Integration

`cli-chat-rs` is an internal workspace member. It is a purely generic messaging TUI.

`julesctl` implements `MessagingAdapter` (`src/adapter/cli_chat_rs.rs`) which translates `get_messages()` into Jules API `/activities` calls, and `send_message()` into POST requests.

**Important:** `cli-chat-rs` does NOT handle git patching. It only handles chatting.

---

## Code Cleanup Rules

- Remove old polling loops (`src/modes/`) that relied on automated `git apply` queues. The new paradigm is user-driven cherry-picking via the Dashboard.
- Remove API artifact-based patching (`src/patch/`). We now rely entirely on `git fetch` and `git cherry-pick`.
- Keep it simple: Ratatui provides excellent primitives. Don't over-engineer custom widgets if standard `List` and `Paragraph` can display text and colors effectively.
