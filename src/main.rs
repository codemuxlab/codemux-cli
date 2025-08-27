use clap::Parser;
use tracing_subscriber::EnvFilter;

use codemux::{Result, Config};
use codemux::cli::{Cli, Commands, Agent, ServerCommands};
use codemux::cli::handlers;
use codemux::utils::tui_writer::{TuiWriter, LogLevel};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("codemux=info".parse().unwrap()))
        .init();

    let cli = Cli::parse();
    let config = Config::load()?;

    // Set up TUI writer for log capture
    let (log_tx, log_rx) = tokio::sync::mpsc::unbounded_channel();
    let _tui_writer = TuiWriter::new(log_tx.clone());

    // Handle commands
    match &cli.command {
        Commands::Run { agent, open, continue_session, resume_session, project, args } => {
            handlers::run_client_session(
                config,
                agent.as_str().to_string(),
                *open,
                *continue_session,
                resume_session.clone(),
                project.clone(),
                args.clone(),
                log_rx,
            ).await
        }
        Commands::Server { command } => {
            handlers::handle_server_command(config, command.clone()).await
        }
        Commands::Attach { session_id } => {
            handlers::attach_to_session(config, session_id.clone(), log_rx).await
        }
        Commands::NewSession { name, agent, project, args } => {
            handlers::create_and_attach_session(
                config,
                name.clone(),
                agent.as_str().to_string(),
                project.clone(),
                args.clone(),
                log_rx,
            ).await
        }
        Commands::KillSession { session_id } => {
            handlers::kill_session(config, session_id.clone()).await
        }
        Commands::AddProject { path, name } => {
            handlers::add_project(config, path.clone(), name.clone()).await
        }
        Commands::List => {
            handlers::list_sessions(config).await
        }
        Commands::ListProjects => {
            handlers::list_projects(config).await
        }
        Commands::Stop => {
            handlers::stop_server(config).await
        }
    }
}