use anyhow::Result;
use clap::{Parser, Subcommand};
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tracing_subscriber::fmt::MakeWriter;

mod analyze;
mod capture;
mod replay;
mod session_data;
mod test_chunking;

use analyze::analyze_jsonl_data;
use capture::{CaptureMode, CaptureSession};
use replay::ReplaySession;
use session_data::SessionRecording;
use test_chunking::{load_test_data_from_jsonl, test_vt100_chunking_strategies};

// Error collection writer to prevent VT100 debug messages from interfering with display
#[derive(Clone)]
struct ErrorCollectorWriter {
    errors: Arc<Mutex<Vec<String>>>,
}

impl ErrorCollectorWriter {
    fn new() -> Self {
        Self {
            errors: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn get_errors(&self) -> Vec<String> {
        self.errors.lock().unwrap().clone()
    }
}

impl Write for ErrorCollectorWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if let Ok(s) = String::from_utf8(buf.to_vec()) {
            if let Ok(mut errors) = self.errors.lock() {
                errors.push(s.trim().to_string());
            }
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl<'a> MakeWriter<'a> for ErrorCollectorWriter {
    type Writer = ErrorCollectorWriter;

    fn make_writer(&'a self) -> Self::Writer {
        self.clone()
    }
}

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
    /// Analyze JSONL capture data for cursor behavior debugging
    Analyze {
        /// Input JSONL file to analyze
        #[arg(short, long)]
        input: PathBuf,
        /// Show detailed VT100 processing steps
        #[arg(short, long)]
        verbose: bool,
    },
    /// Test VT100 chunking strategies to debug cursor positioning
    TestChunking {
        /// Input JSONL file to test different chunking strategies on
        #[arg(short, long)]
        input: PathBuf,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize error collector to prevent VT100 debug messages from interfering
    let error_collector = ErrorCollectorWriter::new();
    tracing_subscriber::fmt()
        .with_writer(error_collector.clone())
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
        Commands::Analyze { input, verbose } => {
            println!("🔍 Analyzing JSONL capture: {}", input.display());
            analyze_jsonl_data(&input, verbose).await?;
        }
        Commands::TestChunking { input } => {
            println!("🧪 Testing VT100 chunking strategies: {}", input.display());
            let raw_data = load_test_data_from_jsonl(input.to_str().unwrap())?;
            test_vt100_chunking_strategies(&raw_data)?;
        }
    }

    Ok(())
}
