mod adapter;
mod api;
mod config;
mod display;
mod git;
mod modes;
mod orchestrator;
mod patch;

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
            if matches!(cli.command, Some(Commands::Init)) || cli.command.is_none() {
                // For init or default TUI, we might handle missing config differently
                if cli.command.is_none() {
                    eprintln!("{} Config not found. Run {} to set up your API key.", "!" .yellow(), "julesctl init".cyan());
                    std::process::exit(1);
                }
                config::Config::default() 
            } else {
                eprintln!("{} {}", "✗ Config error:".red().bold(), e);
                eprintln!("  Run {} to create a starter config.", "julesctl init".cyan());
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
        Commands::Watch { interval, messages } => {
            let repo = repo.ok_or_else(|| anyhow::anyhow!("No repo config found for this directory."))?;
            match repo.mode {
                RepoMode::Single => {
                    modes::single::run_single(client, &repo, interval, messages).await?;
                }
                RepoMode::Manual => {
                    modes::manual::run_manual(client, &repo, interval).await?;
                }
                RepoMode::Orchestrated => {
                    eprintln!(
                        "{} Use 'julesctl orchestrate <goal>' for orchestrated mode.",
                        "✗".red()
                    );
                    std::process::exit(1);
                }
            }
        }

        Commands::Orchestrate { goal } => {
            let repo = repo.ok_or_else(|| anyhow::anyhow!("No repo config found for this directory."))?;
            let goal_text = goal.join(" ");
            if goal_text.trim().is_empty() {
                eprintln!("{} Goal cannot be empty.", "✗".red());
                std::process::exit(1);
            }
            orchestrator::run_orchestrated(client, &repo, &goal_text).await?;
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
                });

            let adapter = match sid {
                Some(id) => adapter::cli_chat_rs::JulesAdapter::new(&cfg.api_key, &id),
                None => adapter::cli_chat_rs::JulesAdapter::new_global(&cfg.api_key),
            };
            let mut app = cli_chat_rs::MessengerApp::new(
                cli_chat_rs::Config::default(),
                Box::new(adapter),
            );

            app.run().await.map_err(|e| anyhow::anyhow!("TUI Error: {}", e))?;
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
        SessionCommands::Add { session_id, label, position } => {
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
            println!("{} Added session {} ({}) at position {}", "✓".green(), session_id.dimmed(), label.yellow(), pos);
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
                    println!("  [{}] {} ({})", s.queue_position, s.label.yellow(), s.session_id.dimmed());
                }
            }
        }
    }

    Ok(())
}
