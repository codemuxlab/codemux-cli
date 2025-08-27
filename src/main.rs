use clap::Parser;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::fmt::MakeWriter;
use std::io::Write;
use std::path::PathBuf;

use codemux::{Result, Config};
use codemux::cli::{Cli, Commands};
use codemux::cli::handlers::{self, RunSessionParams};
use codemux::utils::tui_writer::TuiWriter;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let config = Config::load()?;

    // Configure tracing differently for Claude/TUI mode vs other commands
    let log_rx = match &cli.command {
        Commands::Claude { logfile, .. } => {
            // For commands that use TUI, create TUI writer to capture logs
            let (tui_writer, log_rx) = TuiWriter::new();
            
            if let Some(ref log_path) = logfile {
                println!("📝 Logfile mode enabled - logs will also be written to: {:?}", log_path);
                
                // Create a multi-writer that implements MakeWriter
                #[derive(Clone)]
                struct MultiMakeWriter {
                    tui_writer: TuiWriter,
                    log_path: PathBuf,
                }
                
                impl<'a> MakeWriter<'a> for MultiMakeWriter {
                    type Writer = MultiWriter;
                    
                    fn make_writer(&'a self) -> Self::Writer {
                        let file = std::fs::OpenOptions::new()
                            .create(true)
                            .append(true)
                            .open(&self.log_path)
                            .expect("Failed to open log file");
                        
                        MultiWriter {
                            tui_writer: self.tui_writer.clone(),
                            file,
                        }
                    }
                }
                
                struct MultiWriter {
                    tui_writer: TuiWriter,
                    file: std::fs::File,
                }
                
                impl Write for MultiWriter {
                    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
                        // Write to both TUI writer and file
                        let _ = self.tui_writer.write(buf);
                        self.file.write(buf)
                    }
                    
                    fn flush(&mut self) -> std::io::Result<()> {
                        let _ = self.tui_writer.flush();
                        self.file.flush()
                    }
                }
                
                let multi_writer = MultiMakeWriter {
                    tui_writer: tui_writer.clone(),
                    log_path: log_path.clone(),
                };
                
                let env_filter = if std::env::var("RUST_LOG").is_ok() {
                    EnvFilter::from_default_env()
                } else {
                    EnvFilter::from_default_env().add_directive("codemux=info".parse().unwrap())
                };
                
                tracing_subscriber::fmt()
                    .with_writer(multi_writer)
                    .with_env_filter(env_filter)
                    .with_ansi(false)
                    .init();
            } else {
                // Just TUI writer, no file logging
                let env_filter = if std::env::var("RUST_LOG").is_ok() {
                    EnvFilter::from_default_env()
                } else {
                    EnvFilter::from_default_env().add_directive("codemux=info".parse().unwrap())
                };
                
                tracing_subscriber::fmt()
                    .with_writer(tui_writer)
                    .with_env_filter(env_filter)
                    .with_ansi(false)
                    .init();
            }
            
            log_rx
        }
        Commands::Attach { .. } => {
            // For attach command (TUI mode but no logfile option)
            let (tui_writer, log_rx) = TuiWriter::new();
            
            let env_filter = if std::env::var("RUST_LOG").is_ok() {
                EnvFilter::from_default_env()
            } else {
                EnvFilter::from_default_env().add_directive("codemux=info".parse().unwrap())
            };
            
            tracing_subscriber::fmt()
                .with_writer(tui_writer)
                .with_env_filter(env_filter)
                .with_ansi(false)
                .init();
                
            log_rx
        }
        _ => {
            // For non-TUI commands (server, list, etc.), use stderr normally
            let env_filter = if std::env::var("RUST_LOG").is_ok() {
                EnvFilter::from_default_env()
            } else {
                EnvFilter::from_default_env().add_directive("codemux=info".parse().unwrap())
            };
            
            tracing_subscriber::fmt()
                .with_writer(std::io::stderr)
                .with_env_filter(env_filter)
                .init();
                
            // Create dummy channel for consistency
            let (_tx, rx) = tokio::sync::mpsc::unbounded_channel();
            rx
        }
    };

    // Handle commands
    match &cli.command {
        Commands::Claude { open, continue_session, resume_session, project, logfile, args } => {
            handlers::run_client_session(RunSessionParams {
                config,
                agent: "claude".to_string(),
                open: *open,
                continue_session: *continue_session,
                resume_session: resume_session.clone(),
                project: project.clone(),
                logfile: logfile.clone(),
                args: args.clone(),
                log_rx,
            }).await
        }
        Commands::Server { command } => {
            handlers::handle_server_command(config, command.as_ref().cloned()).await
        }
        Commands::Attach { session_id } => {
            handlers::attach_to_session(config, session_id.clone(), log_rx).await
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