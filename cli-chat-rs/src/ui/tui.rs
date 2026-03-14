use crate::MessengerApp;
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

pub async fn run_tui(app: &mut MessengerApp) -> Result<()> {
    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let res = run_app(&mut terminal, app).await;

    // restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err)
    }

    Ok(())
}

async fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut MessengerApp,
) -> Result<()>
where
    std::io::Error: From<<B as ratatui::backend::Backend>::Error>,
{
    // Connect adapter
    if let Err(e) = app.adapter_mut().connect().await {
        return Err(anyhow::anyhow!("Failed to connect: {:?}", e));
    }

    let chats = app.adapter().get_chats().await.unwrap_or_default();
    let mut selected_chat = 0;
    let mut input_mode = false;
    let mut input = String::new();

    // Cache messages
    let mut messages: Vec<crate::types::Message> = Vec::new();
    let mut current_chat_id = String::new();

    // Setup message subscription
    let mut rx = app
        .adapter_mut()
        .subscribe_to_messages()
        .await
        .unwrap_or_else(|_| {
            let (_tx, empty_rx) = tokio::sync::mpsc::channel(1);
            empty_rx
        });

    loop {
        // Fetch history if selected chat changed
        if !chats.is_empty() && chats[selected_chat].id != current_chat_id {
            current_chat_id = chats[selected_chat].id.clone();
            messages = app
                .adapter()
                .get_messages(&current_chat_id, 50)
                .await
                .unwrap_or_default();
        }

        // Process incoming subscribed messages non-blocking
        while let Ok(msg) = rx.try_recv() {
            if msg.chat_id == current_chat_id {
                messages.push(msg);
            }
        }

        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(30), Constraint::Percentage(70)].as_ref())
                .split(f.area());

            let right_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(0), Constraint::Length(3)].as_ref())
                .split(chunks[1]);

            // Left panel: Chat list
            let items: Vec<ListItem> = chats
                .iter()
                .enumerate()
                .map(|(i, c)| {
                    let mut style = Style::default();
                    if i == selected_chat {
                        style = style.fg(Color::Yellow).add_modifier(Modifier::BOLD);
                    }
                    ListItem::new(Line::from(vec![Span::styled(c.name.clone(), style)]))
                })
                .collect();
            let chats_list =
                List::new(items).block(Block::default().borders(Borders::ALL).title("Sessions"));
            f.render_widget(chats_list, chunks[0]);

            // Right panel top: Messages
            let msg_lines: Vec<Line> = messages
                .iter()
                .map(|m| {
                    let text = match &m.content {
                        crate::types::MessageContent::Text(t) => t.clone(),
                        _ => "[Attachment]".to_string(),
                    };
                    let prefix = if m.is_from_me { "Me: " } else { "Agent: " };
                    let style = if m.is_from_me {
                        Style::default().fg(Color::Blue)
                    } else {
                        Style::default().fg(Color::Green)
                    };
                    Line::from(vec![Span::styled(prefix, style), Span::raw(text)])
                })
                .collect();
            let messages_view = Paragraph::new(msg_lines)
                .block(Block::default().borders(Borders::ALL).title("Messages"));
            f.render_widget(messages_view, right_chunks[0]);

            // Right panel bottom: Input box
            let input_style = if input_mode {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default()
            };
            let title = if input_mode {
                "Input (Press Esc to cancel, Enter to send)"
            } else {
                "Input (Press 'i' to insert, 'q' to quit)"
            };
            let input_widget = Paragraph::new(input.as_str())
                .style(input_style)
                .block(Block::default().borders(Borders::ALL).title(title));
            f.render_widget(input_widget, right_chunks[1]);
        }).map_err(|e| anyhow::anyhow!("Terminal draw error: {}", e))?;

        if event::poll(Duration::from_millis(100))? {
            match event::read()? {
                Event::Key(key) => {
                    if input_mode {
                        match key.code {
                            KeyCode::Enter => {
                                if !input.trim().is_empty() && !chats.is_empty() {
                                    let chat_id = &chats[selected_chat].id;
                                    let content = crate::types::MessageContent::Text(input.clone());
                                    let _ = app.adapter_mut().send_message(chat_id, content).await;
                                }
                                input.clear();
                                input_mode = false;
                            }
                            KeyCode::Char(c) => {
                                input.push(c);
                            }
                            KeyCode::Backspace => {
                                input.pop();
                            }
                            KeyCode::Esc => {
                                input_mode = false;
                            }
                            _ => {}
                        }
                    } else {
                        match key.code {
                            KeyCode::Char('q') => return Ok(()),
                            KeyCode::Char('i') => input_mode = true,
                            KeyCode::Down | KeyCode::Char('j') => {
                                if selected_chat + 1 < chats.len() {
                                    selected_chat += 1;
                                }
                            }
                            KeyCode::Up | KeyCode::Char('k') => {
                                if selected_chat > 0 {
                                    selected_chat -= 1;
                                }
                            }
                            _ => {}
                        }
                    }
                }
                Event::Mouse(mouse_event) => {
                    if mouse_event.kind == MouseEventKind::Down(crossterm::event::MouseButton::Left)
                    {
                        // Very simple mouse click detection for sidebar
                        if mouse_event.column < 30 {
                            let clicked_idx = mouse_event.row.saturating_sub(1) as usize; // account for border
                            if clicked_idx < chats.len() {
                                selected_chat = clicked_idx;
                                input_mode = false;
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }
}
