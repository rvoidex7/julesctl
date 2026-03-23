# julesctl Roadmap & Architectural Manifesto

## Core Philosophy & Manifesto

`julesctl` is NOT a simple terminal chatbot. It completely abandons the traditional conversational AI model. Instead, it adopts a strict **Git-first Workflow paradigm**.

*We are building a highly polished, professional developer tool acting as a visual orchestrator to map, branch, merge, cherry-pick, and track AI-generated code across multiple parallel sessions.*

This document acts as an index to the comprehensive architectural manifesto. Every feature, interaction, and architectural decision listed here must be implemented.

**Explore the modules:**

1.  **Architecture:**
    *   [Core Philosophy](architecture/core-philosophy.md)
    *   [Hybrid Git Engine & Optimizations](architecture/hybrid-git-engine.md)
    *   [Data Management & Ahenk P2P Sync](architecture/data-management-ahenk.md)
2.  **User Interface (TUI):**
    *   [Layout & Navigation](ui/layout-and-navigation.md)
    *   [Scoped Chat Integration & `cli-chat-rs`](ui/scoped-chat-integration.md)
    *   [Settings & Configuration UI](ui/settings-ui.md)
3.  **Git Workflow & Operations:**
    *   [Branching, Synchronization & Safety](git-workflow/branching-and-sync.md)
    *   [The Visual Patch Stack (Catalog Shopping)](git-workflow/visual-patch-stack.md)
4.  **Conflict Resolution:**
    *   [The Conflict Resolution Framework](conflict-resolution/framework.md)