use crate::config::{Config, RepoConfig, RepoMode};
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
use std::time::Duration;

pub enum DashboardAction {
    Quit,
    OpenChat(String, String), // session_id, title
    CreateNew,
}

pub async fn run_dashboard(cfg: &Config, repo: Option<&RepoConfig>) -> Result<DashboardAction> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let res = dashboard_loop(&mut terminal, cfg, repo).await;

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
    _cfg: &Config,
    repo: Option<&RepoConfig>,
) -> io::Result<DashboardAction>
where
    std::io::Error: From<<B as ratatui::backend::Backend>::Error>,
{
    let mut selected_idx = 0;

    // Build the list of available items for this project
    let mut menu_items: Vec<(String, Option<String>, Option<String>)> = Vec::new(); // (Label, session_id, context/type)

    if let Some(r) = repo {
        menu_items.push((format!("Project: {}", r.display_name), None, None));
        menu_items.push(("".to_string(), None, None)); // Separator

        match r.mode {
            RepoMode::Single => {
                if !r.single_session_id.is_empty() {
                    menu_items.push((
                        "Active Single Session".to_string(),
                        Some(r.single_session_id.clone()),
                        Some("Single".to_string()),
                    ));
                }
            }
            RepoMode::Orchestrated => {
                if !r.manager_session_id.is_empty() {
                    menu_items.push((
                        "Manager Session".to_string(),
                        Some(r.manager_session_id.clone()),
                        Some("Manager".to_string()),
                    ));
                }
                // TODO: Read .julesctl-tasks.json to show individual orchestrated tasks here
            }
            RepoMode::Manual => {
                let mut sorted = r.manual_sessions.clone();
                sorted.sort_by_key(|s| s.queue_position);
                for s in sorted {
                    menu_items.push((
                        s.label.clone(),
                        Some(s.session_id.clone()),
                        Some("Manual".to_string()),
                    ));
                }
            }
        }
    } else {
        menu_items.push((
            "No project configured in this directory.".to_string(),
            None,
            None,
        ));
        menu_items.push((
            "Run 'julesctl init' to create a config.".to_string(),
            None,
            None,
        ));
    }

    // Always append Create New / Settings
    menu_items.push(("".to_string(), None, None));
    menu_items.push((
        "Create New Task (Not implemented yet)".to_string(),
        None,
        None,
    ));
    menu_items.push(("Quit".to_string(), None, None));

    loop {
        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints(
                    [
                        Constraint::Length(3),
                        Constraint::Min(0),
                        Constraint::Length(3),
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
                Span::raw(" | AI Multi-Session Orchestrator Dashboard"),
            ]))
            .block(Block::default().borders(Borders::ALL));
            f.render_widget(header, chunks[0]);

            // Main Menu List
            let items: Vec<ListItem> = menu_items
                .iter()
                .enumerate()
                .map(|(i, (label, _, _))| {
                    if label.is_empty() {
                        return ListItem::new(Line::from(vec![Span::raw(" ")])); // Empty separator
                    }
                    let mut style = Style::default();
                    if i == selected_idx {
                        style = style
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD)
                            .bg(Color::DarkGray);
                    }
                    ListItem::new(Line::from(vec![Span::styled(
                        format!("  {}  ", label),
                        style,
                    )]))
                })
                .collect();

            let list = List::new(items).block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Project Sessions "),
            );
            f.render_widget(list, chunks[1]);

            // Footer
            let footer = Paragraph::new("Press Enter to select | Up/Down to navigate | q to quit")
                .style(Style::default().fg(Color::DarkGray))
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
                            // Skip separators
                            if menu_items[selected_idx].0.is_empty() && selected_idx > 0 {
                                selected_idx -= 1;
                            }
                        }
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        if selected_idx + 1 < menu_items.len() {
                            selected_idx += 1;
                            // Skip separators
                            if menu_items[selected_idx].0.is_empty()
                                && selected_idx + 1 < menu_items.len()
                            {
                                selected_idx += 1;
                            }
                        }
                    }
                    KeyCode::Enter => {
                        let (_, session_id, _) = &menu_items[selected_idx];
                        if let Some(id) = session_id {
                            return Ok(DashboardAction::OpenChat(
                                id.clone(),
                                menu_items[selected_idx].0.clone(),
                            ));
                        } else if menu_items[selected_idx].0.contains("Quit") {
                            return Ok(DashboardAction::Quit);
                        } else if menu_items[selected_idx].0.contains("Create New") {
                            return Ok(DashboardAction::CreateNew);
                        }
                    }
                    _ => {}
                },
                Event::Mouse(mouse_event) => {
                    if mouse_event.kind == MouseEventKind::Down(crossterm::event::MouseButton::Left)
                    {
                        let clicked_idx = mouse_event.row.saturating_sub(4) as usize; // Account for header offset + borders
                        if clicked_idx < menu_items.len() && !menu_items[clicked_idx].0.is_empty() {
                            selected_idx = clicked_idx;
                            // Also trigger Enter if they clicked
                            let (_, session_id, _) = &menu_items[selected_idx];
                            if let Some(id) = session_id {
                                return Ok(DashboardAction::OpenChat(
                                    id.clone(),
                                    menu_items[selected_idx].0.clone(),
                                ));
                            } else if menu_items[selected_idx].0.contains("Quit") {
                                return Ok(DashboardAction::Quit);
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }
}
