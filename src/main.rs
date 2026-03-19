mod adapter;
mod api;
mod config;
mod display;
mod git;
pub mod tui_dashboard;

use anyhow::Result;
use clap::{Parser, Subcommand};
use colored::Colorize;
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
                .ok_or_else(|| anyhow::anyhow!("No session ID available. Use --session <id>"))?;

            println!("{} Sending to session {}…", "→".cyan(), sid.dimmed());
            client.send_message(&sid, &text).await?;
            println!("{} Sent.", "✓".green().bold());
        }

        Commands::Status { count, session } => {
            let sid =
                session.ok_or_else(|| anyhow::anyhow!("No session ID. Use --session <id>"))?;

            let activities = client.get_activities(&sid, count).await?;
            display::print_activities(&activities);
        }

        Commands::Session(_sub) => {
            // Deprecated, print warning
            eprintln!("{} Manual session queue management is deprecated. Please use the visual Git dashboard.", "⚠".yellow());
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
                // Directly launch chat for specific session bypass
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

            let mut mutable_cfg = cfg.clone();

            // If current directory isn't a repo, add it automatically to config to avoid the "Home Screen" intermediate step entirely.
            if mutable_cfg.find_repo(&cwd).is_none() {
                let new_repo = config::RepoConfig {
                    path: cwd.to_string_lossy().to_string(),
                    display_name: cwd
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string(),
                    github_url: "".to_string(),
                    post_pull: "".to_string(),
                    sessions: vec![],
                };
                mutable_cfg.repos.push(new_repo);
                let _ = config::save(&mutable_cfg);
            }

            // By default, start with the index of the repo that matches current directory
            let mut active_tab_index = mutable_cfg
                .repos
                .iter()
                .position(|r| PathBuf::from(&r.path) == cwd)
                .unwrap_or(0);

            loop {
                // The dashboard now manages tabs
                let dash_action =
                    tui_dashboard::run_dashboard(&mutable_cfg, active_tab_index).await?;

                match dash_action {
                    tui_dashboard::DashboardAction::Quit => {
                        break;
                    }
                    tui_dashboard::DashboardAction::SwitchTab(new_index) => {
                        active_tab_index = new_index;
                    }
                    tui_dashboard::DashboardAction::OpenChat(session_id, title) => {
                        let adapter = adapter::cli_chat_rs::JulesAdapter::new(
                            &mutable_cfg.api_key,
                            &session_id,
                            &title,
                        );
                        let mut app = cli_chat_rs::MessengerApp::new(
                            cli_chat_rs::Config::default(),
                            Box::new(adapter),
                        );
                        println!("\x1B[2J\x1B[1;1H");
                        app.run()
                            .await
                            .map_err(|e| anyhow::anyhow!("Chat TUI Error: {}", e))?;
                    }
                    tui_dashboard::DashboardAction::CreateNewSession => {
                        println!("\x1B[2J\x1B[1;1H");
                        println!(
                            "{} {}",
                            "julesctl".cyan().bold(),
                            "New Session Creation".dimmed()
                        );
                        println!("{}", "─".repeat(50).dimmed());

                        let mut goal = String::new();
                        println!("Enter the goal / prompt for the new Jules session:");
                        std::io::stdin().read_line(&mut goal)?;
                        let goal = goal.trim();

                        if goal.is_empty() {
                            println!(
                                "{} Goal cannot be empty. Returning to dashboard...",
                                "⚠".yellow()
                            );
                            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                            continue;
                        }

                        if active_tab_index < mutable_cfg.repos.len() {
                            let r = &mutable_cfg.repos[active_tab_index];
                            println!("{} Building context and creating session...", "→".cyan());

                            let full_prompt = config::rules::build_session_prompt(
                                goal,
                                Some(std::path::Path::new(&r.path)),
                            );
                            let safe_title: String = goal.chars().take(40).collect();

                            let github_url = if r.github_url.is_empty() {
                                None
                            } else {
                                Some(r.github_url.as_str())
                            };

                            match client
                                .create_session(
                                    &full_prompt,
                                    &format!("julesctl task: {}", safe_title),
                                    github_url,
                                    None,
                                )
                                .await
                            {
                                Ok(session) => {
                                    println!(
                                        "{} Session created successfully: {}",
                                        "✓".green(),
                                        session.id()
                                    );
                                    let _ = git::graph::fetch_origin(std::path::Path::new(&r.path));
                                }
                                Err(e) => {
                                    println!("{} Failed to create session: {}", "✗".red(), e);
                                }
                            }
                            tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
                        }
                    }
                    tui_dashboard::DashboardAction::CheckoutBranch(target) => {
                        if active_tab_index < mutable_cfg.repos.len() {
                            let r = &mutable_cfg.repos[active_tab_index];
                            let path = std::path::Path::new(&r.path);

                            // Check if the target is a Jules branch. If so, we need to create a new local branch from it
                            // to prevent the user from committing directly to the AI's remote branch.
                            let is_jules_target = target.contains("jules/task-");

                            if is_jules_target {
                                println!("\x1B[2J\x1B[1;1H");
                                println!(
                                    "{} {}",
                                    "julesctl".cyan().bold(),
                                    "Checkout Jules Session".dimmed()
                                );
                                println!("{}", "─".repeat(50).dimmed());
                                println!(
                                    "You selected a Jules AI session branch ({}).",
                                    target.yellow()
                                );
                                println!("You cannot checkout an AI remote branch directly. Let's create a local branch from it.");

                                let mut local_name = String::new();
                                println!("\nEnter a name for your new local branch:");
                                std::io::stdin().read_line(&mut local_name)?;
                                let local_name = local_name.trim();

                                if !local_name.is_empty() {
                                    println!(
                                        "{} Checking out new branch {} from {}...",
                                        "→".cyan(),
                                        local_name.green(),
                                        target.yellow()
                                    );
                                    // Equivalent to `git checkout -b <local_name> <target>`
                                    let mut cmd = std::process::Command::new("git");
                                    cmd.current_dir(path)
                                        .args(["checkout", "-b", local_name, &target]);

                                    if let Ok(output) = cmd.output() {
                                        if output.status.success() {
                                            println!("{} Success!", "✓".green());
                                        } else {
                                            println!(
                                                "{} Failed: {}",
                                                "✗".red(),
                                                String::from_utf8_lossy(&output.stderr)
                                            );
                                        }
                                    }
                                    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                                }
                            } else {
                                // Normal checkout
                                let _ = git::graph::checkout_branch(path, &target);
                            }
                        }
                    }
                }
            }
        }

        Commands::Init => unreachable!(),
    }

    Ok(())
}

// Deprecated handle_session_commands removed
