# Local CLI Encapsulation via Worktrees

The "Universal AI Git Orchestrator" capability within `julesctl` solves a critical UX and architectural problem when developers use both cloud-based AI orchestrators (like Jules) and highly capable local CLI tools (like `claude-code`, `opencode`, or `codex-cli`).

## The Problem
Local coding CLIs are exceptionally powerful because they utilize the host system's compute and context, but they usually lack **Git Awareness**. They read and mutate files directly in the active primary working directory (`std::fs`), which creates immediate risks:
- Conflicts with the developer's uncommitted manual changes.
- Conflicts when switching branches (`git checkout`) while the local AI is actively modifying the file tree.
- No ability to logically isolate experiments into different Git branches.

## The Worktree Sandbox Architecture

`julesctl` acts as a "mothership" for these 3rd-party tools. Instead of rewriting an internal AI coding client, `julesctl` leverages the `git worktree` mechanism to safely encapsulate external processes.

### 1. Transparent Isolation
When a developer decides to spin up a local agent via `julesctl` (e.g., creating a new session and selecting a local tool rather than the Jules API), `julesctl` executes the following background steps:
- A new dedicated local branch is created (e.g., `local/claude-task-1`).
- `git worktree add` is utilized to create a hidden, parallel copy of the current repository state into a completely isolated directory (e.g., `~/.julesctl-worktrees/repo-task-1`).
- The primary working branch and IDE directory remain untouched and completely safe.

### 2. Execution via Embedded PTY
`julesctl` spawns the selected 3rd-party CLI tool as a subprocess, injecting the path of the hidden worktree sandbox as the `Current Working Directory (CWD)`.
Using a Pseudo-Terminal (PTY) emulation layer within Ratatui, the output (stdout) of the local tool is piped directly into a dedicated widget/pane inside the `julesctl` TUI.
- The external tool "believes" it is running in the project root.
- The external tool operates, uses subagents, reads, and writes to files within the hidden worktree.
- The developer interacts with the tool naturally inside the embedded `julesctl` frame via `stdin`.

### 3. Converting Edits into Patch Artifacts
Once the local tool finishes execution, or at periodic intervals, `julesctl` scans the isolated worktree for changes.
Any modifications are automatically bundled, committed to the underlying `local/claude-task-...` branch, and then presented in the `julesctl` **Visual Patch Stack**.

### Summary
By combining embedded PTY execution with `git worktree` sandboxes, `julesctl` instantaneously brings enterprise-grade branch management, conflict resolution, and visual diffing to 3rd-party AI CLI tools without requiring any upstream modifications to those tools.
