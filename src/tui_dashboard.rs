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
use std::io;
use std::path::PathBuf;
use std::time::Duration;

pub enum DashboardAction {
    Quit,
    OpenChat(String, String), // session_id, title
    CreateNewSession,
    SwitchTab(usize),
    CheckoutBranch(String),
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

    // Fetch latest commits from remote before drawing
    let _ = fetch_origin(&repo_path);

    let mut workflow_only = true;

    let mut commits: Vec<GitCommit> = Vec::new();
    if let Ok(fetched_commits) = get_workflow_commits(&repo_path, workflow_only) {
        commits = fetched_commits;
    }

    let mut selected_idx = 0;
    let mut diff_scroll_offset: u16 = 0;
    let mut current_diff = String::from("Select a commit to view changes...");
    let mut current_diff_sha = String::new();
    let mut action_log = String::from("Ready.");

    let mut list_state = ListState::default();

    loop {
        list_state.select(Some(selected_idx));

        // Refresh diff if selection changed
        if !commits.is_empty() && commits[selected_idx].sha != current_diff_sha {
            current_diff_sha = commits[selected_idx].sha.clone();
            diff_scroll_offset = 0; // Reset scroll on new commit
            current_diff =
                get_commit_diff(&repo_path, &current_diff_sha).unwrap_or_else(|e| e.to_string());
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

            let body_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(40), Constraint::Percentage(60)].as_ref())
                .split(chunks[1]);

            // Left Panel (Git Graph)
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
                " Workflow Commits (Filtered) "
            } else {
                " All Git Data "
            };

            let list =
                List::new(items).block(Block::default().borders(Borders::ALL).title(list_title));
            f.render_stateful_widget(list, body_chunks[0], &mut list_state);

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
                    KeyCode::PageUp => {
                        diff_scroll_offset = diff_scroll_offset.saturating_sub(10);
                    }
                    KeyCode::PageDown => {
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
                        if !commits.is_empty() {
                            action_log = apply_cherry_pick(&repo_path, &commits[selected_idx].sha)
                                .unwrap_or_else(|e| format!("Error: {}", e));
                        }
                    }
                    KeyCode::Char('r') | KeyCode::Char('R') => {
                        if !commits.is_empty() {
                            action_log = revert_commit(&repo_path, &commits[selected_idx].sha)
                                .unwrap_or_else(|e| format!("Error: {}", e));
                        }
                    }
                    KeyCode::Char('f') | KeyCode::Char('F') => {
                        workflow_only = !workflow_only;
                        if let Ok(fetched_commits) = get_workflow_commits(&repo_path, workflow_only)
                        {
                            commits = fetched_commits;
                            selected_idx = 0;
                            current_diff_sha = String::new(); // Force refresh diff
                        }
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
                                if !commits.is_empty() {
                                    action_log =
                                        apply_cherry_pick(&repo_path, &commits[selected_idx].sha)
                                            .unwrap_or_else(|e| format!("Error: {}", e));
                                }
                            } else if (41..=53).contains(&col) {
                                // [R] Revert
                                if !commits.is_empty() {
                                    action_log =
                                        revert_commit(&repo_path, &commits[selected_idx].sha)
                                            .unwrap_or_else(|e| format!("Error: {}", e));
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
