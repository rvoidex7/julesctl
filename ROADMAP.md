# julesctl Roadmap & Architectural Manifesto

## Core Philosophy
`julesctl` abandons the traditional conversational chatbot model for an AI coding assistant. Instead, it adopts a **Git-first Workflow paradigm**.
- Jules AI sessions are treated as **Git Branches**.
- The project relies entirely on a 'Workflow/Branch' model for session orchestration. 'Single mode' management has been deprecated to avoid confusion.
- The CLI acts as a visual orchestrator (TUI) to map, branch, merge, cherry-pick, and track AI-generated code.
- A highly polished, professional developer experience prioritizing speed, keyboard shortcuts, and native terminal feel over web UIs.

## Architectural Guidelines
1. **Separation of Concerns:**
   - `julesctl` manages orchestration, session creation, and Git tree visualization.
   - `cli-chat-rs` remains a completely decoupled, generic `ratatui`-based TUI framework for messaging and must NOT contain any `julesctl`-specific logic.
2. **UI Framework & Navigation:**
   - Built with `ratatui`. Prioritize simplicity, avoid over-engineering.
   - **Top-Level Tabs:** A web-browser-like tabbed interface at the very top to manage multiple project workflows concurrently without a separate home screen.
   - **2-Pane Dashboard Layout:**
     - **Left Pane (Viewer):** Switchable between **Branch View** and **Commit Graph View**. Also switchable between **Workflow-scoped** (only showing AI sessions/branches for this specific workflow) and **Global-scoped** (showing the entire repo's git data).
     - **Right Pane (Details):** Displays the patch/diff preview or commit message of the currently selected node.
3. **Workflow & Session Model:**
   - A single local repository directory can host *multiple* Workflows.
   - Users can branch off from any point (including existing Jules session branches) to spawn new parallel Jules sessions.
4. **Safety & Git Rules:**
   - Direct commits to Jules AI branches (🦑) are strictly forbidden. When checking out a Jules branch to apply local patches or make changes, the system will automatically create a new local working branch.
   - When creating a session via API (`create_session`), the `source_context` field must use a valid remote GitHub URL or be omitted to prevent HTTP 400 Bad Request errors.

## Todo List & Future Features (Execution Phase)

### Phase 1: Core TUI Redesign
- [ ] **Remove Obsolete UI:** Remove the useless top title pane and the flawed single-workflow initialization logic.
- [ ] **Tabbed Interface:** Implement top-level tabs for multiple Workflows.
- [ ] **Dynamic Left Pane:** Implement Tab/Toggle controls within the left pane to switch between Branch/Commit views and Workflow/Global scopes.

### Phase 2: Session Management & Git Integration
- [ ] **Branch Protection:** Enforce rules preventing direct commits to Jules branches (🦑). Implement an automatic local branch creation prompt when checking out an AI branch.
- [ ] **Fix Session Creation (HTTP 400):** Fix `create_session` API calls to strictly use remote GitHub URL formats (e.g., `github.com/owner/repo`) for the `source_context` to avoid HTTP 400 errors.
- [ ] **Deprecate Single Mode:** Strip out all logic related to "Single Mode" or "Manual Mode" that conflicts with the unified Workflow paradigm.

### Phase 3: Meta-Prompting & External Integrations
- [ ] **Dynamic Context Injection:** Fully integrate reading from `~/.config/julesctl/rules/`, `.julesctl/rules.md`, `AGENTS.md`, and `.gsd/context.md` into API prompts.
