mod adapter;
mod api;
mod config;
mod display;
mod git;
mod orchestrator;
pub mod tui_dashboard;

use anyhow::Result;
use clap::{Parser, Subcommand};
use colored::Colorize;
use config::RepoMode;
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "julesctl",
    version = "0.1.0",
    about = "Jules AI multi-session orchestrator",
    long_about = "julesctl orchestrates one or multiple Jules AI coding agent sessions.\n\
                  It fetches patches via the Jules API, applies them to a local\n\
                  orchestrator branch, and resolves conflicts automatically."
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Watch Jules session(s) and auto-apply patches (uses repo mode from config)
    Watch {
        #[arg(short, long, default_value_t = 30)]
        interval: u64,
        #[arg(short, long, default_value_t = 4)]
        messages: u32,
    },

    /// Start orchestrated mode with a goal (Mode 2)
    Orchestrate {
        /// The goal Jules manager should break into tasks
        goal: Vec<String>,
    },

    /// Send a message to a session
    Send {
        /// Message text
        message: Vec<String>,
        /// Target session ID (defaults to single_session_id or manager_session_id)
        #[arg(short, long)]
        session: Option<String>,
    },

    /// Show recent activities
    Status {
        #[arg(short, long, default_value_t = 8)]
        count: u32,
        #[arg(short, long)]
        session: Option<String>,
    },

    /// Session management (Mode 3)
    #[command(subcommand)]
    Session(SessionCommands),

    /// Show sessions list from Jules API
    Sessions,

    /// Print resolved config for current directory
    Config,

    /// Launch Terminal UI
    Tui {
        /// Session ID (defaults to single_session_id or manager_session_id)
        #[arg(short, long)]
        session: Option<String>,
    },

    /// Create starter config
    Init,
}

#[derive(Subcommand)]
enum SessionCommands {
    /// Add a session to the manual queue
    Add {
        session_id: String,
        label: String,
        #[arg(short, long)]
        position: Option<usize>,
    },
    /// Remove a session from the manual queue
    Remove { session_id: String },
    /// List configured sessions
    List,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let cfg = match config::load() {
        Ok(c) => c,
        Err(e) => {
            if cli.command.is_none()
                || matches!(
                    cli.command,
                    Some(Commands::Init) | Some(Commands::Tui { .. })
                )
            {
                // For init or default TUI, allow starting with empty config
                config::Config::default()
            } else {
                eprintln!("{} {}", "✗ Config error:".red().bold(), e);
                eprintln!(
                    "  Run {} to create a starter config.",
                    "julesctl init".cyan()
                );
                std::process::exit(1);
            }
        }
    };

    let command = cli.command.unwrap_or(Commands::Tui { session: None });

    // Init doesn't need repo check
    if let Commands::Init = &command {
        return config::init();
    }

    let cwd = std::env::current_dir()?;
    let repo = cfg.find_repo(&cwd).cloned();

    let client = api::JulesClient::new(&cfg.api_key);

    match command {
        Commands::Watch { .. } | Commands::Orchestrate { .. } => {
            eprintln!("{} Watch and Orchestrate commands have been deprecated in favor of the visual Git Dashboard.\n  Simply run 'julesctl' to launch the TUI.", "✗".red());
            std::process::exit(1);
        }

        Commands::Send { message, session } => {
            let text = message.join(" ");
            if text.trim().is_empty() {
                eprintln!("{} Message cannot be empty.", "✗".red());
                std::process::exit(1);
            }
            let sid = session
                .or_else(|| {
                    repo.as_ref().and_then(|r| {
                        if !r.single_session_id.is_empty() {
                            Some(r.single_session_id.clone())
                        } else if !r.manager_session_id.is_empty() {
                            Some(r.manager_session_id.clone())
                        } else {
                            None
                        }
                    })
                })
                .ok_or_else(|| anyhow::anyhow!("No session ID available. Use --session <id>"))?;

            println!("{} Sending to session {}…", "→".cyan(), sid.dimmed());
            client.send_message(&sid, &text).await?;
            println!("{} Sent.", "✓".green().bold());
        }

        Commands::Status { count, session } => {
            let sid = session
                .or_else(|| {
                    repo.as_ref().and_then(|r| {
                        if !r.single_session_id.is_empty() {
                            Some(r.single_session_id.clone())
                        } else if !r.manager_session_id.is_empty() {
                            Some(r.manager_session_id.clone())
                        } else {
                            None
                        }
                    })
                })
                .ok_or_else(|| anyhow::anyhow!("No session ID. Use --session <id>"))?;

            let activities = client.get_activities(&sid, count).await?;
            display::print_activities(&activities);
        }

        Commands::Session(sub) => {
            handle_session_commands(sub, cfg, &cwd)?;
        }

        Commands::Sessions => {
            let sessions = client.list_sessions().await?;
            if sessions.is_empty() {
                println!("{}", "No sessions found.".dimmed());
            } else {
                for s in &sessions {
                    println!(
                        "  {} {} {}",
                        s.id().dimmed(),
                        s.title.yellow(),
                        format!("[{}]", s.state).dimmed()
                    );
                }
            }
        }

        Commands::Config => {
            println!("{}", serde_json::to_string_pretty(&repo)?);
        }

        Commands::Tui { session } => {
            if let Some(id) = session {
                // Directly launch chat for specific session
                let adapter =
                    adapter::cli_chat_rs::JulesAdapter::new(&cfg.api_key, &id, "Direct Session");
                let mut app = cli_chat_rs::MessengerApp::new(
                    cli_chat_rs::Config::default(),
                    Box::new(adapter),
                );
                app.run()
                    .await
                    .map_err(|e| anyhow::anyhow!("TUI Error: {}", e))?;
                return Ok(());
            }

            loop {
                // Launch Dashboard first
                let action = tui_dashboard::run_dashboard(&cfg, repo.as_ref()).await?;

                match action {
                    tui_dashboard::DashboardAction::Quit => {
                        break;
                    }
                    tui_dashboard::DashboardAction::OpenChat(session_id, title) => {
                        // Launch cli-chat-rs specific for this session
                        let adapter = adapter::cli_chat_rs::JulesAdapter::new(
                            &cfg.api_key,
                            &session_id,
                            &title,
                        );
                        let mut app = cli_chat_rs::MessengerApp::new(
                            cli_chat_rs::Config::default(),
                            Box::new(adapter),
                        );

                        // Clear the screen before starting chat to prevent artifacts
                        println!("\x1B[2J\x1B[1;1H");

                        app.run()
                            .await
                            .map_err(|e| anyhow::anyhow!("Chat TUI Error: {}", e))?;

                        // When chat exits, loop continues to reopen dashboard
                    }
                    tui_dashboard::DashboardAction::CreateNew => {
                        // TODO: Implement task creation logic via CLI TUI prompt
                        println!("Create New Task flow is not yet implemented.");
                        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                    }
                }
            }
        }

        Commands::Init => unreachable!(),
    }

    Ok(())
}

fn handle_session_commands(
    sub: SessionCommands,
    mut cfg: config::Config,
    cwd: &PathBuf,
) -> Result<()> {
    let repo = cfg
        .find_repo_mut(cwd)
        .ok_or_else(|| anyhow::anyhow!("No repo entry for current directory"))?;

    match sub {
        SessionCommands::Add {
            session_id,
            label,
            position,
        } => {
            let pos = position.unwrap_or(repo.manual_sessions.len());
            // Shift existing positions if needed
            for s in repo.manual_sessions.iter_mut() {
                if s.queue_position >= pos {
                    s.queue_position += 1;
                }
            }
            repo.manual_sessions.push(config::ManualSession {
                session_id: session_id.clone(),
                label: label.clone(),
                queue_position: pos,
            });
            config::save(&cfg)?;
            println!(
                "{} Added session {} ({}) at position {}",
                "✓".green(),
                session_id.dimmed(),
                label.yellow(),
                pos
            );
        }

        SessionCommands::Remove { session_id } => {
            let before = repo.manual_sessions.len();
            repo.manual_sessions.retain(|s| s.session_id != session_id);
            if repo.manual_sessions.len() < before {
                config::save(&cfg)?;
                println!("{} Removed session {}", "✓".green(), session_id.dimmed());
            } else {
                eprintln!("{} Session {} not found", "✗".red(), session_id);
            }
        }

        SessionCommands::List => {
            if repo.manual_sessions.is_empty() {
                println!("{}", "No sessions configured.".dimmed());
            } else {
                let mut sessions = repo.manual_sessions.clone();
                sessions.sort_by_key(|s| s.queue_position);
                for s in &sessions {
                    println!(
                        "  [{}] {} ({})",
                        s.queue_position,
                        s.label.yellow(),
                        s.session_id.dimmed()
                    );
                }
            }
        }
    }

    Ok(())
}
