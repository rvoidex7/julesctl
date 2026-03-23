# The Conflict Resolution Framework

When a developer applies (cherry-picks) an AI patch via the TUI, Git conflicts are inevitable. Instead of dumping the user into a raw terminal, `julesctl` will pop up a dedicated **Conflict Resolution Modal** offering distinct tiers of solutions. This avoids overengineering by staying focused solely on AI workflow orchestration rather than building a full `gitui` clone.

## Tier 1: Classical 1-Click Operations

*   **`[O]` Keep Ours:** Preserve the local code for the conflicting lines, discarding the AI's patch in those specific areas.
*   **`[T]` Keep Theirs:** Overwrite the local code with the AI's newly generated lines for the conflict block.
*   **`[U]` Undo / Abort:** Quickly abort the cherry-pick (`git cherry-pick --abort`) to revert the working branch to a safe state, allowing the user to test another approach or review the code safely.
*   **`[M]` Manual Resolve via IDE:** Launch the raw file containing conflict markers (`<<<<<<< HEAD`, `=======`, `>>>>>>>`) directly in the user's preferred editor. `julesctl` will attempt to detect the current IDE environment (e.g., `$TERM_PROGRAM=vscode`) or rely on Git's `core.editor` to open the file seamlessly in Vim, VSCode, etc., utilizing sustainable OS-level fallbacks.

## Tier 2: AI-Assisted Auto-Resolution (The Killer Feature)

*   **Structured Prompt Generation:** `julesctl` will automatically parse the conflicting file and wrap the conflict markers (`<<<<<<<` to `>>>>>>>`) inside structured XML tags (e.g., `<local_code>` vs `<ai_code>`). This prevents LLM hallucination and clearly defines the context.
*   **`[R]` Resolve via AI:** The generated XML prompt will be dispatched to an active Jules Session to autonomously merge and resolve the logic.
*   **Target Selection & Clipboard:** The user can select *which* active Jules Session should receive this resolution prompt (e.g., the session that originally caused the conflict). Alternatively, a "Copy to Clipboard" feature allows the user to paste this structured XML prompt into an external web interface if desired.

## Tier 3: Enterprise IDE & Deep Git Mechanics

To provide a premium developer experience comparable to IntelliJ IDEA Ultimate, `julesctl` will leverage deep Git features and advanced algorithms invisibly in the background, significantly reducing the frequency of manual conflict prompts:

*   **`git rerere` (Reuse Recorded Resolution):** `julesctl` will integrate an optional toggle in the [Settings UI](../ui/settings-ui.md) to enable Git's `rerere` cache (`.git/rr-cache/`). If an AI session encounters the exact same conflict block that the user previously resolved manually, Git will automatically replay the user's past decision without pausing the orchestration workflow.
*   **"Magic Wand" / Auto-Merge Non-Conflicting Changes:** Before presenting a raw file conflict, `julesctl` will utilize robust open-source Rust libraries (like the existing `diffy` crate, which supports 3-way diffing logic) to safely and automatically merge chunks of code that are physically distant or logically non-overlapping (e.g., local imports added at the top vs AI functions added at the bottom). Only true, overlapping line conflicts will trigger the Tier 1 or Tier 2 modal.

## Tier 4: Non-Destructive Worktrees & Patch Catalogs

A core architectural manifesto of `julesctl` is treating Jules AI remote branches (`🦑`) purely as an immutable **"Catalog of Commits"** (See: [Visual Patch Stack](../git-workflow/visual-patch-stack.md)). To prevent AI push/pull sync failures and preserve the pristine state of generated solutions:

*   **Strict Cherry-Picking (No Rebasing):** Users must never directly check out, modify, or `git rebase` a Jules AI branch. If a user wishes to extract the 3rd commit from a Jules session while skipping the first two, they must do so via non-destructive cherry-picking (`[A] Apply`) into their own isolated local working branch (`💻`). The AI branch remains permanently intact as a referenceable catalog.
*   **Isolated Parallel Testing via Git Worktrees:** To manage multiple complex conflict resolutions or parallel AI experiments simultaneously, `julesctl` will natively support `git worktree`. This allows developers to check out different local branches (each integrating different AI patches) into physically isolated, hidden directories (e.g., `~/.cache/julesctl/worktrees/`) linked to the same `.git` repository. This ensures zero cross-contamination when switching contexts or IDE windows.

## Intelligent Conflict Parsing (Beyond Line-Diffs)

When applying AI patches, Git's default line-based merge logic often flags false conflicts. `julesctl` will utilize advanced Rust libraries to silently resolve trivial collisions before ever bothering the user:

*   **`diffy` (3-Way Text Merging):** Will be used to power the "Magic Wand" feature, automatically merging chunks of code that are physically distant or logically non-overlapping.
*   **`tree-sitter` (Semantic / AST Merging - Future Tier):** To truly understand code logic, `tree-sitter` bindings can be utilized to parse conflicting files as Abstract Syntax Trees (AST). This allows `julesctl` to determine if a collision is purely structural (e.g., an AI adding a parameter to a function the user just renamed) and safely extract context for the Tier 2 AI-Assisted XML conflict prompt.
