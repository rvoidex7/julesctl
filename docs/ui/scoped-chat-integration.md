# Scoped Chat Integration & `cli-chat-rs` Decoupling

A critical, non-negotiable architectural rule is the absolute decoupling of the `cli-chat-rs` component from `julesctl`.

## 1. The Agnostic UI Framework (`cli-chat-rs`)

*   `cli-chat-rs` is a standalone, generic Ratatui messaging UI library.
*   It knows absolutely nothing about "Jules API", "Git", "Branches", or "Workflows".
*   It must remain generic enough to be plugged into WhatsApp, Telegram, Discord, or any other CLI messaging tool.

## 2. The `JulesAdapter` Integration

*   `julesctl` will implement a specific `JulesAdapter` that translates Jules API `Activity` payloads into the generic `cli-chat-rs` message types.

## 3. Scoped Chat Access & Limitations

*   **Launch Point:** The Chat Interface (`cli-chat-rs`) is explicitly launched **ONLY from the Branch View** when a Jules branch (🦑) is highlighted.
*   **Overlay Rendering:** It opens as a full-screen overlay/modal over the Workflow View (not as a cramped 3rd pane).
*   **Performance Limit:** It only loads and displays the **last 7 messages** by default to ensure maximum performance and UI responsiveness.

## 4. Chat Layout Rules (Codex-rs Inspired)

Within the `cli-chat-rs` UI, messages must be formatted based on their source/type:

*   **AI Messages:** Left-aligned message bubbles.
*   **User Messages:** Right-aligned message bubbles.
*   **System/Action Logs:** Centered, full-width blocks (e.g., "Terminal outputs", "File modifications", status updates).
*   **Special Exception (Jules Plans):** When the Jules API sends a "Plan/Todo list" activity, the adapter will parse it, and `cli-chat-rs` will render it as a structured **Tree/List layout**, distinct from normal text bubbles.
