// Command handlers - placeholder implementations
// TODO: Move actual implementations from old main.rs

use crate::cli::ServerCommands;
use crate::client::{CodeMuxClient, SessionTui};
use crate::server::{manager::SessionManagerHandle, start_web_server};
use crate::utils::tui_writer::LogEntry;
use crate::{Config, Result};
use std::path::PathBuf;
use std::time::SystemTime;
use std::{env, fs};

pub struct RunSessionParams {
    pub config: Config,
    pub agent: String,
    pub open: bool,
    pub continue_session: bool,
    pub resume_session: Option<String>,
    pub project: Option<String>,
    pub logfile: Option<PathBuf>,
    pub args: Vec<String>,
    pub log_rx: tokio::sync::mpsc::UnboundedReceiver<LogEntry>,
}

// Helper function to find most recent JSONL conversation file
fn find_most_recent_jsonl() -> Result<Option<String>> {
    tracing::info!("Looking for most recent JSONL file in ~/.claude/projects/");

    let home =
        env::var("HOME").map_err(|_| anyhow::anyhow!("HOME environment variable not set"))?;
    let claude_projects_path = PathBuf::from(&home).join(".claude").join("projects");

    if !claude_projects_path.exists() {
        tracing::info!("No ~/.claude/projects directory found");
        return Ok(None);
    }

    let mut most_recent: Option<(SystemTime, String, PathBuf)> = None;

    // Walk through all project directories
    for project_dir in fs::read_dir(&claude_projects_path)? {
        let project_dir = project_dir?;
        if !project_dir.file_type()?.is_dir() {
            continue;
        }

        let project_path = project_dir.path();
        tracing::debug!("Checking project directory: {:?}", project_path);

        // Look for JSONL files in this project directory
        if let Ok(entries) = fs::read_dir(&project_path) {
            for entry in entries {
                if let Ok(entry) = entry {
                    let file_path = entry.path();
                    if let Some(extension) = file_path.extension() {
                        if extension == "jsonl" {
                            if let Ok(metadata) = entry.metadata() {
                                if let Ok(modified) = metadata.modified() {
                                    let session_id = file_path
                                        .file_stem()
                                        .and_then(|s| s.to_str())
                                        .unwrap_or("unknown")
                                        .to_string();

                                    tracing::debug!(
                                        "Found JSONL file: {:?}, session_id: {}, modified: {:?}",
                                        file_path,
                                        session_id,
                                        modified
                                    );

                                    match &most_recent {
                                        None => {
                                            most_recent = Some((modified, session_id, file_path));
                                        }
                                        Some((prev_time, _, _)) => {
                                            if modified > *prev_time {
                                                most_recent =
                                                    Some((modified, session_id, file_path));
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    if let Some((_, session_id, file_path)) = most_recent {
        tracing::info!(
            "Most recent JSONL file: {:?} (session_id: {})",
            file_path,
            session_id
        );
        Ok(Some(session_id))
    } else {
        tracing::info!("No JSONL files found");
        Ok(None)
    }
}

pub async fn run_client_session(params: RunSessionParams) -> Result<()> {
    let RunSessionParams {
        config,
        agent,
        open,
        continue_session,
        resume_session,
        project: _project,
        logfile: _logfile, // Logfile handling is done in main.rs tracing setup
        args,
        log_rx,
    } = params;

    tracing::info!("=== ENTERING run_client_session ===");
    tracing::info!(
        "Agent: {}, Open: {}, Continue: {}, Resume: {:?}",
        agent,
        open,
        continue_session,
        resume_session
    );
    tracing::info!("Args: {:?}", args);

    tracing::debug!("Checking if agent '{}' is whitelisted", agent);
    if !config.is_agent_allowed(&agent) {
        tracing::error!("Agent '{}' is not whitelisted in config", agent);
        anyhow::bail!(
            "Code agent '{}' is not whitelisted. Add it to the config to use.",
            agent
        );
    }
    tracing::info!("Agent '{}' is whitelisted, proceeding", agent);

    tracing::info!("=== CONNECTING TO SERVER ===");

    // Create HTTP client
    let client = CodeMuxClient::from_config(&config);

    // Check if server is running, start it if not
    if !client.is_server_running().await {
        tracing::info!("🚀 Starting CodeMux server as independent process...");

        // Start server as independent process using current executable
        let current_exe = std::env::current_exe()
            .map_err(|e| anyhow::anyhow!("Failed to get current executable path: {}", e))?;

        let mut cmd = tokio::process::Command::new(&current_exe);
        cmd.args(&["server", "start"]);

        // Pass through RUST_LOG environment variable
        if let Ok(rust_log) = std::env::var("RUST_LOG") {
            cmd.env("RUST_LOG", rust_log);
        }

        // Spawn the server process
        let child = cmd
            .spawn()
            .map_err(|e| anyhow::anyhow!("Failed to spawn server process: {}", e))?;

        tracing::info!(
            "Spawned server process with PID: {}",
            child.id().unwrap_or(0)
        );

        // Wait a moment for server to start
        tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

        // Verify server is now running
        if !client.is_server_running().await {
            anyhow::bail!(
                "Failed to start server process. Please run 'codemux server start' manually."
            );
        }

        tracing::info!("✅ Server process started successfully");
    }

    // Validate that both --continue and --resume aren't used together
    if continue_session && resume_session.is_some() {
        anyhow::bail!("Cannot use both --continue and --resume flags together. Use --continue to resume the most recent session or --resume <session_id> to resume a specific session.");
    }

    // Determine if we're continuing a previous session
    let (is_continuing, previous_session_id) = if continue_session {
        match find_most_recent_jsonl()? {
            Some(found_session_id) => {
                tracing::info!("🔄 Continuing from previous session: {}", found_session_id);
                (true, Some(found_session_id))
            }
            None => {
                tracing::info!("ℹ️  No existing JSONL files found, creating new session");
                (false, None)
            }
        }
    } else if let Some(ref session_id_to_resume) = resume_session {
        tracing::info!(
            "🔄 Resuming from specified session: {}",
            session_id_to_resume
        );
        (true, Some(session_id_to_resume.clone()))
    } else {
        (false, None)
    };

    // Prepare agent arguments with session continuation info
    let mut agent_args = args;
    if agent.to_lowercase() == "claude" && is_continuing {
        if let Some(prev_id) = &previous_session_id {
            tracing::info!("Adding --resume flag with session ID to Claude agent args");
            agent_args.push("--resume".to_string());
            agent_args.push(prev_id.clone());
        }
    }

    // Get current directory path
    let current_dir = std::env::current_dir()?;
    let current_path = current_dir.to_string_lossy().to_string();

    // Create session on server
    tracing::info!("📋 Creating session on server...");
    tracing::debug!(
        "Creating session with agent: {}, args: {:?}, path: {}",
        agent,
        agent_args,
        current_path
    );

    let session_info = match client
        .create_session_with_path(agent.clone(), agent_args.clone(), current_path)
        .await
    {
        Ok(info) => {
            tracing::info!(
                "✅ Session created successfully on server with ID: {}",
                info.id
            );
            tracing::debug!("Session info: {:?}", info);
            info
        }
        Err(e) => {
            tracing::error!("❌ Failed to create session: {}", e);
            return Err(e);
        }
    };

    let session_id = session_info.id.clone();

    // Connect to the session via WebSocket
    println!("🔌 Connecting to session via WebSocket...");
    let session_connection = client.connect_to_session(&session_id).await?;

    // Convert WebSocket connection into PTY-like channels for TUI
    let pty_channels = session_connection.into_pty_channels();

    // Create session info for TUI
    let working_dir = env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("unknown"))
        .display()
        .to_string();
    let url = format!("http://localhost:8765/session/{}", session_id); // Default port for now

    // Print session info
    if is_continuing {
        println!(
            "\n🔄 CodeMux - Continuing {} Agent Session",
            agent.to_uppercase()
        );
    } else {
        println!("\n🚀 CodeMux - {} Agent Session", agent.to_uppercase());
    }
    println!("📋 Session ID: {}", session_id);
    println!("🌐 Web Interface: {}", url);
    println!("📁 Working Directory: {}", working_dir);

    // Note for Claude sessions
    if agent.to_lowercase() == "claude" {
        if is_continuing {
            if let Some(prev_id) = &previous_session_id {
                println!("💡 Continuing from previous session: {}", prev_id);
                println!("💡 New session ID: {}", session_id);
            } else {
                println!("💡 Claude will use session ID: {}", session_id);
            }
        } else {
            println!("💡 Claude will use session ID: {}", session_id);
        }
        let project_path = if working_dir.starts_with('/') {
            format!("-{}", working_dir[1..].replace('/', "-"))
        } else {
            format!("-{}", working_dir.replace('/', "-"))
        };
        println!(
            "   History will be in: ~/.claude/projects/{}/",
            project_path
        );
    }

    // Open URL if requested
    if open {
        println!("\n🔄 Opening web interface...");
        if let Err(e) = open::that(&url) {
            println!("⚠️  Could not auto-open browser: {}", e);
            println!("💡 Please manually open: {}", url);
        } else {
            println!("✅ Web interface opened in your default browser");
        }
    } else {
        println!("\n💡 Press 'o' in monitoring mode to open the web interface");
    }

    // Try to start TUI, fall back to simple display if it fails
    tracing::info!("Attempting to create TUI...");
    match SessionTui::new(pty_channels, url.clone()) {
        Ok(mut tui) => {
            tracing::info!("TUI created successfully");
            // Run TUI in a separate task
            let tui_session_info = crate::client::tui::SessionInfo {
                id: session_id.clone(),
                agent: agent.clone(),
                _port: 8765, // Default port
                working_dir,
                url: url.clone(),
            };

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
            tracing::error!("TUI creation failed: {}", e);
            eprintln!("\n⚠️  Enhanced TUI not available: {}", e);
            eprintln!("📺 Using simple mode (press Ctrl+C to stop)");
            eprintln!("\n┌─────────────────────────────────────────┐");
            eprintln!("│  ⚡ Status: Running                     │");
            eprintln!("│  🌐 Web UI: {:<23} │", url);
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

pub async fn handle_server_command(config: Config, command: Option<ServerCommands>) -> Result<()> {
    let client = CodeMuxClient::from_config(&config);

    match command {
        Some(ServerCommands::Start { port, detach }) => {
            println!("Starting server on port {}...", port);

            // Check if server is already running
            if client.is_server_running().await {
                println!("Server is already running on port {}", port);
                return Ok(());
            }

            if detach {
                // Start server in background (detached)
                let current_exe = std::env::current_exe()?;
                let mut cmd = tokio::process::Command::new(&current_exe);
                cmd.args(&["server", "start", "--port", &port.to_string()]);

                // Pass through RUST_LOG environment variable
                if let Ok(rust_log) = std::env::var("RUST_LOG") {
                    cmd.env("RUST_LOG", rust_log);
                }

                let child = cmd
                    .spawn()
                    .map_err(|e| anyhow::anyhow!("Failed to spawn detached server: {}", e))?;

                println!(
                    "🚀 CodeMux server started in background with PID: {}",
                    child.id().unwrap_or(0)
                );
                println!("📍 Server will be available at http://localhost:{}", port);

                // Wait a moment and verify it started
                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                if client.is_server_running().await {
                    println!("✅ Server is running successfully");
                } else {
                    println!("⚠️  Server may still be starting up...");
                }
            } else {
                // Start server in foreground
                let session_manager = SessionManagerHandle::new(config);

                println!("🚀 CodeMux server starting on http://localhost:{}", port);
                println!("💡 Use Ctrl+C to stop the server, or 'codemux server start -d' to run in background");
                start_web_server(port, session_manager).await?;
            }
        }

        Some(ServerCommands::Status) => {
            println!("Checking server status...");

            if client.is_server_running().await {
                println!("✅ Server is running");

                // Get project list to show more details
                match client.list_projects().await {
                    Ok(projects) => {
                        if projects.is_empty() {
                            println!("📂 No projects registered");
                        } else {
                            println!("📂 Projects ({}):", projects.len());
                            for project in projects {
                                let session_count = project.sessions.len();
                                println!("  • {} ({} sessions)", project.name, session_count);
                            }
                        }
                    }
                    Err(e) => {
                        println!("⚠️  Could not fetch project details: {}", e);
                    }
                }
            } else {
                println!("❌ Server is not running");
                println!("💡 Start the server with: codemux server start");
            }
        }

        Some(ServerCommands::Stop) => {
            tracing::info!("Stopping server...");

            if !client.is_server_running().await {
                tracing::info!("❌ Server is not running");
                return Ok(());
            }

            match client.shutdown_server().await {
                Ok(()) => {
                    tracing::info!("✅ Server shutdown successfully");

                    // Wait a moment for server to shut down
                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

                    // Verify server is stopped
                    if !client.is_server_running().await {
                        tracing::info!("🛑 Server has stopped");
                    }
                }
                Err(e) => {
                    tracing::error!("❌ Failed to shutdown server: {}", e);
                    tracing::info!(
                        "💡 Server may have already stopped or use Ctrl+C to force stop"
                    );
                }
            }
        }

        None => {
            // Default to showing status when no subcommand provided
            println!("Checking server status...");

            if client.is_server_running().await {
                println!("✅ Server is running");
            } else {
                println!("❌ Server is not running");
                println!("💡 Available commands:");
                println!("  • codemux server start    - Start the server");
                println!("  • codemux server status   - Check server status");
                println!("  • codemux server stop     - Stop the server");
            }
        }
    }

    Ok(())
}

pub async fn attach_to_session(
    _config: Config,
    _session_id: String,
    _log_rx: tokio::sync::mpsc::UnboundedReceiver<LogEntry>,
) -> Result<()> {
    println!("Attach command - implementation needed");
    Ok(())
}

// Removed: create_and_attach_session - no longer needed after removing NewSession command

pub async fn kill_session(_config: Config, _session_id: String) -> Result<()> {
    println!("Kill session command - implementation needed");
    Ok(())
}

pub async fn add_project(config: Config, path: PathBuf, name: Option<String>) -> Result<()> {
    let client = CodeMuxClient::from_config(&config);

    // Check if server is running
    if !client.is_server_running().await {
        println!("❌ Server is not running");
        println!("💡 Start the server first with: codemux server start");
        return Ok(());
    }

    println!("Adding project...");

    // Canonicalize the path
    let canonical_path = path
        .canonicalize()
        .map_err(|e| anyhow::anyhow!("Invalid path {:?}: {}", path, e))?;

    let project_name = name.unwrap_or_else(|| {
        canonical_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unnamed-project")
            .to_string()
    });

    match client
        .create_project(
            project_name.clone(),
            canonical_path.to_string_lossy().to_string(),
        )
        .await
    {
        Ok(_) => {
            println!("✅ Project '{}' added successfully", project_name);
            println!("📁 Path: {}", canonical_path.display());
        }
        Err(e) => {
            println!("❌ Failed to add project: {}", e);
        }
    }

    Ok(())
}

pub async fn list_sessions(config: Config) -> Result<()> {
    let client = CodeMuxClient::from_config(&config);

    // Check if server is running
    if !client.is_server_running().await {
        println!("❌ Server is not running");
        println!("💡 Start the server first with: codemux server start");
        return Ok(());
    }

    println!("📋 Active Sessions:");

    match client.list_projects().await {
        Ok(projects) => {
            if projects.is_empty() {
                println!("   No projects or sessions found");
                println!("💡 Add a project with: codemux add-project <path>");
            } else {
                for project in projects {
                    println!("\n📂 Project: {}", project.name);
                    if project.sessions.is_empty() {
                        println!("   No active sessions");
                    } else {
                        for session in &project.sessions {
                            println!(
                                "   🚀 {} ({}): {}",
                                session.agent, session.status, session.id
                            );
                        }
                    }
                }
            }
        }
        Err(e) => {
            println!("❌ Failed to list sessions: {}", e);
        }
    }

    Ok(())
}

pub async fn list_projects(config: Config) -> Result<()> {
    let client = CodeMuxClient::from_config(&config);

    // Check if server is running
    if !client.is_server_running().await {
        println!("❌ Server is not running");
        println!("💡 Start the server first with: codemux server start");
        return Ok(());
    }

    println!("📂 Registered Projects:");

    match client.list_projects().await {
        Ok(projects) => {
            if projects.is_empty() {
                println!("   No projects registered");
                println!("💡 Add a project with: codemux add-project <path>");
            } else {
                for project in projects {
                    let session_count = project.sessions.len();
                    println!("   • {} ({} sessions)", project.name, session_count);
                    if session_count > 0 {
                        for session in &project.sessions {
                            println!(
                                "     └── {} ({}): {}",
                                session.agent, session.status, session.id
                            );
                        }
                    }
                }
            }
        }
        Err(e) => {
            println!("❌ Failed to list projects: {}", e);
        }
    }

    Ok(())
}

pub async fn stop_server(config: Config) -> Result<()> {
    let client = CodeMuxClient::from_config(&config);

    tracing::info!("Stopping server...");

    if !client.is_server_running().await {
        tracing::info!("❌ Server is not running");
        return Ok(());
    }

    match client.shutdown_server().await {
        Ok(()) => {
            tracing::info!("✅ Server shutdown successfully");

            // Wait a moment for server to shut down
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

            // Verify server is stopped
            if !client.is_server_running().await {
                tracing::info!("🛑 Server has stopped");
            }
        }
        Err(e) => {
            tracing::error!("❌ Failed to shutdown server: {}", e);
            tracing::info!("💡 Server may have already stopped or use Ctrl+C to force stop");
        }
    }

    Ok(())
}
