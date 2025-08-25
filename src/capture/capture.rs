use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use portable_pty::{CommandBuilder, NativePtySystem, PtySize, PtySystem};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use tokio::io::AsyncWriteExt;
use tokio::sync::{mpsc, Mutex};

use crate::session_data::{GridCell, SessionEvent, SessionRecording};

pub struct CaptureSession {
    agent: String,
    args: Vec<String>,
    output_path: PathBuf,
    recording: SessionRecording,
    start_time: Instant,
    capture_mode: CaptureMode,
}

#[derive(Debug, Clone, Copy)]
pub enum CaptureMode {
    Raw,  // Capture raw PTY output
    Grid, // Capture grid updates via VT100 parsing
    Both, // Capture both raw and grid
}

impl CaptureSession {
    pub fn new(
        agent: String,
        args: Vec<String>,
        output_path: PathBuf,
        capture_mode: CaptureMode,
    ) -> Result<Self> {
        let recording = SessionRecording::new(agent.clone(), args.clone());

        Ok(Self {
            agent,
            args,
            output_path,
            recording,
            start_time: Instant::now(),
            capture_mode,
        })
    }

    pub async fn start_recording(&mut self) -> Result<()> {
        println!("🎬 Starting capture session...");
        println!("📝 Press Ctrl+C to stop recording and save");
        println!("📊 Capture mode: {:?}", self.capture_mode);

        // Create PTY system
        let pty_system = NativePtySystem::default();

        // Set initial PTY size
        let pty_size = PtySize {
            rows: 30,
            cols: 120,
            pixel_width: 0,
            pixel_height: 0,
        };

        // Create PTY with command
        let mut cmd = CommandBuilder::new(&self.agent);
        for arg in &self.args {
            cmd.arg(arg);
        }

        let pty_pair = pty_system.openpty(pty_size)?;
        let mut child = pty_pair.slave.spawn_command(cmd)?;

        // Get reader and writer from master PTY
        let reader = Arc::new(Mutex::new(pty_pair.master.try_clone_reader()?));
        let writer = Arc::new(Mutex::new(pty_pair.master.take_writer()?));

        let (input_tx, mut input_rx) = mpsc::unbounded_channel::<Vec<u8>>();

        // Setup crossterm for raw terminal input
        crossterm::terminal::enable_raw_mode()?;

        // Start tasks to handle I/O
        let start_time = self.start_time;
        let capture_mode = self.capture_mode;
        let recording_tx = {
            let (tx, mut rx) = mpsc::unbounded_channel::<SessionEvent>();

            // Task to collect events and add to recording
            let mut recording = self.recording.clone();
            let output_path = self.output_path.clone();
            tokio::spawn(async move {
                while let Some(event) = rx.recv().await {
                    recording.add_event(event);
                }

                // Save recording when done
                recording.finalize();
                if let Err(e) = recording.save(&output_path) {
                    eprintln!("❌ Failed to save recording: {}", e);
                } else {
                    println!("💾 Recording saved to: {}", output_path.display());
                }
            });

            tx
        };

        // Task to handle raw PTY output
        let recording_tx_output = recording_tx.clone();
        let reader_clone = reader.clone();
        tokio::spawn(async move {
            let mut read_buffer = [0u8; 1024];
            let mut vt_parser = vt100::Parser::new(30, 120, 0);
            let mut grid_state: HashMap<(u16, u16), GridCell> = HashMap::new();

            loop {
                let mut reader_guard = reader_clone.lock().await;
                match reader_guard.read(&mut read_buffer) {
                    Ok(0) => {
                        eprintln!("PTY reader reached EOF");
                        break;
                    }
                    Ok(n) => {
                        let data = read_buffer[..n].to_vec();
                        let timestamp = start_time.elapsed().as_millis() as u32;

                        // Record raw PTY output if in Raw or Both mode
                        if matches!(capture_mode, CaptureMode::Raw | CaptureMode::Both) {
                            let event = SessionEvent::RawPtyOutput {
                                timestamp,
                                data: data.clone(),
                            };
                            let _ = recording_tx_output.send(event);
                        }

                        // Parse and record grid updates if in Grid or Both mode
                        if matches!(capture_mode, CaptureMode::Grid | CaptureMode::Both) {
                            vt_parser.process(&data);
                            let screen = vt_parser.screen();

                            // Convert VT100 screen to our grid format
                            let mut new_grid = HashMap::new();
                            let rows = 30u16; // Using initial size - TODO: track resize events
                            let cols = 120u16;

                            for row in 0..rows {
                                for col in 0..cols {
                                    if let Some(cell) = screen.cell(row, col) {
                                        if !cell.contents().is_empty() {
                                            let grid_cell = GridCell {
                                                char: cell.contents().to_string(),
                                                fg_color: None, // TODO: Extract colors from VT100
                                                bg_color: None,
                                                bold: cell.bold(),
                                                italic: cell.italic(),
                                                underline: cell.underline(),
                                            };
                                            new_grid.insert((row, col), grid_cell);
                                        }
                                    }
                                }
                            }

                            // Only send grid update if it changed
                            if new_grid != grid_state {
                                let cursor_pos = screen.cursor_position();
                                let cursor = (cursor_pos.0, cursor_pos.1);
                                let event = SessionEvent::GridUpdate {
                                    timestamp,
                                    size: (rows, cols),
                                    cells: new_grid.clone(),
                                    cursor,
                                };
                                let _ = recording_tx_output.send(event);
                                grid_state = new_grid;
                            }
                        }

                        // Also print to our stdout for monitoring
                        print!("{}", String::from_utf8_lossy(&data));
                        tokio::io::stdout().flush().await.ok();
                    }
                    Err(e) => {
                        eprintln!("Error reading from PTY: {}", e);
                        break;
                    }
                }
            }
        });

        // Task to handle stdin to PTY
        let writer_clone = writer.clone();
        tokio::spawn(async move {
            while let Some(data) = input_rx.recv().await {
                let mut writer_guard = writer_clone.lock().await;
                if let Err(e) = writer_guard.write_all(&data) {
                    eprintln!("❌ Failed to write to PTY: {}", e);
                    break;
                }
                if let Err(e) = writer_guard.flush() {
                    eprintln!("❌ Failed to flush PTY: {}", e);
                    break;
                }
            }
        });

        // Main input handling loop
        loop {
            // Check for keyboard input
            if event::poll(std::time::Duration::from_millis(10)).unwrap_or(false) {
                if let Ok(event) = event::read() {
                    match event {
                        Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                            match key_event.code {
                                KeyCode::Char('c')
                                    if key_event
                                        .modifiers
                                        .contains(crossterm::event::KeyModifiers::CONTROL) =>
                                {
                                    println!("\n🛑 Stopping recording...");
                                    break;
                                }
                                _ => {
                                    // Convert crossterm key event to bytes
                                    let input_bytes = self.key_event_to_bytes(&key_event);
                                    if !input_bytes.is_empty() {
                                        let timestamp = start_time.elapsed().as_millis() as u32;
                                        let event = SessionEvent::Input {
                                            timestamp,
                                            data: input_bytes.clone(),
                                        };
                                        let _ = recording_tx.send(event);
                                        let _ = input_tx.send(input_bytes);
                                    }
                                }
                            }
                        }
                        Event::Resize(cols, rows) => {
                            let timestamp = start_time.elapsed().as_millis() as u32;
                            let event = SessionEvent::Resize {
                                timestamp,
                                rows,
                                cols,
                            };
                            let _ = recording_tx.send(event);
                        }
                        _ => {}
                    }
                }
            }

            // Check if child process exited
            match child.try_wait() {
                Ok(Some(status)) => {
                    println!("📋 Process exited with status: {:?}", status);
                    break;
                }
                Ok(None) => {
                    // Process still running
                    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                }
                Err(e) => {
                    eprintln!("❌ Error checking process status: {}", e);
                    break;
                }
            }
        }

        // Cleanup
        crossterm::terminal::disable_raw_mode()?;

        // Give the recording task time to save
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        println!("✅ Recording session completed");
        Ok(())
    }

    fn key_event_to_bytes(&self, key_event: &crossterm::event::KeyEvent) -> Vec<u8> {
        use crossterm::event::{KeyCode, KeyModifiers};

        let mut bytes = Vec::new();

        match key_event.code {
            KeyCode::Char(c) => {
                if key_event.modifiers.contains(KeyModifiers::CONTROL) {
                    // Control character
                    if c.is_ascii() && c.is_alphabetic() {
                        let ctrl_byte = (c.to_ascii_lowercase() as u8) - b'a' + 1;
                        bytes.push(ctrl_byte);
                    }
                } else {
                    bytes.extend_from_slice(c.to_string().as_bytes());
                }
            }
            KeyCode::Enter => bytes.extend_from_slice(b"\r\n"),
            KeyCode::Tab => bytes.push(b'\t'),
            KeyCode::Backspace => bytes.push(0x7f),
            KeyCode::Delete => bytes.extend_from_slice(b"\x1b[3~"),
            KeyCode::Up => bytes.extend_from_slice(b"\x1b[A"),
            KeyCode::Down => bytes.extend_from_slice(b"\x1b[B"),
            KeyCode::Left => bytes.extend_from_slice(b"\x1b[D"),
            KeyCode::Right => bytes.extend_from_slice(b"\x1b[C"),
            KeyCode::Home => bytes.extend_from_slice(b"\x1b[H"),
            KeyCode::End => bytes.extend_from_slice(b"\x1b[F"),
            KeyCode::PageUp => bytes.extend_from_slice(b"\x1b[5~"),
            KeyCode::PageDown => bytes.extend_from_slice(b"\x1b[6~"),
            KeyCode::Esc => bytes.push(0x1b),
            _ => {} // Ignore other keys
        }

        bytes
    }
}
