use crate::config::Config;
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
use std::time::Duration;

pub enum HomeAction {
    Quit,
    OpenWorkflow(String), // Path of the project
    CreateNewWorkflow,
}

pub async fn run_home(cfg: &Config) -> Result<HomeAction> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let res = home_loop(&mut terminal, cfg).await;

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    res.map_err(Into::into)
}

async fn home_loop<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    cfg: &Config,
) -> io::Result<HomeAction>
where
    std::io::Error: From<<B as ratatui::backend::Backend>::Error>,
{
    let mut selected_idx = 0;
    let mut list_state = ListState::default();

    loop {
        list_state.select(Some(selected_idx));

        let current_dir = std::env::current_dir().unwrap_or_default();
        let is_current_dir_configured = cfg.find_repo(&current_dir).is_some();

        let mut menu_items: Vec<(String, Option<String>)> = Vec::new(); // (Label, path)

        // Add existing configured projects (Workflows)
        if !cfg.repos.is_empty() {
            menu_items.push(("  -- Configured Workflows --".to_string(), None));
            for repo in &cfg.repos {
                let label = format!("  {} ({})", repo.display_name, repo.path);
                menu_items.push((label, Some(repo.path.clone())));
            }
            menu_items.push(("".to_string(), None)); // spacer
        } else {
            menu_items.push(("  No workflows configured yet.".to_string(), None));
            menu_items.push(("".to_string(), None)); // spacer
        }

        // Add actions
        if !is_current_dir_configured {
            menu_items.push((
                format!(
                    "  [+] Initialize Workflow in Current Directory ({})",
                    current_dir.display()
                ),
                Some("CREATE_NEW".to_string()),
            ));
        } else {
            menu_items.push((
                format!(
                    "  [*] Current Directory is already a Workflow ({})",
                    current_dir.display()
                ),
                Some(current_dir.to_string_lossy().to_string()),
            ));
        }

        menu_items.push(("  [Q] Quit".to_string(), Some("QUIT".to_string())));

        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints(
                    [
                        Constraint::Length(3), // Header
                        Constraint::Min(0),    // Body
                        Constraint::Length(3), // Footer
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
                Span::raw(" | Home - Select a Workflow"),
            ]))
            .block(Block::default().borders(Borders::ALL));
            f.render_widget(header, chunks[0]);

            // Main Menu List
            let items: Vec<ListItem> = menu_items
                .iter()
                .enumerate()
                .map(|(i, (label, path))| {
                    if path.is_none() && label.is_empty() {
                        return ListItem::new(Line::from(vec![Span::raw(" ")])); // Empty separator
                    } else if path.is_none() {
                        // Title or info text
                        return ListItem::new(Line::from(vec![Span::styled(
                            label.clone(),
                            Style::default().fg(Color::DarkGray),
                        )]));
                    }

                    let mut style = Style::default();
                    if i == selected_idx {
                        style = style
                            .fg(Color::Black)
                            .bg(Color::Yellow)
                            .add_modifier(Modifier::BOLD);
                    } else if path.as_deref() == Some("CREATE_NEW") {
                        style = style.fg(Color::Green);
                    } else if path.as_deref() == Some("QUIT") {
                        style = style.fg(Color::Red);
                    }

                    ListItem::new(Line::from(vec![Span::styled(label.clone(), style)]))
                })
                .collect();

            let list = List::new(items).block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Available Workflows "),
            );
            f.render_stateful_widget(list, chunks[1], &mut list_state);

            // Footer
            let footer =
                Paragraph::new(" Navigation: Up/Down/j/k | Action: Enter (Select) | Q/Esc (Quit) ")
                    .style(Style::default().fg(Color::DarkGray))
                    .block(Block::default().borders(Borders::ALL));
            f.render_widget(footer, chunks[2]);
        })?;

        if event::poll(Duration::from_millis(100))? {
            match event::read()? {
                Event::Key(key) => match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => return Ok(HomeAction::Quit),
                    KeyCode::Up | KeyCode::Char('k') => {
                        while selected_idx > 0 {
                            selected_idx -= 1;
                            if menu_items[selected_idx].1.is_some() {
                                break;
                            } // Only land on selectable items
                        }
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        while selected_idx + 1 < menu_items.len() {
                            selected_idx += 1;
                            if menu_items[selected_idx].1.is_some() {
                                break;
                            } // Only land on selectable items
                        }
                    }
                    KeyCode::Enter => {
                        let (_, action_val) = &menu_items[selected_idx];
                        if let Some(val) = action_val {
                            if val == "QUIT" {
                                return Ok(HomeAction::Quit);
                            } else if val == "CREATE_NEW" {
                                return Ok(HomeAction::CreateNewWorkflow);
                            } else {
                                return Ok(HomeAction::OpenWorkflow(val.clone()));
                            }
                        }
                    }
                    _ => {}
                },
                Event::Mouse(mouse_event) => {
                    if mouse_event.kind == MouseEventKind::Down(crossterm::event::MouseButton::Left)
                    {
                        let clicked_idx = mouse_event.row.saturating_sub(4) as usize; // Account for header
                        if clicked_idx < menu_items.len() {
                            let (_, action_val) = &menu_items[clicked_idx];
                            if action_val.is_some() {
                                selected_idx = clicked_idx;
                                // Also trigger enter immediately
                                let val = action_val.as_ref().unwrap();
                                if val == "QUIT" {
                                    return Ok(HomeAction::Quit);
                                } else if val == "CREATE_NEW" {
                                    return Ok(HomeAction::CreateNewWorkflow);
                                } else {
                                    return Ok(HomeAction::OpenWorkflow(val.clone()));
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }
}
