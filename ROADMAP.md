
## 5. Technical Excellence & DX (Developer Experience)
Inspired by top-tier CLI tools like `codex-rs` and `Helix`, `julesctl` will adopt the following engineering principles to ensure it is blisteringly fast, keyboard-centric, and a joy to use:

### A. Blazing Fast Navigation & Search
- **Fuzzy Finding (`nucleo` & `ignore`):** Integrate `nucleo` for ultra-fast, `.gitignore`-aware searching. This will be used in the Home Screen for instantly finding/filtering Workflows, and potentially inside the Chat UI to quickly attach file context (e.g., typing `@src/ma...`).

### B. Professional Terminal Ergonomics
- **Vim Keybindings:** Native support for `j`/`k` (navigation), `g`/`G` (top/bottom), and `/` (search) across all List and Paragraph widgets.
- **External Editor Fallback (`$EDITOR`):** Pressing `e` on a massive AI patch or prompt input will temporarily suspend the TUI and open the user's preferred terminal editor (like `vim` or `nano`). Once saved and closed, `julesctl` resumes with the edited text.
- **Flawless Clipboard (`arboard` + OSC 52):** Copying AI-generated code should "just work", even over SSH or inside WSL, by falling back to OSC 52 terminal escape sequences if standard clipboard access fails.

### C. Decoupled UI Architecture (Non-Blocking)
- **Centralized Event Bus (`AppEvent`):** The TUI must never freeze while waiting for the Jules API or Git commands. We will implement an asynchronous Message Bus (e.g., passing `AppEvent::FetchCommits` or `AppEvent::SessionCreated` through `tokio::sync::mpsc`). The UI simply renders state, while background workers handle network and disk I/O.
- **Diff Rendering (`diffy`):** For previewing changes before applying them, or viewing patches fetched directly from the API, we will use `diffy` to generate and render beautiful red/green unified diffs directly in the `ratatui` Right Pane.
