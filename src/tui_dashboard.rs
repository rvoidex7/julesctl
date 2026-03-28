use crate::config::Config;
use crate::git::graph::{
    apply_cherry_pick, fetch_origin, get_commit_diff, get_workflow_commits, revert_commit,
    BranchType, GitCommit,
};
use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, MouseEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Terminal,
};
use tui_tree_widget::{Tree, TreeItem, TreeState};
use std::io;
use std::path::PathBuf;
use std::time::Duration;
use tokio::sync::mpsc;

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
    ActionCompleted(String), // e.g. Revert/Cherry-pick status log
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

async fn dashboard_loop<B: ratatui::backend::Backend>(
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

    // Set up MPSC channel for background task communication
    let (tx, mut rx) = mpsc::channel::<BgTaskResult>(32);

    let mut workflow_only = true;
    let mut is_branch_view = false; // Toggled with `v`
    let mut is_loading = true;      // Tracks UI loading state
    let mut observer_mode = false;  // Task 20: Observer Mode

    let mut commits: Vec<GitCommit> = Vec::new();
    let mut tree_state = TreeState::<String>::default();
    let mut selected_idx = 0;
    let mut diff_scroll_offset: u16 = 0;
    let mut current_diff = String::from("Select a commit to view changes...");
    let mut current_diff_sha = String::new();
    let mut action_log = String::from("Fetching initial commit tree...");

    // Fire off the initial background load (Fetch origin + Get Commits)
    let rp_clone = repo_path.clone();
    let tx_clone = tx.clone();
    tokio::spawn(async move {
        let _ = fetch_origin(&rp_clone).await;
        if let Ok(fetched) = get_workflow_commits(&rp_clone, workflow_only).await {
            let _ = tx_clone.send(BgTaskResult::CommitsFetched(fetched)).await;
        }
    });

    let mut list_state = ListState::default();

    loop {
        list_state.select(Some(selected_idx));

        // Non-blocking drain of all background task messages
        while let Ok(msg) = rx.try_recv() {
            match msg {
                BgTaskResult::CommitsFetched(new_commits) => {
                    commits = new_commits;
                    is_loading = false;
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
            }
        }

        // Fire off background Diff fetch if selection changed
        if !commits.is_empty() && commits[selected_idx].sha != current_diff_sha {
            current_diff_sha = commits[selected_idx].sha.clone();
            diff_scroll_offset = 0; // Reset scroll on new commit
            current_diff = "Loading patch diff...".to_string();
            is_loading = true;

            let rp_clone = repo_path.clone();
            let tx_clone = tx.clone();
            let target_sha = current_diff_sha.clone();
            tokio::spawn(async move {
                let diff_text = get_commit_diff(&rp_clone, &target_sha)
                    .await
                    .unwrap_or_else(|e| e.to_string());
                let _ = tx_clone.send(BgTaskResult::DiffFetched(target_sha, diff_text)).await;
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
            let tab_titles: Vec<Line> = cfg
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

            let tabs = ratatui::widgets::Tabs::new(tab_titles)
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
                // Task 7: Branch View using tui-tree-widget
                let tree_items = vec![
                    TreeItem::new_leaf("main".to_string(), "🐱 origin/main"),
                    TreeItem::new(
                        "jules".to_string(),
                        "🦑 jules/ (AI Sessions)",
                        vec![
                            TreeItem::new_leaf("task1".to_string(), "  ├─ task-1234 (Login Fix)"),
                            TreeItem::new_leaf("task2".to_string(), "  └─ task-5678 (UI Update)"),
                        ],
                    ).unwrap(),
                    TreeItem::new_leaf("local".to_string(), "💻 my-local-branch"),
                ];

                let tree = Tree::new(&tree_items)
                    .unwrap()
                    .block(Block::default().borders(Borders::ALL).title(" Branch View (Press 'V' to toggle) "))
                    .highlight_style(Style::default().fg(Color::Black).bg(Color::Yellow));

                f.render_stateful_widget(tree, body_chunks[0], &mut tree_state);
            } else {
                // Task 8: Commit Graph View
                let items: Vec<ListItem> = commits
                    .iter()
                    .enumerate()
                    .map(|(i, c)| {
                        let mut style = Style::default();
                        if i == selected_idx && !c.sha.is_empty() {
                            style = style.fg(Color::Black).bg(Color::Yellow);
                        }

                        if c.sha.is_empty() {
                            // This is just an empty graph connector line like "| |"
                            return ListItem::new(Line::from(vec![Span::styled(
                                c.graph_prefix.clone(),
                                Style::default().fg(Color::DarkGray),
                            )]));
                        }

                        let prefix = match c.branch_type {
                            BranchType::JulesSession(_) => "🦑",
                            BranchType::RemoteMain => "🐱",
                            BranchType::Local => "💻",
                        };

                        // Format: "| * | 🦑 [abcdef] Commit message"
                        let text = format!(
                            "{} {} [{}] {}",
                            c.graph_prefix, prefix, c.short_sha, c.title
                        );
                        ListItem::new(Line::from(vec![Span::styled(text, style)]))
                    })
                    .collect();

                let list_title = if commits.is_empty() {
                    " No Commits Found "
                } else if workflow_only {
                    " 🦑 Workflow Graph (Press 'V' to toggle) "
                } else {
                    " All Commits (Press 'V' to toggle) "
                };

                let list =
                    List::new(items).block(Block::default().borders(Borders::ALL).title(list_title));
                f.render_stateful_widget(list, body_chunks[0], &mut list_state);
            }

            // Right Panel (Diff Preview)
            let diff_view = Paragraph::new(current_diff.as_str())
                .scroll((diff_scroll_offset, 0))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(" Commit Diff / Metadata "),
                );
            f.render_widget(diff_view, body_chunks[1]);

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
                        " [Q] Quit ",
                        Style::default().fg(Color::White).bg(Color::Red),
                    ),
                ]),
                Line::from(vec![
                    Span::styled("Status: ", Style::default().fg(Color::Green)),
                    Span::raw(action_log.as_str()),
                ]),
            ];

            let footer = Paragraph::new(actions_text).block(Block::default().borders(Borders::ALL));
            f.render_widget(footer, chunks[2]);
        })?;

        if event::poll(Duration::from_millis(100))? {
            match event::read()? {
                Event::Key(key) => match key.code {
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
                        // TODO: Task 11 Fuzzy search placeholder overlay
                        action_log = "Search active. (Fuzzy Finder integration incoming...)".to_string();
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
                            action_log = "Observer Mode ENABLED. Modifications locked.".to_string();
                        } else {
                            action_log = "Observer Mode DISABLED. Modifications unlocked.".to_string();
                        }
                    }
                    KeyCode::Char('e') | KeyCode::Char('E') => {
                        // Task 21: External $EDITOR Fallback
                        if observer_mode {
                            action_log = "Cannot edit in Observer Mode.".to_string();
                        } else {
                            action_log = "Launching External Editor (Fallback)...".to_string();
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
                                action_log = "Can only open Chat on a Jules Session commit (🦑)."
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
                                let log = apply_cherry_pick(&rp_clone, &target_sha)
                                    .await
                                    .unwrap_or_else(|e| format!("Error: {}", e));
                                let _ = tx_clone.send(BgTaskResult::ActionCompleted(log)).await;
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
                                let log = revert_commit(&rp_clone, &target_sha)
                                    .await
                                    .unwrap_or_else(|e| format!("Error: {}", e));
                                let _ = tx_clone.send(BgTaskResult::ActionCompleted(log)).await;
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
                            if let Ok(fetched) = get_workflow_commits(&rp_clone, w_only).await {
                                let _ = tx_clone.send(BgTaskResult::CommitsFetched(fetched)).await;
                            }
                        });
                        selected_idx = 0;
                        current_diff_sha = String::new(); // Force refresh diff on load
                    }
                    KeyCode::Char('v') | KeyCode::Char('V') => {
                        is_branch_view = !is_branch_view;
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
                },
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
                                        let log = apply_cherry_pick(&rp_clone, &target_sha)
                                            .await
                                            .unwrap_or_else(|e| format!("Error: {}", e));
                                        let _ = tx_clone.send(BgTaskResult::ActionCompleted(log)).await;
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
                                        let log = revert_commit(&rp_clone, &target_sha)
                                            .await
                                            .unwrap_or_else(|e| format!("Error: {}", e));
                                        let _ = tx_clone.send(BgTaskResult::ActionCompleted(log)).await;
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
