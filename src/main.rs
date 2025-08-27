mod client;
mod config;
mod embedded_assets;
mod prompt_detector;
mod pty_session;
mod session;
mod tui;
mod tui_writer;
mod web;

use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::fs;
use std::time::SystemTime;
use std::process::Command;
use tokio::time::{sleep, Duration};

use client::{CodeMuxClient, SessionConnection, ServerMessage};
use config::Config;
use session::SessionManager;
use pty_session::{PtyChannels, PtyInputMessage, PtyOutputMessage, PtyControlMessage, GridUpdateMessage};
use portable_pty::PtySize;
// Removed unused imports
use tui_writer::TuiWriter;

/// Shorten a path for display, replacing home directory with ~ and truncating long paths
fn shorten_path_for_display(path: &str) -> String {
    use std::path::Path;
    
    let path_buf = Path::new(path);
    
    // Try to replace home directory with ~
    if let Some(user_dirs) = directories::UserDirs::new() {
        let home_dir = user_dirs.home_dir();
        if let Ok(relative_path) = path_buf.strip_prefix(home_dir) {
            let home_path = if relative_path.as_os_str().is_empty() {
                "~".to_string() // Just home directory
            } else {
                format!("~/{}", relative_path.to_string_lossy())
            };
            return shorten_long_path(&home_path);
        }
    }
    
    shorten_long_path(path)
}

/// Shorten very long paths by truncating the middle
fn shorten_long_path(path: &str) -> String {
    const MAX_LENGTH: usize = 50;
    
    if path.len() <= MAX_LENGTH {
        return path.to_string();
    }
    
    // For very long paths, show start...end
    let start_len = MAX_LENGTH / 2 - 2;
    let end_len = MAX_LENGTH / 2 - 1;
    
    format!("{}...{}", 
        &path[..start_len], 
        &path[path.len() - end_len..]
    )
}

#[derive(Debug, Clone, ValueEnum)]
enum Agent {
    Claude,
    Gemini,
    Aider,
    Cursor,
    Continue,
}

impl Agent {
    fn as_str(&self) -> &'static str {
        match self {
            Agent::Claude => "claude",
            Agent::Gemini => "gemini",
            Agent::Aider => "aider",
            Agent::Cursor => "cursor",
            Agent::Continue => "continue",
        }
    }
}

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
        /// The code agent to run
        #[arg(value_enum)]
        agent: Agent,
        /// Auto-open the web interface in browser
        #[arg(short, long)]
        open: bool,
        /// Continue from the most recent JSONL conversation file
        #[arg(long = "continue")]
        continue_session: bool,
        /// Resume from a specific session ID
        #[arg(long = "resume")]
        resume_session: Option<String>,
        /// Project path or ID (e.g. /path/to/project, ., or project-uuid)
        #[arg(long)]
        project: Option<String>,
        /// Arguments to pass to the agent
        #[arg(trailing_var_arg = true)]
        args: Vec<String>,
    },
    /// Server management commands
    Server {
        #[command(subcommand)]
        command: Option<ServerCommands>,
    },
    /// Attach to an existing session
    Attach {
        /// Session ID to attach to
        session_id: String,
    },
    /// Create a new named session
    NewSession {
        /// Session name
        #[arg(short, long)]
        name: Option<String>,
        /// The code agent to run
        #[arg(value_enum)]
        agent: Agent,
        /// Project path or ID (e.g. /path/to/project, ., or project-uuid)
        #[arg(long)]
        project: Option<String>,
        /// Arguments to pass to the agent
        #[arg(trailing_var_arg = true)]
        args: Vec<String>,
    },
    /// Kill a specific session
    KillSession {
        /// Session ID to terminate
        session_id: String,
    },
    /// Add a project to the server
    AddProject {
        /// Path to the project directory
        path: PathBuf,
        /// Optional name for the project
        #[arg(short, long)]
        name: Option<String>,
    },
    /// List all sessions
    List,
    /// List all projects
    ListProjects,
    /// Stop the server
    Stop,
}

#[derive(Subcommand, Debug)]
enum ServerCommands {
    /// Start the server explicitly
    Start {
        /// Port to listen on
        #[arg(short, long, default_value = "8765")]
        port: u16,
    },
    /// Show server status
    Status,
    /// Stop the server
    Stop,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    tracing::info!("CodeMux starting with command: {:?}", cli.command);

    let config = Config::load()?;
    tracing::debug!("Config loaded successfully");

    // Configure tracing differently for client modes (run, attach, new-session) vs server mode
    let tui_writer_and_rx = if matches!(&cli.command, 
        Commands::Run { .. } | Commands::Attach { .. } | Commands::NewSession { .. }) {
        // For client modes, create TUI writer
        let (tui_writer, log_rx) = TuiWriter::new();

        tracing_subscriber::fmt()
            .with_writer(tui_writer)
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
            .with_ansi(false)
            .init();

        Some(log_rx)
    } else {
        // For server and other modes, use stderr normally
        tracing_subscriber::fmt()
            .with_writer(std::io::stderr)
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
            .init();
        None
    };

    // Initialize logging differently based on mode
    tracing::info!("Initializing command handler");
    match cli.command {
        Commands::Run {
            agent,
            open,
            continue_session,
            resume_session,
            project,
            args,
        } => {
            let agent_str = agent.as_str();
            tracing::info!(
                "Processing Run command - agent: {}, open: {}, continue: {}, resume: {:?}, project: {:?}, args: {:?}",
                agent_str,
                open,
                continue_session,
                resume_session,
                project,
                args
            );
            if let Some(log_rx) = tui_writer_and_rx {
                run_client_session(
                    config,
                    agent_str.to_string(),
                    open,
                    continue_session,
                    resume_session,
                    project,
                    args,
                    log_rx,
                )
                .await?;
            } else {
                // This shouldn't happen since we only create tui_writer_and_rx for Run commands
                panic!("TUI writer should be available for Run command");
            }
        }
        Commands::Server { command } => {
            match command {
                Some(ServerCommands::Start { port }) => {
                    start_server(config, port).await?;
                }
                Some(ServerCommands::Status) => {
                    server_status(config).await?;
                }
                Some(ServerCommands::Stop) => {
                    stop_server(config).await?;
                }
                None => {
                    // Default to start server
                    start_server(config, 8765).await?;
                }
            }
        }
        Commands::Attach { session_id } => {
            if let Some(log_rx) = tui_writer_and_rx {
                attach_to_session(config, session_id, log_rx).await?;
            } else {
                panic!("TUI writer should be available for Attach command");
            }
        }
        Commands::NewSession { name, agent, project, args } => {
            if let Some(log_rx) = tui_writer_and_rx {
                create_and_attach_session(
                    config,
                    name,
                    agent.as_str().to_string(),
                    project,
                    args,
                    log_rx,
                ).await?;
            } else {
                panic!("TUI writer should be available for NewSession command");
            }
        }
        Commands::KillSession { session_id } => {
            kill_session(config, session_id).await?;
        }
        Commands::AddProject { path, name } => {
            add_project(config, path, name).await?;
        }
        Commands::List => {
            list_sessions(config).await?;
        }
        Commands::ListProjects => {
            list_projects(config).await?;
        }
        Commands::Stop => {
            stop_server(config).await?;
        }
    }

    Ok(())
}

fn find_most_recent_jsonl() -> Result<Option<String>> {
    tracing::info!("Looking for most recent JSONL file in ~/.claude/projects/");
    
    let home = std::env::var("HOME").map_err(|_| anyhow::anyhow!("HOME environment variable not set"))?;
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
                                    let session_id = file_path.file_stem()
                                        .and_then(|s| s.to_str())
                                        .unwrap_or("unknown")
                                        .to_string();
                                    
                                    tracing::debug!("Found JSONL file: {:?}, session_id: {}, modified: {:?}", 
                                                  file_path, session_id, modified);
                                    
                                    match &most_recent {
                                        None => {
                                            most_recent = Some((modified, session_id, file_path));
                                        }
                                        Some((prev_time, _, _)) => {
                                            if modified > *prev_time {
                                                most_recent = Some((modified, session_id, file_path));
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
    
    if let Some((modified_time, session_id, file_path)) = most_recent {
        tracing::info!("Most recent JSONL file: {:?}, session_id: {}, modified: {:?}", 
                      file_path, session_id, modified_time);
        Ok(Some(session_id))
    } else {
        tracing::info!("No JSONL files found in ~/.claude/projects/");
        Ok(None)
    }
}

// OLD FUNCTION REMOVED - replaced with run_client_session

async fn start_server(config: Config, port: u16) -> Result<()> {
    tracing::info!("Starting server on port {}", port);

    // Create server PID file
    let pid_file = &config.server.pid_file;
    if pid_file.exists() {
        anyhow::bail!("Server already running (PID file exists). Run 'codemux server stop' first.");
    }

    // Create data directory if needed
    std::fs::create_dir_all(&config.server.data_dir)?;

    // Write PID file
    std::fs::write(pid_file, std::process::id().to_string())?;

    // Create session manager
    let session_manager = Arc::new(RwLock::new(SessionManager::new(config.clone())));

    // Start web server
    let manager_clone = session_manager.clone();
    let server_handle = tokio::spawn(async move {
        if let Err(e) = web::start_web_server(port, Some(manager_clone)).await {
            tracing::error!("Web server error: {}", e);
        }
    });

    println!("Server started on port {}", port);
    println!("Open http://localhost:{} to access the web interface", port);
    println!("Run 'codemux server stop' to stop the server");

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

// Legacy functions removed - replaced by client implementations below

/// Check if server is running by checking PID file and connectivity
async fn is_server_running(config: &Config) -> bool {
    // First try HTTP check as it's the most reliable
    let client = CodeMuxClient::from_config(&config);
    if client.is_server_running().await {
        return true;
    }
    
    // Fallback to PID file check
    let pid_file = &config.server.pid_file;
    if !pid_file.exists() {
        return false;
    }
    
    // Check if process is actually running
    if let Ok(pid_str) = std::fs::read_to_string(pid_file) {
        if let Ok(pid) = pid_str.trim().parse::<u32>() {
            // Try to check if process exists (Unix-specific)
            #[cfg(unix)]
            {
                let result = Command::new("kill")
                    .arg("-0")
                    .arg(pid.to_string())
                    .output();
                    
                if result.is_err() || !result.unwrap().status.success() {
                    // Process doesn't exist, remove stale PID file
                    let _ = std::fs::remove_file(pid_file);
                    return false;
                }
            }
            
            // Check if server is actually responding
            let client = CodeMuxClient::from_config(config);
            return client.is_server_running().await;
        }
    }
    
    // Remove invalid PID file
    let _ = std::fs::remove_file(pid_file);
    false
}

/// Start server in background if not already running
async fn ensure_server_running(config: &Config) -> Result<()> {
    if is_server_running(config).await {
        tracing::debug!("Server is already running");
        return Ok(());
    }
    
    tracing::info!("Server not running, starting in background");
    
    // Get current executable path
    let exe_path = std::env::current_exe()?;
    
    // Start server in background
    let mut cmd = Command::new(exe_path);
    cmd.arg("server")
        .arg("start")
        .arg("--port")
        .arg(config.server.port.to_string());
        
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        cmd.process_group(0); // Create new process group
    }
    
    let child = cmd.spawn()?;
    
    tracing::info!("Started server process with PID: {}", child.id());
    
    // Wait for server to be ready
    let client = CodeMuxClient::from_config(config);
    let mut attempts = 0;
    let max_attempts = 30; // 30 seconds timeout
    
    while attempts < max_attempts {
        if client.is_server_running().await {
            tracing::info!("Server is ready");
            return Ok(());
        }
        
        sleep(Duration::from_secs(1)).await;
        attempts += 1;
    }
    
    anyhow::bail!("Server failed to start within 30 seconds");
}

async fn server_status(config: Config) -> Result<()> {
    let client = CodeMuxClient::from_config(&config);
    
    if client.is_server_running().await {
        println!("✅ Server is running on port {}", config.server.port);
        
        // Show session count
        match client.list_sessions().await {
            Ok(sessions) => {
                println!("📋 Active sessions: {}", sessions.len());
                for session in sessions.iter().take(5) {
                    println!("   - {} ({})", session.id, session.agent);
                }
                if sessions.len() > 5 {
                    println!("   ... and {} more", sessions.len() - 5);
                }
            }
            Err(e) => {
                println!("⚠️  Could not retrieve session list: {}", e);
            }
        }
        
        println!("🌐 Web interface: http://localhost:{}", config.server.port);
    } else {
        println!("❌ Server is not running");
        println!("💡 Start it with: codemux server start");
    }
    
    Ok(())
}

async fn stop_server(config: Config) -> Result<()> {
    let pid_file = &config.server.pid_file;

    if !pid_file.exists() {
        println!("Server is not running");
        return Ok(());
    }

    let pid = std::fs::read_to_string(pid_file)?;
    println!("Stopping server (PID: {})", pid.trim());

    // TODO: Send proper shutdown signal to server process
    let _ = std::fs::remove_file(pid_file);

    println!("Server stopped");
    Ok(())
}

/// Run a client session by connecting to server
async fn run_client_session(
    config: Config,
    agent: String,
    open: bool,
    continue_session: bool,
    resume_session: Option<String>,
    project: Option<String>,
    args: Vec<String>,
    _log_rx: tokio::sync::mpsc::UnboundedReceiver<tui_writer::LogEntry>,
) -> Result<()> {
    tracing::info!("=== ENTERING run_client_session ===");
    
    // Ensure server is running
    ensure_server_running(&config).await?;
    
    let client = CodeMuxClient::from_config(&config);
    
    // Handle session continuity logic
    let final_args = if continue_session || resume_session.is_some() {
        let mut final_args = args.clone();
        
        if continue_session {
            match find_most_recent_jsonl()? {
                Some(session_id) => {
                    tracing::info!("Found previous session to continue: {}", session_id);
                    println!("🔄 Continuing from previous session: {}", session_id);
                    if agent.to_lowercase() == "claude" {
                        final_args.push("--resume".to_string());
                        final_args.push(session_id);
                    }
                }
                None => {
                    tracing::info!("No existing JSONL files found, creating new session");
                    println!("ℹ️  No previous sessions found, creating new session");
                }
            }
        } else if let Some(session_id) = &resume_session {
            tracing::info!("Resuming from specified session: {}", session_id);
            println!("🔄 Resuming from session: {}", session_id);
            if agent.to_lowercase() == "claude" {
                final_args.push("--resume".to_string());
                final_args.push(session_id.clone());
            }
        }
        
        final_args
    } else {
        args
    };
    
    // Resolve project path to project ID if needed
    let resolved_project_id = if let Some(project_input) = &project {
        // Check if it's already a UUID (project ID)
        if project_input.len() == 36 && project_input.chars().filter(|&c| c == '-').count() == 4 {
            // Looks like a UUID, use as-is
            Some(project_input.clone())
        } else {
            // Try to resolve as directory path
            match client.resolve_project_path(project_input).await? {
                Some(project_id) => {
                    tracing::info!("Resolved project path '{}' to project ID: {}", project_input, project_id);
                    Some(project_id)
                }
                None => {
                    println!("⚠️  Project path '{}' not found. Use 'codemux add-project {}' to register it first.", project_input, project_input);
                    None
                }
            }
        }
    } else {
        None
    };
    
    // Create session on server
    tracing::info!("Creating session on server - agent: {}, project: {:?}", agent, resolved_project_id);
    let session = client.create_session(agent.clone(), final_args, resolved_project_id).await?;
    
    println!("\n🚀 CodeMux - {} Agent Session", agent.to_uppercase());
    println!("📋 Session ID: {}", session.id);
    println!("🌐 Web Interface: {}", client.get_session_url(&session.id));
    
    // Open URL if requested
    if open {
        println!("\n🔄 Opening web interface...");
        let url = client.get_session_url(&session.id);
        if let Err(e) = open::that(&url) {
            println!("⚠️  Could not auto-open browser: {}", e);
            println!("💡 Please manually open: {}", url);
        } else {
            println!("✅ Web interface opened in your default browser");
        }
    }
    
    // Connect to session via WebSocket and start TUI
    tracing::info!("Connecting to session via WebSocket");
    let session_conn = client.connect_to_session(&session.id).await?;
    
    println!("🔗 Connected to session. Starting terminal interface...");
    
    // Create WebSocket-based PTY channels for TUI
    let pty_channels = create_websocket_pty_channels(session_conn).await?;
    
    // Start TUI with WebSocket connection
    let mut tui = tui::SessionTui::new(pty_channels, client.get_session_url(&session.id))?;
    let session_info = tui::SessionInfo {
        id: session.id.clone(),
        agent: session.agent.clone(),
        _port: config.server.port,
        working_dir: std::env::current_dir()?.to_string_lossy().to_string(),
        url: client.get_session_url(&session.id),
    };
    tui.run(session_info, _log_rx).await?;
    
    Ok(())
}

/// Attach to an existing session
async fn attach_to_session(
    config: Config,
    session_id: String,
    _log_rx: tokio::sync::mpsc::UnboundedReceiver<tui_writer::LogEntry>,
) -> Result<()> {
    tracing::info!("=== ENTERING attach_to_session ===");
    
    // Ensure server is running
    ensure_server_running(&config).await?;
    
    let client = CodeMuxClient::from_config(&config);
    
    // Check if session exists
    match client.get_session(&session_id).await {
        Ok(session) => {
            println!("\n🔗 Attaching to {} session: {}", session.agent.to_uppercase(), session.id);
            println!("🌐 Web Interface: {}", client.get_session_url(&session.id));
            
            // Connect to session via WebSocket and start TUI
            tracing::info!("Connecting to session via WebSocket");
            let session_conn = client.connect_to_session(&session.id).await?;
            
            println!("🔗 Connected to session. Starting terminal interface...");
            
            // Create WebSocket-based PTY channels for TUI
            let pty_channels = create_websocket_pty_channels(session_conn).await?;
            
            // Start TUI with WebSocket connection
            let mut tui = tui::SessionTui::new(pty_channels, client.get_session_url(&session.id))?;
            let session_info = tui::SessionInfo {
                id: session.id.clone(),
                agent: session.agent.clone(),
                _port: config.server.port,
                working_dir: std::env::current_dir()?.to_string_lossy().to_string(),
                url: client.get_session_url(&session.id),
            };
            tui.run(session_info, _log_rx).await?;
            
            Ok(())
        }
        Err(_) => {
            println!("❌ Session '{}' not found", session_id);
            
            // Show available sessions
            match client.list_sessions().await {
                Ok(sessions) if !sessions.is_empty() => {
                    println!("\n📋 Available sessions:");
                    for session in sessions {
                        println!("   - {} ({})", session.id, session.agent);
                    }
                }
                _ => {
                    println!("💡 No active sessions. Create one with: codemux run <agent>");
                }
            }
            
            anyhow::bail!("Session not found");
        }
    }
}

/// Create a new named session and attach to it
async fn create_and_attach_session(
    config: Config,
    name: Option<String>,
    agent: String,
    project: Option<String>,
    args: Vec<String>,
    _log_rx: tokio::sync::mpsc::UnboundedReceiver<tui_writer::LogEntry>,
) -> Result<()> {
    tracing::info!("=== ENTERING create_and_attach_session ===");
    
    // For now, just create a regular session (named sessions not yet implemented)
    if let Some(name) = name {
        println!("💡 Named sessions not yet implemented, creating regular session");
        println!("   Requested name: {}", name);
    }
    
    run_client_session(config, agent, false, false, None, project, args, _log_rx).await
}

/// Kill a specific session
async fn kill_session(config: Config, session_id: String) -> Result<()> {
    ensure_server_running(&config).await?;
    
    let client = CodeMuxClient::from_config(&config);
    
    match client.delete_session(&session_id).await {
        Ok(_) => {
            println!("✅ Session '{}' terminated", session_id);
        }
        Err(_) => {
            println!("❌ Session '{}' not found or could not be terminated", session_id);
        }
    }
    
    Ok(())
}

/// List all active sessions
async fn list_sessions(config: Config) -> Result<()> {
    ensure_server_running(&config).await?;
    
    let client = CodeMuxClient::from_config(&config);
    
    match client.list_sessions().await {
        Ok(sessions) => {
            if sessions.is_empty() {
                println!("📋 No active sessions");
                println!("💡 Start a new session with: codemux run <agent>");
            } else {
                println!("📋 Active sessions:");
                for session in sessions {
                    let project_info = if let Some(project) = &session.project {
                        format!(" (project: {})", project)
                    } else {
                        String::new()
                    };
                    println!("   - {} | {} | {}{}", 
                        session.id, 
                        session.agent,
                        session.status,
                        project_info
                    );
                }
                println!("\n💡 Attach to a session with: codemux attach <session-id>");
            }
        }
        Err(e) => {
            println!("❌ Could not retrieve sessions: {}", e);
        }
    }
    
    Ok(())
}

/// List all projects
async fn list_projects(config: Config) -> Result<()> {
    ensure_server_running(&config).await?;
    
    let client = CodeMuxClient::from_config(&config);
    
    match client.list_projects().await {
        Ok(projects) => {
            if projects.is_empty() {
                println!("📁 No projects configured");
                println!("💡 Add a project with: codemux add-project <path> --name <name>");
            } else {
                println!("📁 Configured projects:");
                for project in projects {
                    let display_path = shorten_path_for_display(&project.path);
                    println!("   - {} | {} | {} sessions", 
                        project.name, 
                        display_path,
                        project.sessions.len()
                    );
                    for session in project.sessions.iter().take(3) {
                        println!("     └─ {} ({})", session.id, session.agent);
                    }
                    if project.sessions.len() > 3 {
                        println!("     └─ ... and {} more sessions", project.sessions.len() - 3);
                    }
                }
            }
        }
        Err(e) => {
            println!("❌ Could not retrieve projects: {}", e);
        }
    }
    
    Ok(())
}

/// Add a project to the server
async fn add_project(config: Config, path: PathBuf, name: Option<String>) -> Result<()> {
    ensure_server_running(&config).await?;
    
    let client = CodeMuxClient::from_config(&config);
    
    // Convert to absolute path for storage
    let absolute_path = if path.is_absolute() {
        path
    } else {
        std::env::current_dir()?.join(path)
    };
    
    // Canonicalize to resolve symlinks and normalize
    let canonical_path = absolute_path.canonicalize()
        .unwrap_or(absolute_path); // Fall back if canonicalize fails
    
    let path_str = canonical_path.to_string_lossy().to_string();
    let project_name = name.unwrap_or_else(|| {
        canonical_path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Unnamed Project")
            .to_string()
    });
    
    match client.create_project(project_name.clone(), path_str.clone()).await {
        Ok(project) => {
            let display_path = shorten_path_for_display(&project.path);
            println!("✅ Added project '{}' at {}", project.name, display_path);
            println!("💡 Create sessions in this project with: codemux run <agent> --project {}", display_path);
        }
        Err(e) => {
            println!("❌ Could not add project: {}", e);
        }
    }
    
    Ok(())
}

/// Create PTY channels that communicate via WebSocket with server session
async fn create_websocket_pty_channels(session_conn: SessionConnection) -> Result<PtyChannels> {
    use tokio::sync::{broadcast, mpsc, Mutex};
    use std::sync::Arc;
    
    // Create channels for TUI communication
    let (input_tx, mut input_rx) = mpsc::unbounded_channel::<PtyInputMessage>();
    let (output_tx, _output_rx) = broadcast::channel::<PtyOutputMessage>(1000);
    let (control_tx, mut control_rx) = mpsc::unbounded_channel::<PtyControlMessage>();
    let (size_tx, _size_rx) = broadcast::channel::<PtySize>(10);
    let (grid_tx, _grid_rx) = broadcast::channel::<GridUpdateMessage>(100);
    
    let output_tx_clone = output_tx.clone();
    let grid_tx_clone = grid_tx.clone();
    let size_tx_clone = size_tx.clone();
    
    // Wrap session connection in Arc<Mutex> for sharing between tasks
    let session_conn = Arc::new(Mutex::new(session_conn));
    let session_conn_input = session_conn.clone();
    let session_conn_output = session_conn.clone();
    let session_conn_control = session_conn.clone();
    
    // Spawn task to handle WebSocket input (from TUI to server)
    tokio::spawn(async move {
        while let Some(input_msg) = input_rx.recv().await {
            let mut conn = session_conn_input.lock().await;
            if let Err(e) = conn.send_input(input_msg).await {
                tracing::error!("Failed to send input to WebSocket: {}", e);
                break;
            }
        }
        tracing::info!("Input WebSocket handler finished");
    });
    
    // Spawn task to handle WebSocket output (from server to TUI)
    tokio::spawn(async move {
        loop {
            let message = {
                let mut conn = session_conn_output.lock().await;
                conn.receive_message().await
            };
            
            match message {
                Ok(Some(ServerMessage::Output(output_msg))) => {
                    if let Err(e) = output_tx_clone.send(output_msg) {
                        tracing::error!("Failed to broadcast PTY output: {}", e);
                    }
                }
                Ok(Some(ServerMessage::Grid(grid_msg))) => {
                    if let Err(e) = grid_tx_clone.send(grid_msg) {
                        tracing::error!("Failed to broadcast grid update: {}", e);
                    }
                }
                Ok(Some(ServerMessage::Size { rows, cols })) => {
                    let size = PtySize { 
                        rows, 
                        cols, 
                        pixel_width: 0, 
                        pixel_height: 0 
                    };
                    if let Err(e) = size_tx_clone.send(size) {
                        tracing::error!("Failed to broadcast size update: {}", e);
                    }
                }
                Ok(Some(ServerMessage::Error(err))) => {
                    tracing::error!("Server error: {}", err);
                }
                Ok(None) => {
                    tracing::info!("WebSocket connection closed");
                    break;
                }
                Err(e) => {
                    tracing::error!("WebSocket receive error: {}", e);
                    break;
                }
            }
        }
        tracing::info!("Output WebSocket handler finished");
    });
    
    // Spawn task to handle control messages (resize, etc.)
    tokio::spawn(async move {
        while let Some(control_msg) = control_rx.recv().await {
            let mut conn = session_conn_control.lock().await;
            match control_msg {
                PtyControlMessage::Resize { rows, cols } => {
                    if let Err(e) = conn.send_resize(rows, cols).await {
                        tracing::error!("Failed to send resize to WebSocket: {}", e);
                    }
                }
                PtyControlMessage::RequestKeyframe { response_tx: _ } => {
                    if let Err(e) = conn.request_keyframe().await {
                        tracing::error!("Failed to request keyframe via WebSocket: {}", e);
                    }
                }
                PtyControlMessage::Terminate => {
                    tracing::info!("Terminating WebSocket connection");
                    break;
                }
            }
        }
        tracing::info!("Control WebSocket handler finished");
    });
    
    Ok(PtyChannels {
        input_tx,
        output_tx,
        control_tx,
        size_tx,
        grid_tx,
    })
}
