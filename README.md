<div align="center">
  <img src="https://img.shields.io/badge/Rust-000000?style=for-the-badge&logo=rust&logoColor=white" alt="Rust">
  <img src="https://img.shields.io/github/actions/workflow/status/rvoidex7/julesctl/ci.yml?style=for-the-badge&logo=githubactions&logoColor=white" alt="Build Status">
  <img src="https://img.shields.io/badge/License-Apache%202.0-blue.svg?style=for-the-badge" alt="License">
  <img src="https://img.shields.io/badge/Platform-Linux%20%7C%20macOS%20%7C%20Windows%20%7C%20Termux-lightgrey?style=for-the-badge" alt="Platforms">

  <h1>julesctl</h1>
  <p><strong>A Professional Git-First Workflow Manager for Jules AI</strong></p>
  <p><em>Stop chatting with bots. Start orchestrating your code.</em></p>
</div>

---

## 🚀 The Jules Experience, Reimagined

The official Jules web UI treats AI sessions as a flat, conversational history. As your projects scale, finding the right code, managing parallel experiments, and resolving conflicting suggestions becomes a chaotic mess of copy-pasting.

`julesctl` completely abandons the traditional "chatbot" paradigm. We believe AI coding assistants should be managed with the same rigorous, structured tools developers use for human collaboration: **Git**.

By treating every Jules AI session as a dedicated Git branch (`🦑`), `julesctl` transforms your terminal into a powerful, visual orchestration dashboard. It groups your sessions into logical **Workflows**, displaying them as a clean hierarchical tree, allowing you to seamlessly shop for patches, resolve conflicts, and merge AI-generated code directly into your local environment.

---

## ✨ Why `julesctl` is Different

### 1. The Git-First Workflow Paradigm
There is no "manual chat mode" in `julesctl`. Every interaction is contextualized within a local repository **Workflow**.
*   **Infinite Parallelism:** Spawn a "Planner" session, and from its branch, manually spin off ten parallel "Worker" sessions. `julesctl` tracks the exact origin and divergence of every AI thought process.
*   **Visualizing the Chaos:** The built-in TUI maps your local working branch alongside Jules' remote branches (`jules/task-...`). You instantly see where the AI branched off, what it committed (`🦑`), and how it relates to your local changes (`💻`).
*   **Immutable AI Branches:** We treat Jules branches as a pristine "Catalog of Commits." You never rebase or break the AI's sync. Instead, you cherry-pick exactly what you need.

### 2. The Visual Patch Stack (Interactive Cherry-Picking)
Say goodbye to messy web-to-IDE copy-pasting. `julesctl` features a keyboard-driven (Vim/Arrow keys) interface to shop for AI code:
*   Preview in-memory diffs instantly without slow disk operations.
*   **Dual-Patching:** Choose to apply (`[A]`) formalized Git commits (cherry-picking) or pull raw API artifact patches for rapid local testing.
*   Reorder, squash (`s`), or drop (`d`) AI commits directly from the TUI before merging them into your isolated `git worktree`.

### 3. The 4-Tier Conflict Resolution Framework
When an AI patch collides with your local code, `julesctl` doesn't just dump you into a broken file. Our robust framework handles it:
*   **Tier 1 (1-Click Triage):** Instantly Keep Ours `[O]`, Keep Theirs `[T]`, Undo `[U]`, or open your default `$EDITOR` (`[M]`).
*   **Tier 2 (AI-Assisted Auto-Merge):** The killer feature. `julesctl` wraps conflict markers (`<<<<<<<` vs `=======`) in strict XML and dispatches them back to Jules for autonomous resolution.
*   **Tier 3 (Enterprise Mechanics):** Background integration with `git rerere` (Reuse Recorded Resolution) and Rust libraries (`diffy`) silently auto-merges non-overlapping logic before you even see a conflict prompt.
*   **Tier 4 (Safe Worktrees):** Test conflicting AI ideas simultaneously in hidden `git worktree` directories without contaminating your primary codebase.

### 4. A Native, Blazing-Fast TUI Architecture
*   **Hybrid Git Engine:** We use native Rust libraries (`gitoxide`) to render massive commit trees at 60 FPS without blocking the UI thread, while intelligently delegating complex network operations to the system's `git` executable.
*   **Responsive & Touch-First:** Fully adaptive layout. Side-by-side panes on Desktop PC; stacked panes on mobile Android/Termux environments, with complete Ratatui mouse/touch support.
*   **Scoped Chat (`cli-chat-rs`):** When you need to talk to Jules, press `C` on a commit. A decoupled, full-screen overlay opens, injected *only* with the context of that specific branch branch, maintaining maximum performance.

### 5. Cross-Device Syncing via Ahenk
Transition seamlessly from your Desktop to your phone via Termux. `julesctl` uses P2P [Ahenk](https://github.com/Appaholics/Ahenk) sync to securely transfer your global UI settings, active workflow tabs, and API cache (`~/.config/julesctl/`) across devices, while strictly leaving code synchronization to the native Git protocol.

### 6. Universal AI Git Orchestrator (Agnostic CLI Encapsulation)
`julesctl` doesn't just manage cloud agents. It serves as the mothership for local 3rd-party coding tools (e.g., `claude-code`, `opencode`). By embedding them within a built-in Pseudo-Terminal (PTY) and redirecting their execution into isolated `git worktree` sandboxes, `julesctl` instantly grants branch management, visual diffing, and conflict resolution capabilities to local tools that natively lack Git awareness, preventing any collision with your primary working branch.

---

## 📚 Architectural Manifesto & Documentation

`julesctl` is built on a strict, highly considered architectural foundation. We do not accept features that violate our core principles (e.g., adding heavy full-terminal dependencies, blocking UI threads, or abandoning the Git-first model).

For a deep dive into how `julesctl` works under the hood, read our complete documentation index:

👉 **[ROADMAP & Architectural Manifesto](docs/ROADMAP.md)**

---

*(Note: `julesctl` is currently in active development. Installation and binary distribution instructions will be provided in a future release.)*
