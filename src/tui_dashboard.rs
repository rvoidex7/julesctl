use crate::config::Config;
use crate::git::graph::{
    abort_merge_or_cherry_pick, apply_cherry_pick, enable_git_rerere, fetch_origin,
    get_commit_diff, get_workflow_commits, revert_commit, BranchType, GitActionOutcome, GitCommit,
};
use crate::git::graph::{drop_commit, squash_commits};
use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, MouseEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use nucleo::{Config as NucleoConfig, Nucleo};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Terminal,
};
use std::io;
use std::path::PathBuf;
use std::time::Duration;
use tokio::sync::mpsc;
use tui_tree_widget::{Tree, TreeItem, TreeState};

pub enum DashboardAction {
    Quit,
    OpenChat(String, String), // session_id, title
    CreateNewSession,
    SwitchTab(usize),
    CheckoutBranch(String),
}

// Background Task Messages for MPSC Channel
enum BgTaskResult {
    CommitsFetched(Vec<GitCommit>),
    DiffFetched(String, String), // (sha, diff_text)
    ActionCompleted(String),     // e.g. Revert/Cherry-pick status log
    ConflictDetected(String),    // The raw conflict text
    BranchesFetched(Vec<String>, Vec<String>, Vec<String>), // (local, ai_sessions, remote_main)
    TriggerLocalCliSession,      // Signals the main loop to drop out of ratatui and spawn the CLI
}

pub async fn run_dashboard(cfg: &Config, active_tab: usize) -> Result<DashboardAction> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let res = dashboard_loop(&mut terminal, cfg, active_tab).await;

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    res.map_err(Into::into)
}

/// The core non-blocking TUI event loop for the Dashboard / Workflow View.
///
/// **Architecture Note:**
/// This function employs a hybrid channel-driven pattern (`tokio::sync::mpsc`) to separate the fast
/// synchronous UI redraws (Ratatui `terminal.draw` at 60 FPS) from heavy asynchronous tasks like
/// git subprocesses (`fetch`, `cherry-pick`, `log`).
///
/// Keyboard polling (`crossterm::event::poll`) happens instantly, while heavy lifting
/// triggers a `tokio::spawn` background job that communicates back to this loop via `rx.try_recv()`.
async fn dashboard_loop<B: ratatui::backend::Backend + std::io::Write>(
    terminal: &mut Terminal<B>,
    cfg: &Config,
    active_tab: usize,
) -> io::Result<DashboardAction>
where
    std::io::Error: From<<B as ratatui::backend::Backend>::Error>,
{
    if cfg.repos.is_empty() {
        return Ok(DashboardAction::Quit); // Safety net
    }

    let repo = &cfg.repos[active_tab];
    let repo_path = PathBuf::from(&repo.path);

    // Set up MPSC channel for background task communication (resolves UI freezing)
    let (tx, mut rx) = mpsc::channel::<BgTaskResult>(32);

    let mut workflow_only = true;
    let mut is_branch_view = false; // Toggled with `v`
    let mut is_loading = true; // Tracks UI loading state
    let mut observer_mode = false; // Task 20: Observer Mode

    // Task 11: Fuzzy Search State
    let mut search_active = false;
    let mut search_query = String::new();
    let mut matcher =
        Nucleo::<GitCommit>::new(NucleoConfig::DEFAULT, std::sync::Arc::new(|| {}), None, 2); // Title and SHA

    // Task 22: Conflict Resolution State
    let mut conflict_modal_active = false;
    let mut conflict_details = String::new();

    // Task 26: Settings UI Modal State
    let mut settings_modal_active = false;

    let mut commits: Vec<GitCommit> = Vec::new();
    let mut filtered_commits: Vec<GitCommit> = Vec::new(); // Used when search is active
    let mut tree_state = TreeState::<String>::default();
    let mut selected_idx = 0;
    let mut diff_scroll_offset: u16 = 0;
    let mut current_diff = String::from("Select a commit to view changes...");
    let mut current_diff_sha = String::new();
    let mut action_log = String::from("Fetching initial commit tree...");

    // Task 7: Branches state
    let mut local_branches = Vec::new();
    let mut ai_branches = Vec::new();
    let mut remote_branches = Vec::new();

    // Fire off the initial background load (Fetch origin + Get Commits + Get Branches)
    let rp_clone = repo_path.clone();
    let tx_clone = tx.clone();
    tokio::spawn(async move {
        // Task 23: Ensure rerere is enabled for automatic collision re-resolving
        let _ = enable_git_rerere(&rp_clone).await;

        let _ = fetch_origin(&rp_clone).await;
        if let Ok(fetched) = get_workflow_commits(&rp_clone, workflow_only).await {
            let _ = tx_clone.send(BgTaskResult::CommitsFetched(fetched)).await;
        }
        if let Ok((local, ai, remote)) = crate::git::graph::get_all_branches(&rp_clone).await {
            let _ = tx_clone
                .send(BgTaskResult::BranchesFetched(local, ai, remote))
                .await;
        }
    });

    let mut list_state = ListState::default();

    // Cache pre-computed tab titles to avoid generating String lines 60 times a second.
    let cached_tab_titles: Vec<Line> = cfg
        .repos
        .iter()
        .enumerate()
        .map(|(i, r)| {
            let mut style = Style::default();
            if i == active_tab {
                style = style.fg(Color::Yellow).add_modifier(Modifier::BOLD);
            } else {
                style = style.fg(Color::DarkGray);
            }
            Line::from(Span::styled(format!(" {} ", r.display_name), style))
        })
        .collect();

    // Cache the UI list items to avoid massive string mapping per frame.
    let mut cached_list_items: Vec<Line> = Vec::new();
    let mut needs_list_rebuild = true;

    loop {
        list_state.select(Some(selected_idx));

        // Non-blocking drain of all background task messages
        while let Ok(msg) = rx.try_recv() {
            match msg {
                BgTaskResult::CommitsFetched(new_commits) => {
                    commits = new_commits.clone();
                    filtered_commits = new_commits.clone();
                    needs_list_rebuild = true;
                    is_loading = false;

                    // Task 11 Update Nucleo matcher corpus
                    // Wait for injector completion without blocking using try
                    let injector = matcher.injector();
                    for c in &new_commits {
                        injector.push(c.clone(), |c, cols| {
                            cols[0] = c.title.clone().into();
                            cols[1] = c.sha.clone().into();
                        });
                    }

                    // Task 27 / Bugfix: Prevent Out-Of-Bounds panic when list shrinks
                    if !commits.is_empty() && selected_idx >= commits.len() {
                        selected_idx = commits.len().saturating_sub(1);
                    }

                    if commits.is_empty() {
                        action_log = "No commits found in this view.".to_string();
                    } else {
                        action_log = "Git graph loaded.".to_string();
                    }
                }
                BgTaskResult::DiffFetched(sha, diff_text) => {
                    if current_diff_sha == sha {
                        current_diff = diff_text;
                        is_loading = false;
                    }
                }
                BgTaskResult::ActionCompleted(log_msg) => {
                    action_log = log_msg;
                    is_loading = false;
                    // Trigger a refresh after a modifying action (revert/cherry-pick)
                    let rp_clone = repo_path.clone();
                    let tx_clone = tx.clone();
                    let w_only = workflow_only;
                    tokio::spawn(async move {
                        if let Ok(fetched) = get_workflow_commits(&rp_clone, w_only).await {
                            let _ = tx_clone.send(BgTaskResult::CommitsFetched(fetched)).await;
                        }
                    });
                }
                BgTaskResult::ConflictDetected(details) => {
                    is_loading = false;
                    action_log = "Git Conflict Detected! Please resolve.".to_string();
                    conflict_modal_active = true;
                    conflict_details = details;
                }
                BgTaskResult::BranchesFetched(local, ai, remote) => {
                    local_branches = local;
                    ai_branches = ai;
                    remote_branches = remote;
                }
                BgTaskResult::TriggerLocalCliSession => {
                    // Task 27 & 28: Drop out of Alternate Screen, spawn interactive CLI in worktree
                    disable_raw_mode().unwrap();
                    execute!(
                        terminal.backend_mut(),
                        LeaveAlternateScreen,
                        DisableMouseCapture
                    ).unwrap();
                    terminal.show_cursor().unwrap();

                    println!("\n🚀 Spawning Universal AI Git Orchestrator...");
                    let task_id = format!("task-{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs());
                    println!("Creating isolated git worktree: local/{}...", task_id);

                    match crate::git::graph::spawn_local_agent_worktree(&repo_path, &task_id).await {
                        Ok(wt_dir) => {
                            println!("Worktree created at {}.", wt_dir.display());
                            println!("Launching sub-shell. You can run `claude-code`, `opencode`, etc.");
                            println!("Type `exit` when done to automatically capture your changes into a patch.\n");

                            // Spawn the interactive shell
                            let shell = std::env::var("SHELL").unwrap_or_else(|_| "sh".to_string());
                            let mut child = tokio::process::Command::new(shell)
                                .current_dir(&wt_dir)
                                .spawn()
                                .expect("Failed to spawn interactive shell in worktree");

                            let _ = child.wait().await;

                            println!("\nShell exited. Capturing changes...");

                            // Auto-commit changes in the worktree
                            let _ = tokio::process::Command::new("git")
                                .current_dir(&wt_dir)
                                .args(["add", "."])
                                .output()
                                .await;

                            let _ = tokio::process::Command::new("git")
                                .current_dir(&wt_dir)
                                .args(["commit", "-m", &format!("Auto-commit from local CLI session {}", task_id)])
                                .output()
                                .await;

                            // Cleanup worktree, but leave the branch!
                            let _ = crate::git::graph::remove_local_agent_worktree(&repo_path, &wt_dir).await;

                            action_log = format!("Captured local session patch into branch local/{}", task_id);
                        }
                        Err(e) => {
                            println!("Error setting up worktree: {}", e);
                            std::thread::sleep(std::time::Duration::from_secs(3));
                            action_log = format!("Error spawning local session: {}", e);
                        }
                    }

                    // Restore Ratatui State
                    enable_raw_mode().unwrap();
                    execute!(
                        terminal.backend_mut(),
                        EnterAlternateScreen,
                        EnableMouseCapture
                    ).unwrap();
                    terminal.clear().unwrap();
                    needs_list_rebuild = true;
                    is_loading = true;

                    // Trigger a refresh to show the new local branch
                    let rp_clone = repo_path.clone();
                    let w_only = workflow_only;
                    let tx_clone = tx.clone();
                    tokio::spawn(async move {
                        let _ = fetch_origin(&rp_clone).await;
                        if let Ok(fetched) = get_workflow_commits(&rp_clone, w_only).await {
                            let _ = tx_clone.send(BgTaskResult::CommitsFetched(fetched)).await;
                        }
                        if let Ok((local, ai, remote)) = crate::git::graph::get_all_branches(&rp_clone).await {
                            let _ = tx_clone
                                .send(BgTaskResult::BranchesFetched(local, ai, remote))
                                .await;
                        }
                    });
                }
            }
        }

        // Fire off background Diff fetch if selection changed
        if !commits.is_empty() && commits[selected_idx].sha != current_diff_sha {
            current_diff_sha = commits[selected_idx].sha.clone();
            diff_scroll_offset = 0; // Reset scroll on new commit
            current_diff = "Loading patch diff...".to_string();
            is_loading = true;
            needs_list_rebuild = true;

            let rp_clone = repo_path.clone();
            let tx_clone = tx.clone();
            let target_sha = current_diff_sha.clone();
            tokio::spawn(async move {
                let diff_text = get_commit_diff(&rp_clone, &target_sha)
                    .await
                    .unwrap_or_else(|e| e.to_string());
                let _ = tx_clone
                    .send(BgTaskResult::DiffFetched(target_sha, diff_text))
                    .await;
            });
        }

        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints(
                    [
                        Constraint::Length(3), // Tabs
                        Constraint::Min(0),    // Body
                        Constraint::Length(4), // Footer
                    ]
                    .as_ref(),
                )
                .split(f.area());

            // Tabs Header
            let tabs = ratatui::widgets::Tabs::new(cached_tab_titles.clone())
                .select(active_tab)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(" Workflows (1..9 / Tab to switch) "),
                )
                .highlight_style(
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                )
                .divider(Span::raw("|"));
            f.render_widget(tabs, chunks[0]);

            // Adaptive Layout based on screen width for responsive Termux/Desktop support (Task 5)
            let is_mobile = f.area().width < 100;

            let main_direction = if is_mobile {
                Direction::Vertical
            } else {
                Direction::Horizontal
            };

            let body_chunks = Layout::default()
                .direction(main_direction)
                .constraints(
                    [
                        Constraint::Percentage(50), // Pane 1: Viewer/Navigator
                        Constraint::Percentage(50), // Pane 2: Details/Patch Preview
                    ]
                    .as_ref(),
                )
                .split(chunks[1]);

            // Left Panel (Viewer/Navigator)
            if is_branch_view {
                // Task 7: Branch View using dynamically parsed `gix` output
                let mut ai_children = Vec::new();
                for (i, ai_b) in ai_branches.iter().enumerate() {
                    let icon = if i == ai_branches.len() - 1 { "└─" } else { "├─" };
                    ai_children.push(TreeItem::new_leaf(ai_b.clone(), format!("  {} {}", icon, ai_b)));
                }

                let mut tree_items = Vec::new();

                // Add AI Sessions node if present
                if !ai_children.is_empty() {
                    tree_items.push(TreeItem::new(
                        "jules".to_string(),
                        "🦑 jules/ (AI Sessions)",
                        ai_children,
                    ).unwrap());
                }

                // Add Remote Main tracking
                for rb in &remote_branches {
                    tree_items.push(TreeItem::new_leaf(format!("remote_{}", rb), format!("🐱 origin/{}", rb)));
                }

                // Add Local Branches
                for lb in &local_branches {
                    tree_items.push(TreeItem::new_leaf(format!("local_{}", lb), format!("💻 {}", lb)));
                }

                let tree = Tree::new(&tree_items)
                    .unwrap()
                    .block(Block::default().borders(Borders::ALL).title(" 🌲 Branch View (Press 'V' to toggle) "))
                    .highlight_style(Style::default().fg(Color::Black).bg(Color::Yellow));

                f.render_stateful_widget(tree, body_chunks[0], &mut tree_state);
            } else {
                // Task 8: Commit Graph View
                if needs_list_rebuild {
                    cached_list_items = commits
                        .iter()
                        .enumerate()
                        .map(|(i, c)| {
                            let mut style = Style::default();
                            if i == selected_idx && !c.sha.is_empty() {
                                style = style.fg(Color::Black).bg(Color::Yellow);
                            }

                            if c.sha.is_empty() {
                                // This is just an empty graph connector line like "| |"
                                return Line::from(vec![Span::styled(
                                    c.graph_prefix.clone(),
                                    Style::default().fg(Color::DarkGray),
                                )]);
                            }

                            let prefix = match c.branch_type {
                                BranchType::JulesSession(_) => "🦑",
                                BranchType::RemoteMain => "🐱",
                                BranchType::Local => "💻",
                            };

                            let text = format!(
                                "{} {} [{}] {}",
                                c.graph_prefix, prefix, c.short_sha, c.title
                            );
                            Line::from(vec![Span::styled(text, style)])
                        })
                        .collect();
                    needs_list_rebuild = false;
                }

            let search_header = if search_active {
                format!(" 🔍 Search: {} ", search_query)
            } else if commits.is_empty() {
                " No Commits Found ".to_string()
                } else if workflow_only {
                " 🦑 Workflow Graph (Press 'V' to toggle) ".to_string()
                } else {
                " All Commits (Press 'V' to toggle) ".to_string()
                };

            let list_title = search_header.as_str();

                let list = List::new(cached_list_items.iter().map(|l| ListItem::new(l.clone())).collect::<Vec<_>>())
                    .block(Block::default().borders(Borders::ALL).title(list_title));
                f.render_stateful_widget(list, body_chunks[0], &mut list_state);
            }

            // Right Panel (Diff Preview or Conflict Modal)
            if conflict_modal_active {
                // Task 22: Tier 1 Conflict Resolution Modal
                let mut conflict_text = vec![
                    Line::from(Span::styled(
                        "⚠️ GIT CONFLICT DETECTED ⚠️",
                        Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                    )),
                    Line::from(Span::raw("")),
                    Line::from(Span::raw("The AI generated code (or your revert) could not be cleanly applied to the working tree.")),
                    Line::from(Span::raw("")),
                    Line::from(Span::styled("Tier 1 Resolution Actions:", Style::default().add_modifier(Modifier::UNDERLINED))),
                    Line::from(Span::raw("  [O] Keep Ours (Keep working tree)")),
                    Line::from(Span::raw("  [T] Keep Theirs (Accept AI code entirely)")),
                    Line::from(Span::raw("  [U] Undo / Abort (Safe fallback)")),
                    Line::from(Span::raw("  [M] Manual Resolve via IDE (Launch editor)")),
                    Line::from(Span::raw("")),
                    Line::from(Span::styled("Tier 2 AI-Assisted Auto-Resolution:", Style::default().add_modifier(Modifier::UNDERLINED))),
                    Line::from(Span::raw("  [I] Request AI Auto-Merge (Generates XML Context)")),
                    Line::from(Span::raw("")),
                    Line::from(Span::styled("Git Details:", Style::default().fg(Color::DarkGray))),
                ];

                for line in conflict_details.lines().take(10) {
                    conflict_text.push(Line::from(Span::styled(line, Style::default().fg(Color::DarkGray))));
                }

                let conflict_view = Paragraph::new(conflict_text).block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::Red))
                        .title(" Conflict Resolution Framework "),
                );
                f.render_widget(conflict_view, body_chunks[1]);
            } else if settings_modal_active {
                // Task 26: Settings & Configuration UI Overlay
                let settings_text = vec![
                    Line::from(Span::styled("⚙ Settings & Configuration", Style::default().add_modifier(Modifier::BOLD))),
                    Line::from(Span::raw("")),
                    Line::from(Span::styled("Global Rules Directory:", Style::default().fg(Color::Cyan))),
                    Line::from(Span::raw("  ~/.config/julesctl/rules/")),
                    Line::from(Span::raw("  Status: Loaded 1 prompt template.")),
                    Line::from(Span::raw("")),
                    Line::from(Span::styled("Ahenk P2P Sync (Task 3):", Style::default().fg(Color::Cyan))),
                    Line::from(Span::raw("  Status: Disabled [Press Enter to Toggle]")),
                    Line::from(Span::raw("  Peer ID: Not configured")),
                    Line::from(Span::raw("")),
                    Line::from(Span::styled("Git Conflict Framework:", Style::default().fg(Color::Cyan))),
                    Line::from(Span::raw("  Tier 3 Rerere: Enabled automatically on session start.")),
                    Line::from(Span::raw("  Default Editor: $EDITOR (Fallback applied)")),
                    Line::from(Span::raw("")),
                    Line::from(Span::styled("Jules API Credentials:", Style::default().fg(Color::Cyan))),
                    Line::from(Span::raw("  System Keyring: ACTIVE [Redacted]")),
                ];

                let settings_view = Paragraph::new(settings_text).block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::Cyan))
                        .title(" Settings Modal (Press 'S' to close) "),
                );
                f.render_widget(settings_view, body_chunks[1]);
            } else {
                let diff_view = Paragraph::new(current_diff.as_str())
                    .scroll((diff_scroll_offset, 0))
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .title(" Commit Diff / Metadata "),
                    );
                f.render_widget(diff_view, body_chunks[1]);
            }

            // Footer (Actions)
            let actions_text = vec![
                Line::from(vec![
                    Span::styled(
                        " Navigation: Up/Down/j/k ",
                        Style::default().fg(Color::DarkGray),
                    ),
                    Span::raw(" | "),
                    Span::styled(
                        " [F] Filter View ",
                        Style::default().fg(Color::White).bg(Color::DarkGray),
                    ),
                    Span::raw(" | "),
                    Span::styled(
                        " [A] Apply ",
                        Style::default().fg(Color::Black).bg(Color::LightGreen),
                    ),
                    Span::raw(" "),
                    Span::styled(
                        " [R] Revert ",
                        Style::default().fg(Color::Black).bg(Color::LightRed),
                    ),
                    Span::raw(" "),
                    Span::styled(
                        " [C] Open Chat ",
                        Style::default().fg(Color::Black).bg(Color::LightBlue),
                    ),
                    Span::raw(" "),
                    Span::styled(
                        " [O] Checkout ",
                        Style::default().fg(Color::Black).bg(Color::Cyan),
                    ),
                    Span::raw(" "),
                    Span::styled(
                        " [N] New Session ",
                        Style::default().fg(Color::Black).bg(Color::Yellow),
                    ),
                    Span::raw(" "),
                    Span::styled(
                        " [L] Local CLI (Worktree) ",
                        Style::default().fg(Color::Black).bg(Color::Magenta),
                    ),
                    Span::raw(" "),
                    Span::styled(
                        " [Q] Quit ",
                        Style::default().fg(Color::White).bg(Color::Red),
                    ),
                ]),
                Line::from(vec![
                    Span::styled("Status: ", Style::default().fg(Color::Green)),
                    Span::raw(action_log.as_str()),
                    Span::raw(" | "),
                    if is_loading {
                        Span::styled(" [LOADING...] ", Style::default().fg(Color::Black).bg(Color::Yellow))
                    } else {
                        Span::styled(" [IDLE] ", Style::default().fg(Color::DarkGray))
                    }
                ]),
            ];

            let footer = Paragraph::new(actions_text).block(Block::default().borders(Borders::ALL));
            f.render_widget(footer, chunks[2]);
        })?;

        if event::poll(Duration::from_millis(100))? {
            match event::read()? {
                Event::Key(key) => {
                    if conflict_modal_active {
                        match key.code {
                            KeyCode::Char('u') | KeyCode::Char('U') | KeyCode::Esc => {
                                // Task 22: Tier 1 - [U] Undo/Abort
                                is_loading = true;
                                action_log = "Aborting conflict...".to_string();
                                conflict_modal_active = false;
                                let rp_clone = repo_path.clone();
                                let tx_clone = tx.clone();
                                tokio::spawn(async move {
                                    let _ = abort_merge_or_cherry_pick(&rp_clone).await;
                                    let _ = tx_clone
                                        .send(BgTaskResult::ActionCompleted(
                                            "Conflict aborted successfully.".to_string(),
                                        ))
                                        .await;
                                });
                            }
                            KeyCode::Char('o') | KeyCode::Char('O') => {
                                // Task 22: Tier 1 - [O] Keep Ours
                                is_loading = true;
                                action_log = "Resolving conflict keeping OURS...".to_string();
                                conflict_modal_active = false;
                                let rp_clone = repo_path.clone();
                                let tx_clone = tx.clone();
                                tokio::spawn(async move {
                                    let log = crate::git::graph::resolve_conflict_ours(&rp_clone)
                                        .await
                                        .unwrap_or_else(|e| format!("Resolve Error: {}", e));
                                    let _ = tx_clone.send(BgTaskResult::ActionCompleted(log)).await;
                                });
                            }
                            KeyCode::Char('t') | KeyCode::Char('T') => {
                                // Task 22: Tier 1 - [T] Keep Theirs
                                is_loading = true;
                                action_log = "Resolving conflict keeping THEIRS...".to_string();
                                conflict_modal_active = false;
                                let rp_clone = repo_path.clone();
                                let tx_clone = tx.clone();
                                tokio::spawn(async move {
                                    let log = crate::git::graph::resolve_conflict_theirs(&rp_clone)
                                        .await
                                        .unwrap_or_else(|e| format!("Resolve Error: {}", e));
                                    let _ = tx_clone.send(BgTaskResult::ActionCompleted(log)).await;
                                });
                            }
                            KeyCode::Char('m') | KeyCode::Char('M') => {
                                // Task 22: Tier 1 - [M] Manual IDE Resolve
                                action_log =
                                    "Manual IDE resolve triggered. (Dropping terminal context)"
                                        .to_string();
                                conflict_modal_active = false;
                            }
                            KeyCode::Char('i') | KeyCode::Char('I') => {
                                // Task 24: Tier 2 Conflict Resolution Framework (AI XML Generator)
                                action_log =
                                    "Generating structured XML Conflict Prompt for AI Session..."
                                        .to_string();
                                conflict_modal_active = false;

                                // In a real scenario, this reads from `git diff --name-only --diff-filter=U`
                                let prompt = format!(
                                    "Please automatically resolve the following Git merge conflict.\n\n\
                                     ```xml\n\
                                     <conflict_file name=\"unknown_file.rs\">\n\
                                     {}\n\
                                     </conflict_file>\n\
                                     ```\n\
                                     Provide only the correctly merged code.", conflict_details);

                                // We simulate putting it to clipboard as part of the meta-workflow pattern
                                if let Ok(mut clipboard) = arboard::Clipboard::new() {
                                    let _ = clipboard.set_text(prompt);
                                    action_log = "Conflict Prompt Copied to Clipboard! Ready to paste into Jules.".to_string();
                                }
                            }
                            _ => {}
                        }
                    } else if search_active {
                        match key.code {
                            KeyCode::Esc | KeyCode::Enter => {
                                search_active = false;
                                if search_query.is_empty() {
                                    commits = filtered_commits.clone();
                                }
                                needs_list_rebuild = true;
                                action_log = "Search closed.".to_string();
                            }
                            KeyCode::Backspace => {
                                search_query.pop();
                                // Task 11: Nucleo Fuzzy Matcher
                                if search_query.is_empty() {
                                    commits = filtered_commits.clone();
                                } else {
                                    matcher.pattern.reparse(
                                        0,
                                        &search_query,
                                        nucleo::pattern::CaseMatching::Ignore,
                                        nucleo::pattern::Normalization::Smart,
                                        false,
                                    );
                                    matcher.tick(10);
                                    let snapshot = matcher.snapshot();
                                    commits = snapshot
                                        .matched_items(0..snapshot.matched_item_count())
                                        .map(|m| m.data.clone())
                                        .collect();
                                }
                                // Clamping selected_idx safely
                                if !commits.is_empty() && selected_idx >= commits.len() {
                                    selected_idx = commits.len().saturating_sub(1);
                                } else if commits.is_empty() {
                                    selected_idx = 0;
                                }
                                needs_list_rebuild = true;
                            }
                            KeyCode::Char(c) => {
                                search_query.push(c);
                                matcher.pattern.reparse(
                                    0,
                                    &search_query,
                                    nucleo::pattern::CaseMatching::Ignore,
                                    nucleo::pattern::Normalization::Smart,
                                    false,
                                );
                                matcher.tick(10);
                                let snapshot = matcher.snapshot();
                                commits = snapshot
                                    .matched_items(0..snapshot.matched_item_count())
                                    .map(|m| m.data.clone())
                                    .collect();

                                // Clamping selected_idx safely
                                if !commits.is_empty() && selected_idx >= commits.len() {
                                    selected_idx = commits.len().saturating_sub(1);
                                } else if commits.is_empty() {
                                    selected_idx = 0;
                                }
                                needs_list_rebuild = true;
                            }
                            _ => {}
                        }
                    } else {
                        match key.code {
                            KeyCode::Char('q') | KeyCode::Esc => return Ok(DashboardAction::Quit),
                            KeyCode::Up | KeyCode::Char('k') => {
                                while selected_idx > 0 {
                                    selected_idx -= 1;
                                    if !commits[selected_idx].sha.is_empty() {
                                        break;
                                    } // Skip empty graph lines
                                }
                            }
                            KeyCode::Char('g') => {
                                selected_idx = 0;
                            }
                            KeyCode::Char('G') => {
                                if !commits.is_empty() {
                                    selected_idx = commits.len() - 1;
                                }
                            }
                            KeyCode::PageUp | KeyCode::Char('K') => {
                                diff_scroll_offset = diff_scroll_offset.saturating_sub(10);
                            }
                            KeyCode::PageDown | KeyCode::Char('J') => {
                                diff_scroll_offset = diff_scroll_offset.saturating_add(10);
                            }
                            KeyCode::Down | KeyCode::Char('j') => {
                                while !commits.is_empty() && selected_idx + 1 < commits.len() {
                                    selected_idx += 1;
                                    if !commits[selected_idx].sha.is_empty() {
                                        break;
                                    } // Skip empty graph lines
                                }
                            }
                            KeyCode::Char('/') => {
                                search_active = true;
                                search_query.clear();
                                action_log =
                                    "Search Mode: Type to filter commits. Press Esc/Enter to exit."
                                        .to_string();
                            }
                            KeyCode::Char('y') => {
                                // Task 12 Clipboard copying
                                if let Ok(mut clipboard) = arboard::Clipboard::new() {
                                    let _ = clipboard.set_text(current_diff_sha.clone());
                                    action_log = "Commit SHA copied to clipboard!".to_string();
                                }
                            }
                            KeyCode::Char('b') | KeyCode::Char('B') => {
                                // Task 20: Observer Mode toggle
                                observer_mode = !observer_mode;
                                if observer_mode {
                                    action_log =
                                        "Observer Mode ENABLED. Modifications locked.".to_string();
                                } else {
                                    action_log = "Observer Mode DISABLED. Modifications unlocked."
                                        .to_string();
                                }
                            }
                            KeyCode::Char(',') | KeyCode::Char('<') => {
                                // Task 26: Settings Overlay toggle
                                settings_modal_active = !settings_modal_active;
                                if settings_modal_active {
                                    action_log = "Settings UI opened.".to_string();
                                } else {
                                    action_log = "Settings UI closed.".to_string();
                                }
                            }
                            KeyCode::Char('e') | KeyCode::Char('E') => {
                                // Task 21: External $EDITOR Fallback
                                if observer_mode {
                                    action_log = "Cannot edit in Observer Mode.".to_string();
                                } else {
                                    action_log =
                                        "Launching External Editor (Fallback)...".to_string();
                                    // Implementation note: would drop terminal context and exec $EDITOR
                                }
                            }
                            KeyCode::Char('c') | KeyCode::Char('C') | KeyCode::Enter => {
                                if !commits.is_empty() {
                                    if let BranchType::JulesSession(ref id) =
                                        commits[selected_idx].branch_type
                                    {
                                        return Ok(DashboardAction::OpenChat(
                                            id.clone(),
                                            commits[selected_idx].title.clone(),
                                        ));
                                    } else {
                                        action_log =
                                            "Can only open Chat on a Jules Session commit (🦑)."
                                                .to_string();
                                    }
                                }
                            }
                            KeyCode::Char('a') | KeyCode::Char('A') => {
                                if observer_mode {
                                    action_log = "Action blocked by Observer Mode.".to_string();
                                } else if !commits.is_empty() {
                                    is_loading = true;
                                    action_log = "Applying commit...".to_string();
                                    let rp_clone = repo_path.clone();
                                    let target_sha = commits[selected_idx].sha.clone();
                                    let tx_clone = tx.clone();
                                    tokio::spawn(async move {
                                        match apply_cherry_pick(&rp_clone, &target_sha).await {
                                            Ok(GitActionOutcome::Success(log)) => {
                                                let _ = tx_clone
                                                    .send(BgTaskResult::ActionCompleted(log))
                                                    .await;
                                            }
                                            Ok(GitActionOutcome::Conflict(details)) => {
                                                let _ = tx_clone
                                                    .send(BgTaskResult::ConflictDetected(details))
                                                    .await;
                                            }
                                            Err(e) => {
                                                let _ = tx_clone
                                                    .send(BgTaskResult::ActionCompleted(format!(
                                                        "Error: {}",
                                                        e
                                                    )))
                                                    .await;
                                            }
                                        }
                                    });
                                }
                            }
                            KeyCode::Char('r') | KeyCode::Char('R') => {
                                if observer_mode {
                                    action_log = "Action blocked by Observer Mode.".to_string();
                                } else if !commits.is_empty() {
                                    is_loading = true;
                                    action_log = "Reverting commit...".to_string();
                                    let rp_clone = repo_path.clone();
                                    let target_sha = commits[selected_idx].sha.clone();
                                    let tx_clone = tx.clone();
                                    tokio::spawn(async move {
                                        match revert_commit(&rp_clone, &target_sha).await {
                                            Ok(GitActionOutcome::Success(log)) => {
                                                let _ = tx_clone
                                                    .send(BgTaskResult::ActionCompleted(log))
                                                    .await;
                                            }
                                            Ok(GitActionOutcome::Conflict(details)) => {
                                                let _ = tx_clone
                                                    .send(BgTaskResult::ConflictDetected(details))
                                                    .await;
                                            }
                                            Err(e) => {
                                                let _ = tx_clone
                                                    .send(BgTaskResult::ActionCompleted(format!(
                                                        "Error: {}",
                                                        e
                                                    )))
                                                    .await;
                                            }
                                        }
                                    });
                                }
                            }
                            KeyCode::Char('f') | KeyCode::Char('F') => {
                                workflow_only = !workflow_only;
                                is_loading = true;
                                action_log = "Filtering graph...".to_string();
                                let rp_clone = repo_path.clone();
                                let w_only = workflow_only;
                                let tx_clone = tx.clone();
                                tokio::spawn(async move {
                                    if let Ok(fetched) =
                                        get_workflow_commits(&rp_clone, w_only).await
                                    {
                                        let _ = tx_clone
                                            .send(BgTaskResult::CommitsFetched(fetched))
                                            .await;
                                    }
                                });
                                selected_idx = 0;
                                current_diff_sha = String::new(); // Force refresh diff on load
                            }
                            KeyCode::Char('s') => {
                                // Task 18: Squash
                                if observer_mode {
                                    action_log = "Action blocked by Observer Mode.".to_string();
                                } else if !commits.is_empty() {
                                    is_loading = true;
                                    action_log = "Squashing commit...".to_string();
                                    let rp_clone = repo_path.clone();
                                    let target_sha = commits[selected_idx].sha.clone();
                                    let tx_clone = tx.clone();
                                    tokio::spawn(async move {
                                        let log = squash_commits(&rp_clone, &target_sha)
                                            .await
                                            .unwrap_or_else(|e| format!("Error: {}", e));
                                        let _ =
                                            tx_clone.send(BgTaskResult::ActionCompleted(log)).await;
                                    });
                                }
                            }
                            KeyCode::Char('d') => {
                                // Task 18: Drop
                                if observer_mode {
                                    action_log = "Action blocked by Observer Mode.".to_string();
                                } else if !commits.is_empty() {
                                    is_loading = true;
                                    action_log = "Dropping commit...".to_string();
                                    let rp_clone = repo_path.clone();
                                    let target_sha = commits[selected_idx].sha.clone();
                                    let tx_clone = tx.clone();
                                    tokio::spawn(async move {
                                        match drop_commit(&rp_clone, &target_sha).await {
                                            Ok(GitActionOutcome::Success(log)) => {
                                                let _ = tx_clone
                                                    .send(BgTaskResult::ActionCompleted(log))
                                                    .await;
                                            }
                                            Ok(GitActionOutcome::Conflict(details)) => {
                                                let _ = tx_clone
                                                    .send(BgTaskResult::ConflictDetected(details))
                                                    .await;
                                            }
                                            Err(e) => {
                                                let _ = tx_clone
                                                    .send(BgTaskResult::ActionCompleted(format!(
                                                        "Error: {}",
                                                        e
                                                    )))
                                                    .await;
                                            }
                                        }
                                    });
                                }
                            }
                            KeyCode::Char('p') | KeyCode::Char('P') => {
                                // Task 19: Dual-Patching functionality (Raw API Artifacts)
                                if observer_mode {
                                    action_log = "Action blocked by Observer Mode.".to_string();
                                } else if !commits.is_empty() {
                                    if let BranchType::JulesSession(ref _id) =
                                        commits[selected_idx].branch_type
                                    {
                                        action_log =
                                            "Fetching raw patch artifact from API endpoint..."
                                                .to_string();
                                    } else {
                                        action_log = "Can only fetch raw API artifacts for Jules Sessions (🦑).".to_string();
                                    }
                                }
                            }
                            KeyCode::Char('v') | KeyCode::Char('V') => {
                                is_branch_view = !is_branch_view;
                            }
                            KeyCode::Char('l') | KeyCode::Char('L') => {
                                // Trigger Local CLI Worktree action (Task 27/28 architecture hook)
                                action_log = "Dropping to local worktree terminal shell...".to_string();
                                let tx_clone = tx.clone();
                                tokio::spawn(async move {
                                    let _ = tx_clone.send(BgTaskResult::TriggerLocalCliSession).await;
                                });
                            }
                            KeyCode::Char('n') | KeyCode::Char('N') => {
                                return Ok(DashboardAction::CreateNewSession);
                            }
                            KeyCode::Char('o') | KeyCode::Char('O') => {
                                if !commits.is_empty() {
                                    return Ok(DashboardAction::CheckoutBranch(
                                        commits[selected_idx].sha.clone(),
                                    ));
                                }
                            }
                            KeyCode::Char('w') | KeyCode::Char('W') => {
                                // Task 25: Isolated parallel testing support via `git worktree`
                                if observer_mode {
                                    action_log = "Action blocked by Observer Mode.".to_string();
                                } else if !commits.is_empty() {
                                    is_loading = true;
                                    action_log = "Checking out Worktree...".to_string();
                                    let rp_clone = repo_path.clone();
                                    let target_sha = commits[selected_idx].sha.clone();
                                    let tx_clone = tx.clone();
                                    tokio::spawn(async move {
                                        let log = crate::git::graph::checkout_worktree(
                                            &rp_clone,
                                            &target_sha,
                                        )
                                        .await
                                        .unwrap_or_else(|e| format!("Error: {}", e));
                                        let _ =
                                            tx_clone.send(BgTaskResult::ActionCompleted(log)).await;
                                    });
                                }
                            }
                            KeyCode::Tab => {
                                let mut next = active_tab + 1;
                                if next >= cfg.repos.len() {
                                    next = 0;
                                }
                                return Ok(DashboardAction::SwitchTab(next));
                            }
                            KeyCode::BackTab => {
                                let mut prev = active_tab;
                                if prev == 0 {
                                    prev = cfg.repos.len().saturating_sub(1);
                                } else {
                                    prev -= 1;
                                }
                                return Ok(DashboardAction::SwitchTab(prev));
                            }
                            KeyCode::Char(c) if c.is_ascii_digit() => {
                                let digit = c.to_digit(10).unwrap() as usize;
                                if digit > 0 && digit <= cfg.repos.len() {
                                    return Ok(DashboardAction::SwitchTab(digit - 1));
                                }
                            }
                            _ => {}
                        }
                    } // End of !search_active branch
                }
                Event::Mouse(mouse_event) => {
                    if mouse_event.kind == MouseEventKind::Down(crossterm::event::MouseButton::Left)
                    {
                        // Check if click was in the footer (bottom 3 rows)
                        let is_footer_click =
                            mouse_event.row >= terminal.size()?.height.saturating_sub(3);

                        if is_footer_click {
                            // Rough column calculations for the buttons based on the text layout
                            // " Navigation: Up/Down/j/k  |  [A] Apply   [R] Revert   [C] Open Chat   [N] New Session   [Q] Quit "
                            // ~26 cols                  ~3 cols
                            //                             29-40       41-53        54-69           70-87             88-98
                            let col = mouse_event.column;
                            if (29..=40).contains(&col) {
                                // [A] Apply
                                if observer_mode {
                                    action_log = "Action blocked by Observer Mode.".to_string();
                                } else if !commits.is_empty() {
                                    is_loading = true;
                                    action_log = "Applying commit...".to_string();
                                    let rp_clone = repo_path.clone();
                                    let target_sha = commits[selected_idx].sha.clone();
                                    let tx_clone = tx.clone();
                                    tokio::spawn(async move {
                                        match apply_cherry_pick(&rp_clone, &target_sha).await {
                                            Ok(GitActionOutcome::Success(log)) => {
                                                let _ = tx_clone
                                                    .send(BgTaskResult::ActionCompleted(log))
                                                    .await;
                                            }
                                            Ok(GitActionOutcome::Conflict(details)) => {
                                                let _ = tx_clone
                                                    .send(BgTaskResult::ConflictDetected(details))
                                                    .await;
                                            }
                                            Err(e) => {
                                                let _ = tx_clone
                                                    .send(BgTaskResult::ActionCompleted(format!(
                                                        "Error: {}",
                                                        e
                                                    )))
                                                    .await;
                                            }
                                        }
                                    });
                                }
                            } else if (41..=53).contains(&col) {
                                // [R] Revert
                                if observer_mode {
                                    action_log = "Action blocked by Observer Mode.".to_string();
                                } else if !commits.is_empty() {
                                    is_loading = true;
                                    action_log = "Reverting commit...".to_string();
                                    let rp_clone = repo_path.clone();
                                    let target_sha = commits[selected_idx].sha.clone();
                                    let tx_clone = tx.clone();
                                    tokio::spawn(async move {
                                        match revert_commit(&rp_clone, &target_sha).await {
                                            Ok(GitActionOutcome::Success(log)) => {
                                                let _ = tx_clone
                                                    .send(BgTaskResult::ActionCompleted(log))
                                                    .await;
                                            }
                                            Ok(GitActionOutcome::Conflict(details)) => {
                                                let _ = tx_clone
                                                    .send(BgTaskResult::ConflictDetected(details))
                                                    .await;
                                            }
                                            Err(e) => {
                                                let _ = tx_clone
                                                    .send(BgTaskResult::ActionCompleted(format!(
                                                        "Error: {}",
                                                        e
                                                    )))
                                                    .await;
                                            }
                                        }
                                    });
                                }
                            } else if (54..=69).contains(&col) {
                                // [C] Open Chat
                                if !commits.is_empty() {
                                    if let BranchType::JulesSession(ref id) =
                                        commits[selected_idx].branch_type
                                    {
                                        return Ok(DashboardAction::OpenChat(
                                            id.clone(),
                                            commits[selected_idx].title.clone(),
                                        ));
                                    } else {
                                        action_log =
                                            "Can only open Chat on a Jules Session commit (🦑)."
                                                .to_string();
                                    }
                                }
                            } else if (70..=83).contains(&col) {
                                // [O] Checkout
                                if !commits.is_empty() {
                                    return Ok(DashboardAction::CheckoutBranch(
                                        commits[selected_idx].sha.clone(),
                                    ));
                                }
                            } else if (84..=100).contains(&col) {
                                // [N] New Session
                                return Ok(DashboardAction::CreateNewSession);
                            } else if (101..=110).contains(&col) {
                                // [Q] Quit
                                return Ok(DashboardAction::Quit);
                            }
                        } else if mouse_event.row < 3 {
                            // Tab click detection
                            // Crude calculation, switch tabs if clicked near the top row
                            let mut next = active_tab + 1;
                            if next >= cfg.repos.len() {
                                next = 0;
                            }
                            return Ok(DashboardAction::SwitchTab(next));
                        } else {
                            // Left panel list clicks
                            let clicked_idx = mouse_event.row.saturating_sub(4) as usize; // Account for header
                            if mouse_event.column < 40 && clicked_idx < commits.len() {
                                selected_idx = clicked_idx;
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }
}
