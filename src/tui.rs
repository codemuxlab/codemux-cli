use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, EventStream, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::io::{Write, Read};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame, Terminal,
};
use std::io;
use tokio::time::{Duration, Instant};
use tui_term::widget::PseudoTerminal;
use vt100::Parser;
use futures_util::StreamExt;
use tokio::sync::mpsc;

use std::sync::Arc;
use crate::session::SessionManager;

/// Write a debug log message to the debug log file
fn debug_log(debug_mode: bool, message: impl std::fmt::Display) {
    if debug_mode {
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("codemux_debug.log") 
        {
            use std::io::Write;
            let _ = writeln!(
                file, 
                "[{}] {}", 
                chrono::Utc::now().format("%Y-%m-%d %H:%M:%S%.3f"),
                message
            );
        }
    }
}

pub struct SessionTui {
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
    start_time: Instant,
    interactive_mode: bool,
    status_message: String,
    debug_mode: bool,
    session_manager: Option<Arc<tokio::sync::RwLock<SessionManager>>>,
    session_id: Option<String>,
    vt100_parser: Parser,
}

pub struct SessionInfo {
    pub id: String,
    pub agent: String,
    pub _port: u16,
    pub working_dir: String,
    pub url: String,
}

impl SessionTui {
    /// Log a debug message if debug mode is enabled
    fn debug(&self, message: impl std::fmt::Display) {
        debug_log(self.debug_mode, message);
    }
    
    pub fn new(debug_mode: bool) -> Result<Self> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;

        Ok(SessionTui {
            terminal,
            start_time: Instant::now(),
            interactive_mode: false,
            status_message: "Ready - Press Ctrl+T for interactive mode".to_string(),
            debug_mode,
            session_manager: None,
            session_id: None,
            vt100_parser: Parser::new(30, 120, 0), // rows, cols, scrollback
        })
    }

    pub fn set_session_context(&mut self, session_manager: Arc<tokio::sync::RwLock<SessionManager>>, session_id: String) {
        self.session_manager = Some(session_manager);
        self.session_id = Some(session_id);
    }
    
    async fn resize_pty_to_match_tui(&self, terminal_area: Rect) {
        if let Some(session_manager) = &self.session_manager {
            if let Some(session_id) = &self.session_id {
                let manager = session_manager.read().await;
                if let Some(pty_session) = manager.sessions.get(session_id) {
                    let pty = pty_session.pty.lock().await;
                    let result = pty.resize(portable_pty::PtySize {
                        rows: terminal_area.height,
                        cols: terminal_area.width,
                        pixel_width: 0,
                        pixel_height: 0,
                    });
                    
                    if self.debug_mode {
                        match result {
                            Ok(_) => {
                                self.debug(format!("PTY resized successfully to {}x{}", 
                                    terminal_area.width, terminal_area.height));
                            }
                            Err(e) => {
                                self.debug(format!("PTY resize failed: {}", e));
                            }
                        }
                    }
                }
            }
        }
    }

    async fn send_input_to_pty(&self, key: &crossterm::event::KeyEvent) {
        self.debug(format!("send_input_to_pty called with key: {:?}", key));
        
        if let Some(session_manager) = &self.session_manager {
            if let Some(session_id) = &self.session_id {
                let manager = session_manager.read().await;
                if let Some(pty_session) = manager.sessions.get(session_id) {
                    // Convert crossterm key event to bytes for PTY
                    if let Some(input_bytes) = key_to_bytes(key) {
                        self.debug(format!("Sending to PTY: {:?} (bytes: {:?})", key, input_bytes));
                        
                        let mut writer = pty_session.writer.lock().await;
                        // Send the input
                        let write_result = writer.write_all(&input_bytes);
                        let flush_result = writer.flush();
                        
                        self.debug(format!("Write result: {:?}, Flush result: {:?}", write_result, flush_result));
                        
                        // For debugging: if this is Enter, also log that we sent a line terminator
                        if matches!(key.code, crossterm::event::KeyCode::Enter) {
                            self.debug("SENT ENTER - line should be processed now");
                        }
                    } else {
                        self.debug(format!("key_to_bytes returned None for key: {:?}", key));
                    }
                } else {
                    self.debug(format!("PTY session not found for id: {:?}", session_id));
                }
            } else {
                self.debug("No session_id set");
            }
        } else {
            self.debug("No session_manager set");
        }
    }

    async fn create_pty_output_stream(&self) -> mpsc::Receiver<Vec<u8>> {
        let (tx, rx) = mpsc::channel(100);
        
        if let Some(session_manager) = &self.session_manager {
            if let Some(session_id) = &self.session_id {
                let session_manager = session_manager.clone();
                let session_id = session_id.clone();
                let debug_mode = self.debug_mode;
                
                self.debug(format!("Creating PTY output stream for session: {}", session_id));
                
                // Spawn a task to continuously read from PTY
                tokio::spawn(async move {
                    debug_log(debug_mode, "PTY reader task started");
                    let mut buffer = [0u8; 4096];
                    
                    loop {
                        // Read from PTY with proper async locking
                        let manager = session_manager.read().await;
                        if let Some(pty_session) = manager.sessions.get(&session_id) {
                            // Use async lock instead of try_lock to avoid busy waiting
                            let mut reader = pty_session.reader.lock().await;
                            
                            match reader.read(&mut buffer) {
                                Ok(0) => {
                                    debug_log(debug_mode, "PTY reader reached EOF");
                                    break;
                                }
                                Ok(n) => {
                                    let data = buffer[..n].to_vec();
                                    debug_log(debug_mode, format!("PTY read {} bytes, sending to channel", data.len()));
                                    drop(reader); // Release lock before sending
                                    drop(manager); // Release manager lock too
                                    if tx.send(data).await.is_err() {
                                        debug_log(debug_mode, "Channel receiver dropped, exiting PTY reader");
                                        break; // Receiver dropped
                                    }
                                }
                                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                                    // No data available right now, sleep a bit
                                    drop(reader);
                                    drop(manager);
                                    tokio::time::sleep(Duration::from_millis(10)).await;
                                }
                                Err(e) => {
                                    debug_log(debug_mode, format!("PTY read error: {}", e));
                                    drop(reader);
                                    drop(manager);
                                    tokio::time::sleep(Duration::from_millis(100)).await;
                                }
                            }
                        } else {
                            debug_log(debug_mode, "Session not found in PTY reader");
                            break;
                        }
                    }
                });
            }
        }
        
        rx
    }

    pub async fn run(&mut self, session_info: SessionInfo) -> Result<()> {
        self.interactive_mode = false;
        self.status_message = "Ready - Press Ctrl+T for interactive mode".to_string();
        
        loop {
            if self.interactive_mode {
                self.run_interactive_mode(&session_info).await?;
            } else {
                self.run_monitoring_mode(&session_info).await?;
            }
        }
    }
    
    async fn run_monitoring_mode(&mut self, session_info: &SessionInfo) -> Result<()> {
        self.debug("=== ENTERING MONITORING MODE ===");
        
        use tokio::time::interval;
        let mut display_interval = interval(Duration::from_secs(1));
        let mut event_stream = EventStream::new();
        
        // Initial render
        let uptime = self.start_time.elapsed();
        self.draw(session_info, uptime)?;
        
        loop {
            tokio::select! {
                biased; // Ensure keyboard events get priority over display updates
                
                // Handle keyboard events from async stream (prioritize user input)
                maybe_event = event_stream.next() => {
                    match maybe_event {
                        Some(Ok(Event::Key(key))) => {
                            if key.kind == KeyEventKind::Press {
                                self.debug(format!("MONITORING MODE - Key: {:?} modifiers: {:?}", key.code, key.modifiers));
                            
                            // Handle quit
                            if key.code == KeyCode::Char('c') && key.modifiers.contains(event::KeyModifiers::CONTROL) {
                                return Ok(());
                            }
                            
                            // Handle toggle to interactive mode
                            if key.code == KeyCode::Char('t') && key.modifiers.contains(event::KeyModifiers::CONTROL) {
                                self.debug("SWITCHING TO INTERACTIVE MODE");
                                
                                self.interactive_mode = true;
                                self.status_message = "Interactive mode ON - Direct PTY input (Ctrl+T to toggle off)".to_string();
                                
                                // Resize PTY for interactive mode
                                let terminal_size = self.terminal.size()?;
                                let terminal_area = Rect {
                                    x: 0,
                                    y: 1, // Account for status bar
                                    width: terminal_size.width,
                                    height: terminal_size.height.saturating_sub(1),
                                };
                                self.vt100_parser.set_size(terminal_area.height, terminal_area.width);
                                self.resize_pty_to_match_tui(terminal_area).await;
                                
                                // Re-render and exit to switch modes
                                let uptime = self.start_time.elapsed();
                                self.draw(session_info, uptime)?;
                                return Ok(());
                            }
                            
                            // Handle other monitoring mode keys
                            match key.code {
                                KeyCode::Char('r') => {
                                    self.status_message = "Display refreshed".to_string();
                                    let uptime = self.start_time.elapsed();
                                    self.draw(session_info, uptime)?;
                                }
                                _ => {}
                            }
                            }
                        }
                        Some(Ok(_)) => {
                            // Other events (mouse, resize, etc.) - ignore for now
                        }
                        Some(Err(e)) => {
                            self.debug(format!("Event stream error: {:?}", e));
                            // Continue trying to read events
                        }
                        None => {
                            self.debug("Event stream terminated");
                            return Ok(()); // Exit if event stream ends
                        }
                    }
                }
                
                // Update display every second (lower priority)
                _ = display_interval.tick() => {
                    let uptime = self.start_time.elapsed();
                    self.draw(session_info, uptime)?;
                    
                    self.debug(format!("DISPLAY UPDATE - uptime: {}s", uptime.as_secs()));
                }
            }
        }
    }
    
    async fn run_interactive_mode(&mut self, session_info: &SessionInfo) -> Result<()> {
        self.debug("=== ENTERING INTERACTIVE MODE ===");
        
        let mut event_stream = EventStream::new();
        let mut pty_output_stream = self.create_pty_output_stream().await;
        
        // Add a periodic timer to keep the display updated
        use tokio::time::interval;
        let mut display_interval = interval(Duration::from_secs(1));
        
        // Add a rate limiter for PTY processing to prevent starvation
        let mut pty_throttle = interval(Duration::from_millis(50));
        
        // Initial render
        let uptime = self.start_time.elapsed();
        self.draw(session_info, uptime)?;
        
        // Debug the initial VT100 parser state
        let vt100_size = self.vt100_parser.screen().size();
        let terminal_size = self.terminal.size()?;
        self.debug(format!("Starting interactive mode loop - Terminal: {}x{}, VT100: {}x{}", 
            terminal_size.width, terminal_size.height, vt100_size.1, vt100_size.0));
        
        self.debug("Starting interactive mode loop");
        
        loop {
            tokio::select! {
                biased; // Process branches in order, ensuring timer gets a chance
                
                // Periodic display update (also serves as heartbeat)
                _ = display_interval.tick() => {
                    let uptime = self.start_time.elapsed();
                    self.debug(format!("Interactive mode heartbeat - uptime: {}s", uptime.as_secs()));
                    self.draw(session_info, uptime)?;
                }
                
                // Handle keyboard events from async stream (prioritize user input)
                maybe_event = event_stream.next() => {
                    match maybe_event {
                        Some(Ok(Event::Key(key))) => {
                            if key.kind == KeyEventKind::Press {
                                self.debug(format!("INTERACTIVE MODE - Key: {:?} modifiers: {:?}", key.code, key.modifiers));
                                
                                // Handle quit
                                if key.code == KeyCode::Char('c') && key.modifiers.contains(event::KeyModifiers::CONTROL) {
                                    return Ok(());
                                }
                                
                                // Handle toggle back to monitoring mode
                                if key.code == KeyCode::Char('t') && key.modifiers.contains(event::KeyModifiers::CONTROL) {
                                    self.debug("SWITCHING TO MONITORING MODE");
                                    
                                    self.interactive_mode = false;
                                    self.status_message = "Interactive mode OFF - Press Ctrl+T to toggle on".to_string();
                                    
                                    // Re-render and exit to switch modes
                                    let uptime = self.start_time.elapsed();
                                    self.draw(session_info, uptime)?;
                                    return Ok(());
                                }
                                
                                // Send all other keys to PTY
                                self.send_input_to_pty(&key).await;
                            }
                        }
                        Some(Ok(_)) => {
                            // Other events (mouse, resize, etc.) - ignore for now
                        }
                        Some(Err(e)) => {
                            self.debug(format!("Event stream error: {:?}", e));
                            // Continue trying to read events
                        }
                        None => {
                            self.debug("Event stream terminated");
                            return Ok(()); // Exit if event stream ends
                        }
                    }
                }
                
                // Handle PTY output (throttled to prevent starvation)
                _ = pty_throttle.tick() => {
                    // Try to drain multiple chunks at once, but limited per cycle
                    let mut chunks_processed = 0;
                    let max_chunks_per_cycle = 3; // Reduced to ensure fairness
                    
                    while chunks_processed < max_chunks_per_cycle {
                        match pty_output_stream.try_recv() {
                            Ok(data) => {
                                self.vt100_parser.process(&data);
                                
                                if self.debug_mode && chunks_processed == 0 {
                                    // Only log first chunk to avoid spam, but show cursor position
                                    let output_str = String::from_utf8_lossy(&data);
                                    let screen = self.vt100_parser.screen();
                                    self.debug(format!("PTY OUTPUT - {} bytes: {:?} | Cursor: ({}, {})", 
                                        data.len(), output_str, screen.cursor_position().0, screen.cursor_position().1));
                                }
                                
                                chunks_processed += 1;
                            }
                            Err(_) => break, // No more data available
                        }
                    }
                    
                    if chunks_processed > 0 {
                        if self.debug_mode {
                            self.debug(format!("Processed {} PTY chunks this cycle", chunks_processed));
                        }
                        
                        // Re-render once after processing batch
                        let uptime = self.start_time.elapsed();
                        self.draw(session_info, uptime)?;
                    }
                }
            }
        }
    }

    fn draw(&mut self, session_info: &SessionInfo, uptime: Duration) -> Result<()> {
        // Pre-compute terminal size and ensure VT100 parser matches if in interactive mode
        let terminal_size = self.terminal.size()?;
        if self.interactive_mode {
            let terminal_area_height = terminal_size.height.saturating_sub(1); // Account for status bar
            let terminal_area_width = terminal_size.width;
            
            let current_size = self.vt100_parser.screen().size();
            if current_size != (terminal_area_height, terminal_area_width) {
                self.vt100_parser.set_size(terminal_area_height, terminal_area_width);
                if self.debug_mode {
                    self.debug(format!("Resized VT100 parser from {}x{} to {}x{}", 
                        current_size.0, current_size.1,
                        terminal_area_height, terminal_area_width));
                }
            }
        }
        
        self.terminal.draw(|f| {
            let size = f.area();
            
            if self.interactive_mode {
                // Fullscreen interactive mode - just status bar and terminal
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(1),  // Minimal status bar
                        Constraint::Min(0),     // Full PTY terminal
                    ])
                    .split(size);

                // Minimal status bar
                let mode_text = format!("🚀 {} | 💬 INTERACTIVE | {} | Ctrl+T=Toggle | Ctrl+C=Exit", 
                    session_info.agent.to_uppercase(), 
                    format_duration(uptime)
                );
                let status_bar = Paragraph::new(mode_text)
                    .style(Style::default().bg(Color::Blue).fg(Color::White).add_modifier(Modifier::BOLD))
                    .alignment(Alignment::Center);
                f.render_widget(status_bar, chunks[0]);

                // PTY terminal area - show actual output via PseudoTerminal
                let terminal_area = chunks[1];
                
                let pseudo_terminal = PseudoTerminal::new(self.vt100_parser.screen())
                    .block(Block::default().borders(Borders::NONE));
                f.render_widget(pseudo_terminal, terminal_area);

            } else {
                // Normal monitoring mode layout
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(3),  // Header
                        Constraint::Min(10),    // Main content
                        Constraint::Length(3),  // Footer
                    ])
                    .split(size);

                // Header
                let header = Paragraph::new(format!("🚀 CodeMux - {} Agent Session", session_info.agent.to_uppercase()))
                    .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
                    .alignment(Alignment::Center)
                    .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::Blue)));
                f.render_widget(header, chunks[0]);

                // Main content area
                let content_chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(8),  // Session info
                        Constraint::Length(5),  // Status
                        Constraint::Min(3),     // Instructions
                    ])
                    .margin(1)
                    .split(chunks[1]);

                // Session information
                draw_session_info(f, content_chunks[0], session_info);
                
                // Status section
                draw_status(f, content_chunks[1], uptime, self.interactive_mode);
                
                // Instructions
                draw_instructions(f, content_chunks[2]);

                // Footer
                let footer = Paragraph::new("Press Ctrl+C to stop | Press Ctrl+T for interactive mode | Press 'r' to refresh")
                    .style(Style::default().fg(Color::Gray))
                    .alignment(Alignment::Center)
                    .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::Gray)));
                f.render_widget(footer, chunks[2]);
            }
        })?;

        Ok(())
    }

}

// Convert crossterm KeyEvent to bytes for PTY input
fn key_to_bytes(key: &crossterm::event::KeyEvent) -> Option<Vec<u8>> {
    use crossterm::event::{KeyCode, KeyModifiers};
    use std::io::Write;
    
    match key.code {
        KeyCode::Enter => Some(b"\r".to_vec()),
        KeyCode::Tab => Some(b"\t".to_vec()),
        KeyCode::Backspace => Some(b"\x7f".to_vec()),  // DEL character
        KeyCode::Delete => Some(b"\x1b[3~".to_vec()),  // Delete sequence
        KeyCode::Left => Some(b"\x1b[D".to_vec()),
        KeyCode::Right => Some(b"\x1b[C".to_vec()),
        KeyCode::Up => Some(b"\x1b[A".to_vec()),
        KeyCode::Down => Some(b"\x1b[B".to_vec()),
        KeyCode::Home => Some(b"\x1b[H".to_vec()),
        KeyCode::End => Some(b"\x1b[F".to_vec()),
        KeyCode::PageUp => Some(b"\x1b[5~".to_vec()),
        KeyCode::PageDown => Some(b"\x1b[6~".to_vec()),
        KeyCode::Esc => Some(b"\x1b".to_vec()),
        KeyCode::Char(c) => {
            if key.modifiers.contains(KeyModifiers::CONTROL) {
                // Handle Ctrl+key combinations (except Ctrl+I which we reserve)
                match c {
                    'a'..='z' => {
                        let ctrl_char = (c as u8) - b'a' + 1;
                        Some(vec![ctrl_char])
                    }
                    _ => None,
                }
            } else {
                // Regular character
                let mut bytes = Vec::new();
                let _ = write!(&mut bytes, "{}", c);
                Some(bytes)
            }
        }
        _ => None,
    }
}

fn draw_session_info(f: &mut Frame, area: Rect, session_info: &SessionInfo) {
        let info_block = Block::default()
            .title("📋 Session Information")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Green));

        let agent_upper = session_info.agent.to_uppercase();
        let info_lines = vec![
            Line::from(vec![
                Span::styled("🆔 Session ID: ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::raw(&session_info.id[..8]),
            ]),
            Line::from(vec![
                Span::styled("🌐 Web Interface: ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::styled(&session_info.url, Style::default().fg(Color::Blue).add_modifier(Modifier::UNDERLINED)),
            ]),
            Line::from(vec![
                Span::styled("📁 Working Directory: ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::raw(&session_info.working_dir),
            ]),
            Line::from(vec![
                Span::styled("🔧 Agent: ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::styled(&agent_upper, Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)),
            ]),
        ];

        let info_paragraph = Paragraph::new(info_lines)
            .block(info_block)
            .wrap(Wrap { trim: true });
        
        f.render_widget(info_paragraph, area);
}

fn draw_status(f: &mut Frame, area: Rect, uptime: Duration, interactive_mode: bool) {
        let status_block = Block::default()
            .title("⚡ Status")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Green));

        let uptime_str = format_duration(uptime);
        
        let mode_status = if interactive_mode {
            Span::styled("💬 Interactive", Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD))
        } else {
            Span::styled("👁️  Monitoring", Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD))
        };

        let status_lines = vec![
            Line::from(vec![
                Span::styled("Status: ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::styled("🟢 Running", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(vec![
                Span::styled("Mode: ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                mode_status,
            ]),
            Line::from(vec![
                Span::styled("Uptime: ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::raw(uptime_str),
            ]),
        ];

        let status_paragraph = Paragraph::new(status_lines)
            .block(status_block);
        
        f.render_widget(status_paragraph, area);
}

fn draw_instructions(f: &mut Frame, area: Rect) {
        let instructions_block = Block::default()
            .title("💡 Instructions")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan));

        let instructions = vec![
            Line::from("1. Open the web interface URL above in your browser"),
            Line::from("2. Interact with the AI agent through the web terminal"),
            Line::from("3. The session will persist until you stop it here"),
            Line::from(""),
            Line::from(vec![
                Span::styled("Tip: ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::raw("Keep this terminal open to maintain the session"),
            ]),
        ];

        let instructions_paragraph = Paragraph::new(instructions)
            .block(instructions_block)
            .wrap(Wrap { trim: true });
        
        f.render_widget(instructions_paragraph, area);
}

impl Drop for SessionTui {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(
            self.terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        );
        let _ = self.terminal.show_cursor();
    }
}

fn format_duration(duration: Duration) -> String {
    let total_seconds = duration.as_secs();
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;

    if hours > 0 {
        format!("{}h {}m {}s", hours, minutes, seconds)
    } else if minutes > 0 {
        format!("{}m {}s", minutes, seconds)
    } else {
        format!("{}s", seconds)
    }
}