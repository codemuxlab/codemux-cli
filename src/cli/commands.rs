use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Debug, Clone, ValueEnum)]
pub enum Agent {
    Claude,
    Gemini,
    Aider,
    Cursor,
    Continue,
}

impl Agent {
    pub fn as_str(&self) -> &'static str {
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
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
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
    },
    /// Show server status
    Status,
    /// Stop the server
    Stop,
}