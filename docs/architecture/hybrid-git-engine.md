# Hybrid Git Engine & Implementation Guidelines

To prevent "overengineering" while achieving the extreme performance required for a modern terminal orchestrator, `julesctl` carefully delegates responsibilities across a hybrid stack of system tools and specialized Rust crates. This architecture draws inspiration from the state-management of massive Rust TUIs (like Codex CLI or Helix) while strictly avoiding their domain-specific logic (e.g., executing remote LLMs).

## The Hybrid Git Engine

Replacing the system Git completely with native libraries is an anti-pattern that leads to unmaintainable network/SSH authentication code. Instead, `julesctl` splits Git operations:

1.  **Lightning-Fast UI Reads (`gix` / `gitoxide`):** To achieve smooth, 60-FPS rendering of branch trees and thousands of commits without freezing the UI thread, `julesctl` will natively parse `.git` objects using `gitoxide`. This completely eliminates the slow OS subprocess overhead of shelling out for `git log` or `git status`.
2.  **Reliable Writes & Networking (`std::process::Command`):** For executing `git fetch`, `git push`, complex rebases, or authenticating via SSH/HTTPS, `julesctl` will rely entirely on asynchronously shelling out to the user's system `git` executable. This ensures 100% compatibility with user configurations, proxy settings, and credential helpers.

## Implementation Guidelines & Rust Best Practices for AI Agents

When developing or refactoring features for `julesctl`, the following Rust optimizations and safety constraints MUST be obeyed to ensure the TUI remains blazing fast and stable:

### 1. Asynchronous Subprocesses (Avoiding UI Freezes)

*   TUI applications block on the main thread. Therefore, executing heavy blocking commands like `git fetch` or `git log` via `std::process::Command` directly in the UI loop is strictly prohibited.
*   **Rule:** Use `tokio::process::Command` or wrap blocking Git shell invocations in `tokio::task::spawn_blocking` to keep the UI responsive.

### 2. TUI Rendering Optimizations (Zero-Copy & Lifetimes)

*   The `terminal.draw` closure executes dozens of times per second. Allocating strings inside this loop drastically impacts performance.
*   **Rule:** Avoid `.clone()`, `.to_string()`, and `format!` inside rendering blocks whenever possible. Instead, structure data with references (`&'a str`) or use Ratatui's `Cow` (Clone-on-write) wrapped strings (like `Span::raw("Static")`).

### 3. Safe Truncation (UTF-8 Emoji Awareness)

*   Slicing multi-byte characters (like the 🦑 emoji) at byte boundaries (e.g., `&s[..10]`) will cause the application to panic and ruin the terminal state.
*   **Rule:** Always truncate strings safely using iterators: `.chars().take(n).collect::<String>()`.

### 4. Error Handling and Panic Prevention

*   Panicking in a raw-mode terminal leaves the user's terminal broken (no echoing, hidden cursor).
*   **Rule:** The use of `.unwrap()` and `.expect()` is strictly forbidden outside of test modules. All errors must be handled safely and surfaced to the UI's `action_log` or status bar.
