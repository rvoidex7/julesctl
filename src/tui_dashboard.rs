use crate::config::{Config, RepoConfig};
use crate::git::graph::{
    apply_cherry_pick, fetch_origin, get_commit_diff, get_workflow_commits, BranchType, GitCommit,
    revert_commit
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
where std::io::Error: From<<B as ratatui::backend::Backend>::Error>
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
                current_diff = get_commit_diff(path, &current_diff_sha).unwrap_or_else(|e| e.to_string());
            }
        }

        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3), // Header
                    Constraint::Min(0),    // Body
                    Constraint::Length(4), // Footer
                ].as_ref())
                .split(f.area());

            // Header
            let header = Paragraph::new(Line::from(vec![
                Span::styled("julesctl", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                Span::raw(" | Git-First AI Orchestrator Workflow"),
                if repo_path.is_none() {
                    Span::styled(" [No Project Configured]", Style::default().fg(Color::Red))
                } else {
                    Span::raw("")
                }
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

            let list_title = if commits.is_empty() { " No Commits Found " } else { " Workflow Commits " };
            let list = List::new(items)
                .block(Block::default().borders(Borders::ALL).title(list_title));
            f.render_widget(list, body_chunks[0]);

            // Right Panel (Diff Preview)
            let diff_view = Paragraph::new(current_diff.as_str())
                .block(Block::default().borders(Borders::ALL).title(" Commit Diff / Metadata "));
            f.render_widget(diff_view, body_chunks[1]);

            // Footer (Actions)
            let actions_text = vec![
                Line::from(vec![
                    Span::styled("Navigation: ", Style::default().fg(Color::DarkGray)),
                    Span::raw("Up/Down/j/k | "),
                    Span::styled("Actions: ", Style::default().fg(Color::Cyan)),
                    Span::raw("[A] Apply (Cherry-Pick) | [R] Revert | [C] Open Jules Chat | [N] New Session | [Q] Quit"),
                ]),
                Line::from(vec![
                    Span::styled("Status: ", Style::default().fg(Color::Green)),
                    Span::raw(action_log.as_str()),
                ])
            ];

            let footer = Paragraph::new(actions_text)
                .block(Block::default().borders(Borders::ALL));
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
                        if !commits.is_empty() {
                            if let BranchType::JulesSession(ref id) = commits[selected_idx].branch_type {
                                return Ok(DashboardAction::OpenChat(id.clone(), commits[selected_idx].title.clone()));
                            } else {
                                action_log = "Can only open Chat on a Jules Session commit (🦑).".to_string();
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
                    if mouse_event.kind == MouseEventKind::Down(crossterm::event::MouseButton::Left) {
                        let clicked_idx = mouse_event.row.saturating_sub(4) as usize; // Account for header
                        if mouse_event.column < 40 && clicked_idx < commits.len() {
                            selected_idx = clicked_idx;
                        }
                    }
                }
                _ => {}
            }
        }
    }
}
