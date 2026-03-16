use crate::config::{Config, RepoConfig};
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
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Terminal,
};
use std::io;
use std::path::PathBuf;
use std::time::Duration;

pub enum DashboardAction {
    Quit,
    OpenChat(String, String), // session_id, title
    CreateNewSession,
    InitProject, // When no project is configured
}

pub async fn run_dashboard(_cfg: &Config, repo: Option<&RepoConfig>) -> Result<DashboardAction> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let res = dashboard_loop(&mut terminal, repo).await;

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
    repo: Option<&RepoConfig>,
) -> io::Result<DashboardAction>
where
    std::io::Error: From<<B as ratatui::backend::Backend>::Error>,
{
    let repo_path = repo.map(|r| PathBuf::from(&r.path));

    if let Some(ref path) = repo_path {
        // Fetch latest commits from remote before drawing
        let _ = fetch_origin(path);
    }

    let mut commits: Vec<GitCommit> = Vec::new();
    if let Some(ref path) = repo_path {
        if let Ok(fetched_commits) = get_workflow_commits(path) {
            commits = fetched_commits;
        }
    }

    let mut selected_idx = 0;
    let mut current_diff = String::from("Select a commit to view changes...");
    let mut current_diff_sha = String::new();
    let mut action_log = String::from("Ready.");

    loop {
        // Refresh diff if selection changed
        if !commits.is_empty() && commits[selected_idx].sha != current_diff_sha {
            current_diff_sha = commits[selected_idx].sha.clone();
            if let Some(ref path) = repo_path {
                current_diff =
                    get_commit_diff(path, &current_diff_sha).unwrap_or_else(|e| e.to_string());
            }
        }

        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints(
                    [
                        Constraint::Length(3), // Header
                        Constraint::Min(0),    // Body
                        Constraint::Length(4), // Footer
                    ]
                    .as_ref(),
                )
                .split(f.area());

            // Header
            let header = Paragraph::new(Line::from(vec![
                Span::styled(
                    "julesctl",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(" | Git-First AI Orchestrator Workflow"),
                if repo_path.is_none() {
                    Span::styled(" [No Project Configured]", Style::default().fg(Color::Red))
                } else {
                    Span::raw("")
                },
            ]))
            .block(Block::default().borders(Borders::ALL));
            f.render_widget(header, chunks[0]);

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
                    if i == selected_idx {
                        style = style.fg(Color::Black).bg(Color::Yellow);
                    }

                    let prefix = match c.branch_type {
                        BranchType::JulesSession(_) => "🦑",
                        BranchType::RemoteMain => "🐱",
                        BranchType::Local => "💻",
                    };

                    let text = format!("{} [{}] {}", prefix, c.short_sha, c.title);
                    ListItem::new(Line::from(vec![Span::styled(text, style)]))
                })
                .collect();

            let list_title = if repo_path.is_none() {
                " [Action Required] "
            } else if commits.is_empty() {
                " No Commits Found "
            } else {
                " Workflow Commits "
            };

            let final_items = if repo_path.is_none() {
                vec![
                    ListItem::new(Line::from(Span::styled(
                        "No project configured here.",
                        Style::default().fg(Color::Red),
                    ))),
                    ListItem::new(Line::from(Span::raw(""))),
                    ListItem::new(Line::from(Span::styled(
                        "  ▶ Initialize Project (julesctl init) ",
                        Style::default().fg(Color::Black).bg(Color::Yellow),
                    ))),
                ]
            } else {
                items
            };

            let list = List::new(final_items)
                .block(Block::default().borders(Borders::ALL).title(list_title));
            f.render_widget(list, body_chunks[0]);

            // Right Panel (Diff Preview)
            let diff_view = Paragraph::new(current_diff.as_str()).block(
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
                        " [N] New Session ",
                        Style::default().fg(Color::Black).bg(Color::Yellow),
                    ),
                    Span::raw(" "),
                    Span::styled(
                        " [Q] Quit ",
                        Style::default().fg(Color::White).bg(Color::DarkGray),
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
                        if selected_idx > 0 {
                            selected_idx -= 1;
                        }
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        if !commits.is_empty() && selected_idx + 1 < commits.len() {
                            selected_idx += 1;
                        }
                    }
                    KeyCode::Char('c') | KeyCode::Char('C') | KeyCode::Enter => {
                        if repo_path.is_none() {
                            return Ok(DashboardAction::InitProject);
                        } else if !commits.is_empty() {
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
                            if let Some(ref path) = repo_path {
                                action_log = apply_cherry_pick(path, &commits[selected_idx].sha)
                                    .unwrap_or_else(|e| format!("Error: {}", e));
                            }
                        }
                    }
                    KeyCode::Char('r') | KeyCode::Char('R') => {
                        if !commits.is_empty() {
                            if let Some(ref path) = repo_path {
                                action_log = revert_commit(path, &commits[selected_idx].sha)
                                    .unwrap_or_else(|e| format!("Error: {}", e));
                            }
                        }
                    }
                    KeyCode::Char('n') | KeyCode::Char('N') => {
                        return Ok(DashboardAction::CreateNewSession);
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
                            if col >= 29 && col <= 40 {
                                // [A] Apply
                                if !commits.is_empty() {
                                    if let Some(ref path) = repo_path {
                                        action_log =
                                            apply_cherry_pick(path, &commits[selected_idx].sha)
                                                .unwrap_or_else(|e| format!("Error: {}", e));
                                    }
                                }
                            } else if col >= 41 && col <= 53 {
                                // [R] Revert
                                if !commits.is_empty() {
                                    if let Some(ref path) = repo_path {
                                        action_log =
                                            revert_commit(path, &commits[selected_idx].sha)
                                                .unwrap_or_else(|e| format!("Error: {}", e));
                                    }
                                }
                            } else if col >= 54 && col <= 69 {
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
                            } else if col >= 70 && col <= 87 {
                                // [N] New Session
                                return Ok(DashboardAction::CreateNewSession);
                            } else if col >= 88 && col <= 98 {
                                // [Q] Quit
                                return Ok(DashboardAction::Quit);
                            }
                        } else if repo_path.is_none() {
                            // If they click inside the left panel while no project configured, auto-trigger init
                            if mouse_event.column < 40
                                && mouse_event.row > 3
                                && mouse_event.row < 10
                            {
                                return Ok(DashboardAction::InitProject);
                            }
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
