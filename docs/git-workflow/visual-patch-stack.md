# The Visual Patch Stack

Because Jules AI remote branches (`🦑`) are treated purely as an immutable **"Catalog of Commits"**, users need a powerful UI to browse and extract patches into their local testing branches (`💻`).

## 1. Keyboard-Driven Patch Shopping

Instead of buggy mouse drag-and-drop, the TUI will feature a robust keyboard-driven "Interactive Cherry-Pick" interface.
*   Users can navigate a Jules branch using `j/k`.
*   Press `Shift + Up/Down` to visually reorder commits.
*   Press `s` to squash two commits together.
*   Press `d` to drop a bad patch.
*   Once the list is finalized in the UI, `julesctl` executes a silent, automated sequence of `git cherry-pick` commands to construct the exact desired state on the user's local branch, leaving the original AI branch completely untouched.

## 2. Dual-Patching Specifics

The Dual-Patching mechanism gives developers complete control over how AI code is integrated:

*   **Git Commits (Cherry-Picking):** Users can highlight an AI commit (🦑) in the Workflow View and press `a` to cherry-pick the formal Git commit into their local working branch. This preserves the AI's commit history and message.
*   **Raw API Artifacts (Patch Pulling):** Alternatively, users can fetch the raw diff directly from the Jules API `/artifacts` endpoint. This is useful for applying the exact current state of the code without pulling the entire Git history, ideal for quick testing or manual adjustments before formalizing a commit.

## 3. Read-Only Observer Mode (`v`)

Toggled via the `v` keybind, the Observer Mode allows users to safely inspect other branches and commits without altering their active working branch.

*   **Visual Indicators:** When active, the TUI will display distinct visual cues (e.g., a specific border color or a prominent status bar warning) to clearly indicate that the environment is locked.
*   **Functional Restrictions:** All state-modifying actions (like `a` for apply, or `n` for new session) will be disabled or hidden to prevent accidental changes.

## 4. External `$EDITOR` Fallback

For scenarios requiring complex manual conflict resolution or deep code review, `julesctl` will support launching an external editor.
*   Pressing a dedicated keybind (`e`) will temporarily save the current patch or payload to disk, open the user's `$EDITOR` (e.g., `vim`, `nano`, `hx`), and automatically ingest the modified content upon exit.
