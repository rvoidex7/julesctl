# TUI Layout & Navigation

The TUI must be native, fast, feature-rich regarding keyboard/mouse interactions, and fully **Responsive** for mobile/Android (Termux) environments.

## 1. Responsive & Touch-First Design

*   **Adaptive Layout:** The UI automatically adapts based on screen width: **Side-by-side (Horizontal)** panes on Desktop PC, and **Stacked (Vertical)** panes on narrow screens (Android/Termux).
*   **Touch-First Interactions:** Every interactive element (Tabs, List items, Action buttons) must natively support Ratatui Mouse events. This ensures seamless tapping on mobile touchscreens and clicking on desktop mice.

## 2. Top-Level Tabs (Web Browser Style)

*   The very top of the screen features a tabbed interface representing active Workflows.
*   Users can seamlessly switch between entirely different Workflows (even across different projects) instantly, akin to switching tabs in a web browser. The old, useless "Title Pane" is removed.

## 3. The 2-Pane Workflow View Layout (No 3rd Pane Clutter)

To avoid cramping the screen (especially critical for Termux/mobile), we strictly use 2 Panes. Complex views like Chat open as full-screen overlays/modals over the Workflow View (See: [Scoped Chat Integration](scoped-chat-integration.md)).

### Pane 1 (Viewer/Navigator - Left/Top)

This pane contains internal toggles to switch between different data scopes and visual representations.

*   **Scope Toggle:**
    *   **Workflow-scoped:** Shows only the AI sessions, branches, and commits created specifically for the currently active Workflow.
    *   **Global-scoped:** Shows the entire repository's raw git data (all branches, all commits).
*   **View Toggle:**
    *   **Branch View:** A hierarchical, folder-like structure. We will specifically implement the `tui-tree-widget` library here to display the nested relationships of branches cleanly and natively.
    *   **Commit Graph View:** A custom-coded native ASCII git graph parsing implementation (similar to `git log --graph`).
        *   **The Working Branch Line:** In this graph, the **leftmost continuous vertical line** strictly represents the active local working branch (the stable integration ground).
        *   **Emoji Graph Nodes:** The standard Git `*` nodes in the graph are explicitly replaced by identity emojis for instant visual recognition:
            *   🐱 (Remote/GitHub commits)
            *   🦑 (Jules AI session branches)
            *   💻 (Local branches/commits)

### Pane 2 (Details/Patch Preview - Right/Bottom)

*   Dynamically updates based on Pane 1's selection.
*   Displays the exact patch/diff preview, commit message, or branch details.
*   *Implementation Detail:* We will use the `diffy` crate for in-memory, blazing-fast red/green diff generation when previewing API patches, avoiding heavy, slow disk-based Git commands for UI previews.

## 4. Professional Keybindings & Navigation Standards

To ensure a seamless, native terminal experience, the following universal keybinding map is strictly enforced:

### Navigational Fallbacks

*   Native **Vim movement keys** (`j` down, `k` up, `h` collapse left, `l` expand right, `g` top, `G` bottom) are fully supported across all lists and scrollable components.
*   Standard **Arrow Keys** remain universally active for users unfamiliar with Vim.

### Action Keybindings (First-Letter Mnemonic)

*   **`Tab`**: Switch between active top-level Workflows.
*   **`v`**: View toggle (Switch Left Pane between Branch Tree and Commit Graph).
*   **`s`**: Scope toggle (Switch Left Pane between Workflow-only and Global repo data).
*   **`c` or `Enter`**: Open the Chat (`cli-chat-rs`) overlay for the selected Jules branch.
*   **`a`**: Apply / Cherry-pick the currently previewed patch/commit into the active working branch.
*   **`r`**: Revert / Undo the selected commit or patch.
*   **`n`**: Initialize a **New Session** from the currently highlighted branch.
*   **`b`**: Enter **Read-Only Observer Mode** (Inspect a branch's commits/files without checking out or altering the local working environment).
*   **`e`**: Open the currently previewed patch or payload in the external `$EDITOR` (e.g., vim/nano) for manual review/modification.
*   **`q` or `Esc`**: Close modal/overlay or quit the application.
*   **`/`**: Open the fuzzy finder search bar.

## 5. Advanced Navigation Features

*   **Fuzzy Search (`/`):** Pressing `/` will open a fuzzy finder powered by `nucleo` and `ignore`. This feature will allow blazing-fast navigation by searching through branch names, commit messages, and file contents across the repository and active workflows.
*   **Clipboard Fallbacks (`arboard`, OSC 52):** To enhance usability across different terminal emulators and SSH sessions, `julesctl` will integrate robust clipboard support. We will utilize `arboard` for native OS clipboard integration. As a fallback for remote or headless environments, OSC 52 escape sequences will be implemented, allowing seamless copying of commit hashes, diff snippets, and chat messages.