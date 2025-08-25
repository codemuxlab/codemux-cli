use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

mod capture;
mod replay;
mod session_data;

use capture::{CaptureMode, CaptureSession};
use replay::ReplaySession;
use session_data::SessionRecording;

#[derive(Parser, Debug)]
#[command(name = "codemux-capture")]
#[command(about = "Capture and replay code agent sessions for testing", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Capture a code agent session to a file
    Capture {
        /// The code agent to run (claude, gemini, aider, etc.)
        agent: String,
        /// Output file to save the session recording
        #[arg(short, long)]
        output: PathBuf,
        /// Capture mode: raw (PTY output), grid (VT100 parsed), or both
        #[arg(short, long, default_value = "raw")]
        mode: String,
        /// Arguments to pass to the agent
        #[arg(trailing_var_arg = true)]
        args: Vec<String>,
    },
    /// Replay a captured session
    Replay {
        /// Input file containing the session recording
        #[arg(short, long)]
        input: PathBuf,
        /// Start playback at specific timestamp (milliseconds)
        #[arg(short, long, default_value = "0")]
        start: u32,
        /// Auto-play on start (vs paused)
        #[arg(short, long)]
        auto_play: bool,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize tracing for debugging
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    match cli.command {
        Commands::Capture {
            agent,
            output,
            mode,
            args,
        } => {
            println!("🎬 Starting capture session for {}", agent);
            println!("📁 Recording to: {}", output.display());

            let capture_mode = match mode.as_str() {
                "raw" => CaptureMode::Raw,
                "grid" => CaptureMode::Grid,
                "both" => CaptureMode::Both,
                _ => {
                    eprintln!(
                        "❌ Invalid capture mode: {}. Use 'raw', 'grid', or 'both'",
                        mode
                    );
                    return Ok(());
                }
            };

            let mut capture = CaptureSession::new(agent, args, output, capture_mode)?;
            capture.start_recording().await?;
        }
        Commands::Replay {
            input,
            start,
            auto_play,
        } => {
            println!("▶️ Starting replay of: {}", input.display());

            let recording = SessionRecording::load(&input)?;
            let mut replay = ReplaySession::new(recording, start, auto_play)?;
            replay.start_playback().await?;
        }
    }

    Ok(())
}
