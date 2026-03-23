# julesctl

> Jules AI local workflow manager — a Git-first, visual orchestrator for your AI coding sessions.

`julesctl` transforms your local terminal into a powerful branch and patch manager for Jules AI. Instead of manually copying code from a browser or constantly switching branches, `julesctl` lets you spin up multiple parallel Jules sessions (AI developers) from any commit, visualize their changes as a Git tree, and seamlessly cherry-pick, test, or revert their patches right from your editor.

## Documentation & Architectural Manifesto
For a comprehensive overview of our architectural goals, including UI layout, Git-first branching strategies, conflict resolution tiers, and cross-device syncing via Ahenk, please see our detailed documentation:
* [Roadmap & Architectural Index](docs/ROADMAP.md)

## The Paradigm: Git-First Workflow

`julesctl` abandons the traditional "chat-bot" UI as its primary interface.

Instead, it treats Jules AI sessions as **Git Branches**.
When you start a task, Jules writes code and commits it to a remote branch (`jules/task-...`).
`julesctl` visually graphs these branches alongside your local work.

### Features
1. **Visual Git Workflow:** See exactly where your local code is, where Jules branched off, and what commits Jules has made (marked with 🦑).
2. **Patch Picker:** Select any Jules commit in the TUI to instantly preview the `diff`.
3. **Seamless Integration:** Apply (`cherry-pick`), revert, or merge Jules' code without leaving the dashboard.
4. **Context-Aware Chat:** Need to talk to Jules about a specific branch? Press `C` on a Jules commit to launch a scoped `ratatui` chat interface (`cli-chat-rs`) that understands the exact context of that branch.
5. **Moddable Rules:** Fully compatible with tools like Get-Shit-Done (GSD). Automatically injects `.gsd/context.md` or `AGENTS.md` into new Jules sessions.

## Installation

```bash
git clone https://github.com/rvoidex7/julesctl
cd julesctl
cargo install --path .
```

## Setup & Usage

```bash
cd my-project
julesctl init
```
This creates `.config/julesctl/config.toml` (and global rules in `~/.config/julesctl/rules/`).

Run the dashboard:
```bash
julesctl
```

### The Dashboard UI

- **Left Panel (Git Graph):** Displays your current workflow's commits.
  - 🐱 = Github/Remote commit
  - 🦑 = Jules AI session branch
  - (No emoji) = Local unpushed commit
- **Right Panel (Top):** Shows the `diff` (patch preview) and commit message for the selected node.
- **Right Panel (Bottom):** Available actions (`[A] Apply`, `[R] Revert`, `[C] Open Chat`, `[N] New Session`).

## TUI Dashboard and cli-chat-rs Integration

Running `julesctl` with no arguments launches the **Project Dashboard**. This dashboard scopes directly to your currently active project directory and presents the visual Git tree.

From this Dashboard, you can select an active Jules task (🦑). `julesctl` will then spin up the `cli-chat-rs` generic TUI specifically scoped to that single session. This ensures that `cli-chat-rs` remains a lightweight and decoupled chat framework utilizing `ratatui`, whilst `julesctl` manages the complex branch orchestration.
