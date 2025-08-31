use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "codemux")]
#[command(about = "Terminal multiplexer for AI code agents", long_about = None)]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Run Claude AI coding assistant
    Claude {
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
        /// Path to write logs to file (in addition to TUI display)
        #[arg(long)]
        logfile: Option<PathBuf>,
        /// Arguments to pass to Claude
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
    /// Kill a specific session
    KillSession {
        /// Session ID to terminate
        session_id: String,
    },
    /// Add a project to the server
    AddProject {
        /// Project path
        path: PathBuf,
        /// Optional project name (defaults to directory name)
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

#[derive(Subcommand, Debug, Clone)]
pub enum ServerCommands {
    /// Start the server explicitly
    Start {
        /// Port to listen on
        #[arg(short, long, default_value = "8765")]
        port: u16,
        /// Run server in background (detached)
        #[arg(short, long)]
        detach: bool,
    },
    /// Show server status
    Status,
    /// Stop the server
    Stop,
}
