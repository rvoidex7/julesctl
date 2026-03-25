# Contributing to julesctl

First off, thank you for considering contributing to `julesctl`!

`julesctl` is a high-performance, Git-first workflow orchestrator for Jules AI. To maintain its speed, reliability, and clear architectural vision, we have established strict guidelines for contributors.

**Please read this entire document before submitting a Pull Request.**

## 1. The Architectural Manifesto

`julesctl` is built upon a very specific set of design principles and architectural rules. We do not accept features or code changes that violate these principles.

Before you write any code, you **must** read and understand our complete documentation suite located in the `docs/` directory, specifically starting with the [ROADMAP & Architectural Manifesto](docs/ROADMAP.md).

Key concepts you need to grasp:
*   **The Git-First Paradigm:** We do not build traditional chat bots. Everything revolves around Git branches (`🦑`).
*   **The Hybrid Git Engine:** Understand when to use `gix` (fast reads) versus shelling out to `std::process::Command` (reliable writes/networking).
*   **Decoupled Chat:** `cli-chat-rs` must remain completely agnostic to `julesctl` business logic.

## 2. Strict Rust Development Rules

To ensure a blazing-fast, crash-free terminal experience (especially critical in raw mode), all code must adhere to the following strict safety and optimization rules outlined in [Hybrid Git Engine & Optimizations](docs/architecture/hybrid-git-engine.md):

### A. Non-Blocking UI Threads
*   **Rule:** You must never run blocking code (e.g., heavy disk I/O, `git fetch`, or API network calls) on the main Ratatui rendering thread.
*   **Enforcement:** Use `tokio::process::Command` or `tokio::task::spawn_blocking` for all system commands and network operations.

### B. Zero-Panic Policy
*   **Rule:** The use of `.unwrap()` and `.expect()` is **strictly forbidden** outside of `#[cfg(test)]` modules.
*   **Why:** Panicking in a raw-mode terminal breaks the user's terminal session (e.g., hiding the cursor, stopping input echo).
*   **Enforcement:** All errors must be handled gracefully using `Result`, and bubbled up to the TUI's internal event bus (`AppEvent`) or status bar for the user to see.

### C. Safe String Truncation
*   **Rule:** You must safely slice strings containing multi-byte UTF-8 characters (like our heavily used `🦑` emoji).
*   **Enforcement:** Never use raw byte slicing (`&s[..10]`). Always use iterators: `.chars().take(n).collect::<String>()`.

### D. Render Loop Optimization
*   **Rule:** Minimize string allocations inside the Ratatui `terminal.draw` closure.
*   **Enforcement:** Avoid `.clone()`, `.to_string()`, and `format!` in rendering blocks. Rely on references (`&'a str`) or `Span::raw()`.

## 3. Submitting a Pull Request

1.  **Fork the repository** and create your branch from `main`.
2.  **Follow the Code Style:** Ensure your code passes standard Rust checks by running `cargo fmt` and `cargo clippy`.
3.  **Run the Tests:** Run `cargo test` to ensure you haven't broken existing functionality.
4.  **Describe Your Changes:** Your PR description must clearly explain *what* you changed and *why*. If your change affects the architecture, explicitly mention which part of the `ROADMAP.md` it aligns with or modifies.
5.  **AI Code Review:** Note that your PR will be automatically reviewed by the `google-labs-code/jules-action` bot upon submission. Please address any constructive feedback it provides.

## 4. Code of Conduct

By participating in this project, you agree to abide by our [Code of Conduct](CODE_OF_CONDUCT.md). We expect all contributors to maintain a respectful and collaborative environment.

Thank you for helping make `julesctl` the best local AI orchestrator available!
