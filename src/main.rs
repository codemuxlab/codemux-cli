mod config;
mod prompt_detector;
mod pty;
mod pty_session;
mod session;
mod tui;
mod tui_writer;
mod web;

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use config::Config;
use pty_session::PtySession;
use session::SessionManager;
use tui::{SessionInfo as TuiSessionInfo, SessionTui};
use tui_writer::TuiWriter;

#[derive(Parser, Debug)]
#[command(name = "codemux")]
#[command(about = "Terminal multiplexer for AI code agents", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Run a single code agent session
    Run {
        /// The code agent to run (claude, gemini, aider, etc.)
        agent: String,
        /// Port to listen on for the web UI
        #[arg(short, long, default_value = "8765")]
        port: u16,
        /// Enable debug logging for key events
        #[arg(long)]
        debug: bool,
        /// Arguments to pass to the agent
        #[arg(trailing_var_arg = true)]
        args: Vec<String>,
    },
    /// Start the daemon service
    Daemon {
        /// Port to listen on
        #[arg(short, long, default_value = "8080")]
        port: u16,
    },
    /// Add a project to the daemon
    AddProject {
        /// Path to the project directory
        path: PathBuf,
        /// Optional name for the project
        #[arg(short, long)]
        name: Option<String>,
    },
    /// List all projects and sessions
    List,
    /// Stop the daemon
    Stop,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let config = Config::load()?;

    // Configure tracing differently for run vs daemon mode
    let tui_writer_and_rx = if matches!(&cli.command, Commands::Run { .. }) {
        // For run mode, create TUI writer
        let (tui_writer, log_rx) = TuiWriter::new();

        tracing_subscriber::fmt()
            .with_writer(tui_writer)
            .with_ansi(false) // No ANSI colors in output
            .init();

        if matches!(&cli.command, Commands::Run { debug: true, .. }) {
            let debug_log_path = std::env::temp_dir().join("codemux-debug.log");
            println!(
                "🐛 Debug mode enabled - logs will also be written to: {:?}",
                debug_log_path
            );
        }

        Some(log_rx)
    } else {
        // For daemon and other modes, use stderr normally
        tracing_subscriber::fmt()
            .with_writer(std::io::stderr)
            .init();
        None
    };

    // Initialize logging differently based on mode
    match cli.command {
        Commands::Run {
            agent,
            port,
            debug,
            args,
        } => {
            if let Some(log_rx) = tui_writer_and_rx {
                run_quick_session(config, agent, port, debug, args, log_rx).await?;
            } else {
                // This shouldn't happen since we only create tui_writer_and_rx for Run commands
                panic!("TUI writer should be available for Run command");
            }
        }
        Commands::Daemon { port } => {
            start_daemon(config, port).await?;
        }
        Commands::AddProject { path, name } => {
            add_project(path, name).await?;
        }
        Commands::List => {
            list_projects().await?;
        }
        Commands::Stop => {
            stop_daemon().await?;
        }
    }

    Ok(())
}

async fn run_quick_session(
    config: Config,
    agent: String,
    port: u16,
    debug: bool,
    args: Vec<String>,
    log_rx: tokio::sync::mpsc::UnboundedReceiver<tui_writer::LogEntry>,
) -> Result<()> {
    if !config.is_agent_allowed(&agent) {
        anyhow::bail!(
            "Code agent '{}' is not whitelisted. Add it to the config to use.",
            agent
        );
    }

    tracing::info!("Starting {} with args: {:?}", agent, args);

    // Create a temporary session manager
    let session_manager = Arc::new(RwLock::new(SessionManager::new(config.clone())));

    // Create broadcast channel for grid updates
    let (grid_broadcast_tx, _grid_broadcast_rx) = tokio::sync::broadcast::channel(1000);

    // Create PTY session directly (not through SessionManager)
    let final_args = args;
    let session_id = Uuid::new_v4().to_string();

    // Add session ID to args if the agent is Claude
    let mut agent_args = final_args.clone();
    if agent.to_lowercase() == "claude" {
        agent_args.push("--session-id".to_string());
        agent_args.push(session_id.clone());
    }

    let (mut pty_session, pty_channels) =
        PtySession::new(session_id.clone(), agent.clone(), agent_args)?;

    // Start the PTY session
    pty_session.start().await?;

    // Start web server in background with run mode UI and PTY channels
    let manager_clone = session_manager.clone();
    let agent_clone = agent.clone();
    let grid_rx_for_web = grid_broadcast_tx.subscribe();
    let pty_channels_for_web = pty_channels.clone();
    tokio::spawn(async move {
        if let Err(e) = web::start_web_server_run_mode(
            port,
            manager_clone,
            agent_clone,
            grid_rx_for_web,
            pty_channels_for_web,
        )
        .await
        {
            tracing::error!("Web server error: {}", e);
        }
    });

    // Keep PTY session alive by moving it to a task
    tokio::spawn(async move {
        // PTY session will run until dropped
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }
    });

    // Create TUI session info
    let tui_session_info = TuiSessionInfo {
        id: session_id.clone(),
        agent: agent.clone(),
        _port: port,
        working_dir: std::env::current_dir()
            .unwrap_or_else(|_| std::path::PathBuf::from("unknown"))
            .display()
            .to_string(),
        url: format!(
            "http://localhost:{}/?session={}&agent={}",
            port, session_id, agent
        ),
    };

    // Print session info
    println!("\n🚀 CodeMux - {} Agent Session", agent.to_uppercase());
    println!("📋 Session ID: {}", session_id);
    println!("🌐 Web Interface: {}", tui_session_info.url);
    println!("📁 Working Directory: {}", tui_session_info.working_dir);

    // Note for Claude sessions
    if agent.to_lowercase() == "claude" {
        println!("💡 Claude will use session ID: {}", session_id);
        let project_path = if tui_session_info.working_dir.starts_with('/') {
            format!("-{}", tui_session_info.working_dir[1..].replace('/', "-"))
        } else {
            format!("-{}", tui_session_info.working_dir.replace('/', "-"))
        };
        println!(
            "   History will be in: ~/.claude/projects/{}/",
            project_path
        );
    }

    // Open URL automatically
    println!("\n🔄 Opening web interface...");
    if let Err(e) = open::that(&tui_session_info.url) {
        println!("⚠️  Could not auto-open browser: {}", e);
        println!("💡 Please manually open: {}", tui_session_info.url);
    } else {
        println!("✅ Web interface opened in your default browser");
    }

    // Try to start TUI, fall back to simple display if it fails
    match SessionTui::new(debug) {
        Ok(mut tui) => {
            // Set PTY channels for the new architecture
            tui.set_pty_channels(pty_channels);

            // Set up WebSocket broadcast channel
            tui.set_websocket_broadcast(grid_broadcast_tx.clone());

            // Initial PTY resize using channels
            // TODO: Implement resize via channels instead of direct PTY access

            // Run TUI in a separate task
            let tui_handle = tokio::spawn(async move { tui.run(tui_session_info, log_rx).await });

            // Wait for either Ctrl+C or TUI to exit
            tokio::select! {
                _ = tokio::signal::ctrl_c() => {
                    // Don't print here - TUI is still active
                }
                result = tui_handle => {
                    // TUI has exited, safe to print after cleanup
                    match result {
                        Ok(Ok(_)) => {}, // Normal exit
                        Ok(Err(e)) => tracing::error!("TUI error: {}", e),
                        Err(e) => tracing::error!("TUI task error: {}", e),
                    }
                }
            }

            // TUI has cleaned up, now safe to print
            eprintln!("\nShutting down...");
        }
        Err(e) => {
            eprintln!("\n⚠️  Enhanced TUI not available: {}", e);
            eprintln!("📺 Using simple mode (press Ctrl+C to stop)");
            eprintln!("\n┌─────────────────────────────────────────┐");
            eprintln!("│  ⚡ Status: Running                     │");
            eprintln!("│  🌐 Web UI: {:<23} │", tui_session_info.url);
            eprintln!("└─────────────────────────────────────────┘");

            // Simple fallback - just wait for Ctrl+C
            tokio::signal::ctrl_c().await?;
            eprintln!("\nShutting down...");
        }
    }

    // Clean up session - PTY session will be cleaned up when dropped
    tracing::info!("Session {} finished", session_id);

    Ok(())
}

async fn start_daemon(config: Config, port: u16) -> Result<()> {
    tracing::info!("Starting daemon on port {}", port);

    // Create daemon PID file
    let pid_file = &config.daemon.pid_file;
    if pid_file.exists() {
        anyhow::bail!("Daemon already running (PID file exists). Run 'codemux stop' first.");
    }

    // Create data directory if needed
    std::fs::create_dir_all(&config.daemon.data_dir)?;

    // Write PID file
    std::fs::write(pid_file, std::process::id().to_string())?;

    // Create session manager
    let session_manager = Arc::new(RwLock::new(SessionManager::new(config.clone())));

    // Start web server
    let manager_clone = session_manager.clone();
    let server_handle = tokio::spawn(async move {
        if let Err(e) = web::start_web_server(port, manager_clone).await {
            tracing::error!("Web server error: {}", e);
        }
    });

    println!("Daemon started on port {}", port);
    println!("Open http://localhost:{} to access the web interface", port);
    println!("Run 'codemux stop' to stop the daemon");

    // Wait for shutdown signal
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            println!("\nReceived Ctrl-C, shutting down...");
        }
        _ = server_handle => {
            println!("Server stopped unexpectedly");
        }
    }

    // Clean up PID file
    let _ = std::fs::remove_file(pid_file);

    Ok(())
}

async fn add_project(path: PathBuf, name: Option<String>) -> Result<()> {
    // TODO: Connect to daemon and add project
    println!("Adding project at {:?} with name {:?}", path, name);
    println!("Note: This requires the daemon to be running");
    Ok(())
}

async fn list_projects() -> Result<()> {
    // TODO: Connect to daemon and list projects
    println!("Listing projects...");
    println!("Note: This requires the daemon to be running");
    Ok(())
}

async fn stop_daemon() -> Result<()> {
    let config = Config::load()?;
    let pid_file = &config.daemon.pid_file;

    if !pid_file.exists() {
        println!("Daemon is not running");
        return Ok(());
    }

    let pid = std::fs::read_to_string(pid_file)?;
    println!("Stopping daemon (PID: {})", pid.trim());

    // TODO: Send proper shutdown signal to daemon process
    let _ = std::fs::remove_file(pid_file);

    println!("Daemon stopped");
    Ok(())
}
