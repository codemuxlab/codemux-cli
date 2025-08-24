use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
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

use std::sync::Arc;
use crate::session::SessionManager;

pub struct SessionTui {
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
    start_time: Instant,
    interactive_mode: bool,
    status_message: String,
    debug_mode: bool,
    session_manager: Option<Arc<tokio::sync::RwLock<SessionManager>>>,
    session_id: Option<String>,
    pty_buffer: String,
    max_buffer_lines: usize,
    needs_pty_resize: bool,
}

pub struct SessionInfo {
    pub id: String,
    pub agent: String,
    pub _port: u16,
    pub working_dir: String,
    pub url: String,
}

impl SessionTui {
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
            pty_buffer: String::new(),
            max_buffer_lines: 10000,
            needs_pty_resize: false,
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
                    let _ = pty.resize(portable_pty::PtySize {
                        rows: terminal_area.height,
                        cols: terminal_area.width,
                        pixel_width: 0,
                        pixel_height: 0,
                    });
                }
            }
        }
    }

    async fn send_input_to_pty(&self, key: &crossterm::event::KeyEvent) {
        if self.debug_mode {
            if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open("codemux_debug.log") {
                use std::io::Write;
                let _ = writeln!(file, "[{}] send_input_to_pty called with key: {:?}", 
                    chrono::Utc::now().format("%Y-%m-%d %H:%M:%S%.3f"), key);
            }
        }
        
        if let Some(session_manager) = &self.session_manager {
            if let Some(session_id) = &self.session_id {
                let manager = session_manager.read().await;
                if let Some(pty_session) = manager.sessions.get(session_id) {
                    // Convert crossterm key event to bytes for PTY
                    if let Some(input_bytes) = key_to_bytes(key) {
                        if self.debug_mode {
                            if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open("codemux_debug.log") {
                                use std::io::Write;
                                let _ = writeln!(file, "[{}] Sending to PTY: {:?} (bytes: {:?})", 
                                    chrono::Utc::now().format("%Y-%m-%d %H:%M:%S%.3f"), key, input_bytes);
                            }
                        }
                        
                        let pty = pty_session.pty.lock().await;
                        if let Ok(mut writer) = pty.take_writer() {
                            let _ = writer.write_all(&input_bytes);
                            let _ = writer.flush();
                        }
                    } else {
                        if self.debug_mode {
                            if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open("codemux_debug.log") {
                                use std::io::Write;
                                let _ = writeln!(file, "[{}] key_to_bytes returned None for key: {:?}", 
                                    chrono::Utc::now().format("%Y-%m-%d %H:%M:%S%.3f"), key);
                            }
                        }
                    }
                } else {
                    if self.debug_mode {
                        if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open("codemux_debug.log") {
                            use std::io::Write;
                            let _ = writeln!(file, "[{}] PTY session not found for id: {:?}", 
                                chrono::Utc::now().format("%Y-%m-%d %H:%M:%S%.3f"), session_id);
                        }
                    }
                }
            } else {
                if self.debug_mode {
                    if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open("codemux_debug.log") {
                        use std::io::Write;
                        let _ = writeln!(file, "[{}] No session_id set", 
                            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S%.3f"));
                    }
                }
            }
        } else {
            if self.debug_mode {
                if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open("codemux_debug.log") {
                    use std::io::Write;
                    let _ = writeln!(file, "[{}] No session_manager set", 
                        chrono::Utc::now().format("%Y-%m-%d %H:%M:%S%.3f"));
                }
            }
        }
    }

    async fn read_pty_output(&mut self) {
        if let Some(session_manager) = &self.session_manager {
            if let Some(session_id) = &self.session_id {
                let manager = session_manager.read().await;
                if let Some(pty_session) = manager.sessions.get(session_id) {
                    // Use try_lock to avoid blocking on the reader lock
                    if let Ok(mut reader) = pty_session.reader.try_lock() {
                        let mut buffer = [0u8; 1024]; // Smaller buffer for non-blocking reads
                        
                        // Only try to read once per call to avoid blocking
                        match reader.read(&mut buffer) {
                            Ok(0) => {
                                // EOF - PTY might be closed
                                if self.debug_mode {
                                    if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open("codemux_debug.log") {
                                        use std::io::Write;
                                        let _ = writeln!(file, "[{}] PTY reader reached EOF", 
                                            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S%.3f"));
                                    }
                                }
                            }
                            Ok(n) => {
                                let new_data = String::from_utf8_lossy(&buffer[..n]);
                                self.pty_buffer.push_str(&new_data);
                                
                                if self.debug_mode {
                                    if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open("codemux_debug.log") {
                                        use std::io::Write;
                                        let _ = writeln!(file, "[{}] Read {} bytes from PTY: {:?}", 
                                            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S%.3f"), n, new_data);
                                    }
                                }
                                
                                // Keep buffer size manageable
                                let lines: Vec<&str> = self.pty_buffer.lines().collect();
                                if lines.len() > self.max_buffer_lines {
                                    let keep_from = lines.len().saturating_sub(self.max_buffer_lines);
                                    self.pty_buffer = lines[keep_from..].join("\n");
                                }
                            }
                            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                                // No data available - this is expected for non-blocking reads
                            }
                            Err(e) => {
                                if self.debug_mode {
                                    if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open("codemux_debug.log") {
                                        use std::io::Write;
                                        let _ = writeln!(file, "[{}] PTY read error: {}", 
                                            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S%.3f"), e);
                                    }
                                }
                            }
                        }
                    } else {
                        // Reader is locked - skip this iteration
                        if self.debug_mode {
                            if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open("codemux_debug.log") {
                                use std::io::Write;
                                let _ = writeln!(file, "[{}] PTY reader is locked, skipping", 
                                    chrono::Utc::now().format("%Y-%m-%d %H:%M:%S%.3f"));
                            }
                        }
                    }
                }
            }
        }
    }

    pub async fn run(&mut self, session_info: SessionInfo) -> Result<()> {
        loop {
            let uptime = self.start_time.elapsed();
            
            
            // Read PTY output if in interactive mode
            if self.interactive_mode {
                self.read_pty_output().await;
            }
            
            self.draw(&session_info, uptime)?;
            
            // Resize PTY if we just entered interactive mode
            if self.needs_pty_resize && self.interactive_mode {
                if self.debug_mode {
                    if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open("codemux_debug.log") {
                        use std::io::Write;
                        let _ = writeln!(file, "[{}] Starting PTY resize", 
                            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S%.3f"));
                    }
                }
                
                // Get terminal size after draw
                let terminal_size = self.terminal.size()?;
                let terminal_area = Rect {
                    x: 0,
                    y: 1, // Account for status bar
                    width: terminal_size.width,
                    height: terminal_size.height.saturating_sub(1),
                };
                self.resize_pty_to_match_tui(terminal_area).await;
                self.needs_pty_resize = false;
                
                if self.debug_mode {
                    if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open("codemux_debug.log") {
                        use std::io::Write;
                        let _ = writeln!(file, "[{}] PTY resize completed", 
                            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S%.3f"));
                    }
                }
            }

            // Check for events with timeout
            if event::poll(Duration::from_millis(50))? {
                if let Event::Key(key) = event::read()? {
                    if key.kind == KeyEventKind::Press {
                        if self.debug_mode {
                            if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open("codemux_debug.log") {
                                use std::io::Write;
                                let _ = writeln!(file, "[{}] === KEY EVENT === Key: {:?} modifiers: {:?} Interactive: {}", 
                                    chrono::Utc::now().format("%Y-%m-%d %H:%M:%S%.3f"),
                                    key.code, key.modifiers, self.interactive_mode);
                            }
                        }
                        // Debug key presses
                        tracing::debug!("Key pressed: {:?} with modifiers: {:?}", key.code, key.modifiers);
                        
                        if self.debug_mode {
                            if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open("codemux_debug.log") {
                                use std::io::Write;
                                let is_ctrl_c = key.code == KeyCode::Char('c') && key.modifiers.contains(event::KeyModifiers::CONTROL);
                                let is_ctrl_t = key.code == KeyCode::Char('t') && key.modifiers.contains(event::KeyModifiers::CONTROL);
                                let _ = writeln!(file, "[{}] Key: {:?} modifiers: {:?} | Ctrl+C: {} | Ctrl+T: {} | Interactive: {}", 
                                    chrono::Utc::now().format("%Y-%m-%d %H:%M:%S%.3f"),
                                    key.code, key.modifiers, is_ctrl_c, is_ctrl_t, self.interactive_mode);
                            }
                        }
                        
                        // Universal quit - check first!
                        if key.code == KeyCode::Char('c') && key.modifiers.contains(event::KeyModifiers::CONTROL) {
                            break;
                        }

                        // Universal toggle with Ctrl+T
                        let is_toggle_key = key.code == KeyCode::Char('t') && key.modifiers.contains(event::KeyModifiers::CONTROL);
                        
                        if is_toggle_key {
                            self.interactive_mode = !self.interactive_mode;
                            if self.interactive_mode {
                                self.needs_pty_resize = true; // Flag that we need to resize
                                self.status_message = "Interactive mode ON - Direct PTY input (Ctrl+T to toggle off)".to_string();
                            } else {
                                self.status_message = "Interactive mode OFF - Press Ctrl+T to toggle on".to_string();
                            }
                            
                            if self.debug_mode {
                                if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open("codemux_debug.log") {
                                    use std::io::Write;
                                    let _ = writeln!(file, "[{}] Mode toggled - Interactive: {}", 
                                        chrono::Utc::now().format("%Y-%m-%d %H:%M:%S%.3f"), self.interactive_mode);
                                }
                            }
                            
                            // Skip to next iteration to redraw with new mode
                            continue;
                        }

                        if self.interactive_mode {
                            if self.debug_mode {
                                if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open("codemux_debug.log") {
                                    use std::io::Write;
                                    let _ = writeln!(file, "[{}] In interactive mode, calling send_input_to_pty", 
                                        chrono::Utc::now().format("%Y-%m-%d %H:%M:%S%.3f"));
                                }
                            }
                            // In interactive mode, pass all other input to PTY
                            self.send_input_to_pty(&key).await;
                        } else {
                            // In monitoring mode, handle TUI navigation
                            match key.code {
                                KeyCode::Char('r') => {
                                    self.status_message = "Display refreshed".to_string();
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    fn draw(&mut self, session_info: &SessionInfo, uptime: Duration) -> Result<()> {
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

                // PTY terminal area - show actual output
                let terminal_area = chunks[1];
                
                let terminal_paragraph = Paragraph::new(self.pty_buffer.as_str())
                    .style(Style::default().fg(Color::White).bg(Color::Black))
                    .wrap(Wrap { trim: false });
                f.render_widget(terminal_paragraph, terminal_area);

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