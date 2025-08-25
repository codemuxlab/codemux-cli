use anyhow::Result;
use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, EventStream, KeyCode, KeyEventKind,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame, Terminal,
};
use std::io;
use std::io::Read;
use tokio::time::{Duration, Instant};
// Removed VT100 parser - now consuming grid updates from PTY session
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use crate::pty_session::{PtyChannels, PtyInputMessage};
use crate::session::SessionManager;
use crate::tui_writer::{LogEntry, LogLevel};
use std::sync::Arc;

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

/// Convert VT100 color to CSS color string
fn format_color(color: vt100::Color) -> String {
    match color {
        vt100::Color::Default => "#ffffff".to_string(),
        vt100::Color::Idx(0) => "#000000".to_string(),
        vt100::Color::Idx(1) => "#800000".to_string(),
        vt100::Color::Idx(2) => "#008000".to_string(),
        vt100::Color::Idx(3) => "#808000".to_string(),
        vt100::Color::Idx(4) => "#000080".to_string(),
        vt100::Color::Idx(5) => "#800080".to_string(),
        vt100::Color::Idx(6) => "#008080".to_string(),
        vt100::Color::Idx(7) => "#c0c0c0".to_string(),
        vt100::Color::Idx(8) => "#808080".to_string(),
        vt100::Color::Idx(9) => "#ff0000".to_string(),
        vt100::Color::Idx(10) => "#00ff00".to_string(),
        vt100::Color::Idx(11) => "#ffff00".to_string(),
        vt100::Color::Idx(12) => "#0000ff".to_string(),
        vt100::Color::Idx(13) => "#ff00ff".to_string(),
        vt100::Color::Idx(14) => "#00ffff".to_string(),
        vt100::Color::Idx(15) => "#ffffff".to_string(),
        vt100::Color::Idx(n) if n >= 16 && n < 232 => {
            // 216-color cube
            let n = n - 16;
            let r = (n / 36) * 51;
            let g = ((n % 36) / 6) * 51;
            let b = (n % 6) * 51;
            format!("#{:02x}{:02x}{:02x}", r, g, b)
        }
        vt100::Color::Idx(n) if n >= 232 => {
            // 24-level grayscale
            let level = (n - 232) * 10 + 8;
            format!("#{:02x}{:02x}{:02x}", level, level, level)
        }
        vt100::Color::Rgb(r, g, b) => format!("#{:02x}{:02x}{:02x}", r, g, b),
        _ => "#ffffff".to_string(),
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct GridCell {
    pub char: char,
    pub fg_color: Option<String>,
    pub bg_color: Option<String>,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CursorPosition {
    pub row: u16,
    pub col: u16,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GridSize {
    pub rows: u16,
    pub cols: u16,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GridUpdate {
    pub r#type: String, // "grid_update"
    pub cursor: CursorPosition,
    pub size: GridSize,
    pub cells: Vec<(u16, u16, GridCell)>, // (row, col, cell)
}

pub struct SessionTui {
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
    start_time: Instant,
    interactive_mode: bool,
    status_message: String,
    debug_mode: bool,
    session_manager: Option<Arc<tokio::sync::RwLock<SessionManager>>>,
    session_id: Option<String>,
    system_logs: Vec<LogEntry>,
    // Terminal state from PTY session grid updates
    terminal_grid: std::collections::HashMap<(u16, u16), crate::pty_session::GridCell>,
    terminal_cursor: (u16, u16),
    terminal_size: (u16, u16),
    websocket_broadcast_tx: Option<tokio::sync::broadcast::Sender<String>>,
    // New channel-based PTY communication
    pty_channels: Option<PtyChannels>,
    // Incremental rendering state
    needs_redraw: bool,
    dirty_cells: std::collections::HashSet<(u16, u16)>,
    cursor_dirty: bool,
    last_render_time: std::time::Instant,
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
            system_logs: Vec::new(),
            terminal_grid: std::collections::HashMap::new(),
            terminal_cursor: (0, 0),
            terminal_size: (30, 120), // Default size
            websocket_broadcast_tx: None,
            pty_channels: None,
            needs_redraw: true,
            dirty_cells: std::collections::HashSet::new(),
            cursor_dirty: false,
            last_render_time: std::time::Instant::now(),
        })
    }

    pub fn set_session_context(
        &mut self,
        session_manager: Arc<tokio::sync::RwLock<SessionManager>>,
        session_id: String,
    ) {
        self.session_manager = Some(session_manager);
        self.session_id = Some(session_id);
    }

    pub fn set_websocket_broadcast(&mut self, tx: tokio::sync::broadcast::Sender<String>) {
        self.websocket_broadcast_tx = Some(tx);
    }

    pub fn set_pty_channels(&mut self, channels: PtyChannels) {
        self.pty_channels = Some(channels);
    }

    // Old VT100-based methods removed - now using grid updates from PTY session

    pub async fn initial_pty_resize(&mut self) -> Result<()> {
        // Get current terminal size and resize PTY to match
        let terminal_area = match self.terminal.size() {
            Ok(size) => Rect {
                x: 0,
                y: 0,
                width: size.width,
                height: size.height,
            },
            Err(e) => {
                // Fallback for headless environments (CI, containers, etc.)
                // Check for environment variables first
                let width = std::env::var("COLUMNS")
                    .ok()
                    .and_then(|s| s.parse::<u16>().ok())
                    .unwrap_or(120);
                let height = std::env::var("LINES")
                    .ok()
                    .and_then(|s| s.parse::<u16>().ok())
                    .unwrap_or(30);

                tracing::warn!(
                    "Could not detect terminal size: {}. Using fallback size {}x{}",
                    e,
                    width,
                    height
                );

                Rect {
                    x: 0,
                    y: 0,
                    width,
                    height,
                }
            }
        };

        self.resize_pty_to_match_tui(terminal_area).await;

        // Update terminal size tracking
        self.terminal_size = (terminal_area.height, terminal_area.width);

        Ok(())
    }

    pub fn add_system_log(&mut self, log_entry: LogEntry) {
        self.system_logs.push(log_entry);

        // Keep only last 10 log entries to prevent memory growth
        if self.system_logs.len() > 10 {
            self.system_logs.remove(0);
        }
    }

    /// Mark specific cells as dirty for incremental rendering
    fn mark_cells_dirty(&mut self, cells: &[(u16, u16)]) {
        for &(row, col) in cells {
            self.dirty_cells.insert((row, col));
        }
        self.needs_redraw = true;
    }

    /// Mark cursor as dirty for incremental rendering
    fn mark_cursor_dirty(&mut self, old_cursor: (u16, u16), new_cursor: (u16, u16)) {
        if old_cursor != new_cursor {
            self.cursor_dirty = true;
            self.needs_redraw = true;
        }
    }

    /// Check if enough time has passed to warrant a redraw (for batching)
    fn should_redraw_now(&self) -> bool {
        if !self.needs_redraw {
            return false;
        }
        
        // Always redraw immediately in interactive mode for responsiveness
        if self.interactive_mode {
            return true;
        }
        
        let elapsed = self.last_render_time.elapsed().as_millis();
        
        // In monitoring mode, batch updates (redraw at most every 50ms)
        // But force redraw after 200ms to prevent stuck updates
        elapsed >= 50 || elapsed >= 200
    }

    /// Clear all dirty state after a successful redraw
    fn clear_dirty_state(&mut self) {
        self.needs_redraw = false;
        self.dirty_cells.clear();
        self.cursor_dirty = false;
        self.last_render_time = std::time::Instant::now();
    }

    /// Force a full redraw (for keyframes or major updates)
    fn mark_full_redraw(&mut self) {
        self.needs_redraw = true;
        self.dirty_cells.clear(); // Clear because we're doing full redraw
        self.cursor_dirty = true;
    }

    async fn resize_pty_to_match_tui(&self, terminal_area: Rect) {
        if let Some(channels) = &self.pty_channels {
            let resize_msg = crate::pty_session::PtyControlMessage::Resize {
                rows: terminal_area.height,
                cols: terminal_area.width,
            };

            if let Err(e) = channels.control_tx.send(resize_msg) {
                self.debug(format!("Failed to send PTY resize command: {}", e));
            } else {
                self.debug(format!(
                    "Sent PTY resize command to {}x{}",
                    terminal_area.width, terminal_area.height
                ));
            }
        } else {
            self.debug("No PTY channels available for resize");
        }
    }

    async fn send_input_to_pty(&self, key: &crossterm::event::KeyEvent) {
        self.debug(format!("send_input_to_pty called with key: {:?}", key));

        if let Some(channels) = &self.pty_channels {
            // Convert crossterm key event to bytes for PTY
            if let Some(input_bytes) = key_to_bytes(key) {
                self.debug(format!(
                    "Sending to PTY: {:?} (bytes: {:?})",
                    key, input_bytes
                ));

                let input_msg = PtyInputMessage {
                    data: input_bytes,
                    client_id: "tui".to_string(),
                };

                if let Err(e) = channels.input_tx.send(input_msg) {
                    self.debug(format!("Failed to send input to PTY: {}", e));
                } else {
                    // For debugging: if this is Enter, also log that we sent a line terminator
                    if matches!(key.code, crossterm::event::KeyCode::Enter) {
                        self.debug("SENT ENTER - line should be processed now");
                    }
                }
            } else {
                self.debug(format!("key_to_bytes returned None for key: {:?}", key));
            }
        } else {
            self.debug("No PTY channels available for input");
        }
    }

    async fn create_pty_output_stream(&self) -> mpsc::Receiver<Vec<u8>> {
        let (tx, rx) = mpsc::channel(100);

        if let Some(session_manager) = &self.session_manager {
            if let Some(session_id) = &self.session_id {
                let session_manager = session_manager.clone();
                let session_id = session_id.clone();
                let debug_mode = self.debug_mode;

                self.debug(format!(
                    "Creating PTY output stream for session: {}",
                    session_id
                ));

                // Spawn a task to continuously read from PTY
                tokio::spawn(async move {
                    debug_log(debug_mode, "PTY reader task started");
                    let mut buffer = [0u8; 4096];

                    loop {
                        // Get the reader in a scoped block to release manager lock quickly
                        let reader_arc = {
                            let manager = session_manager.read().await;
                            if let Some(pty_session) = manager.sessions.get(&session_id) {
                                Some(pty_session.reader.clone())
                            } else {
                                None
                            }
                        }; // manager lock is released here

                        if let Some(reader_arc) = reader_arc {
                            // Now lock the reader without holding the manager lock
                            let mut reader = reader_arc.lock().await;

                            match reader.read(&mut buffer) {
                                Ok(0) => {
                                    debug_log(debug_mode, "PTY reader reached EOF");
                                    break;
                                }
                                Ok(n) => {
                                    let data = buffer[..n].to_vec();
                                    debug_log(
                                        debug_mode,
                                        format!(
                                            "PTY read {} bytes, sending to channel",
                                            data.len()
                                        ),
                                    );
                                    drop(reader); // Release reader lock before sending
                                    if tx.send(data).await.is_err() {
                                        debug_log(
                                            debug_mode,
                                            "Channel receiver dropped, exiting PTY reader",
                                        );
                                        break; // Receiver dropped
                                    }
                                }
                                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                                    // No data available right now, sleep a bit
                                    drop(reader);
                                    tokio::time::sleep(Duration::from_millis(10)).await;
                                }
                                Err(e) => {
                                    debug_log(debug_mode, format!("PTY read error: {}", e));
                                    drop(reader);
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

    pub async fn run(
        &mut self,
        session_info: SessionInfo,
        mut log_rx: tokio::sync::mpsc::UnboundedReceiver<LogEntry>,
    ) -> Result<()> {
        self.interactive_mode = false;
        self.status_message = "Ready - Press Ctrl+T for interactive mode".to_string();
        
        // Perform initial PTY resize to match current terminal size
        if let Err(e) = self.initial_pty_resize().await {
            self.debug(format!("Failed to perform initial PTY resize: {}", e));
        }

        loop {
            tokio::select! {
                // Handle incoming system logs
                log_entry = log_rx.recv() => {
                    if let Some(log_entry) = log_entry {
                        self.add_system_log(log_entry);
                        // Re-render to show the new log entry
                        let uptime = self.start_time.elapsed();
                        let _ = self.draw(&session_info, uptime);
                    }
                }

                // Run the current mode
                should_quit = async {
                    if self.interactive_mode {
                        self.run_interactive_mode(&session_info).await
                    } else {
                        self.run_monitoring_mode(&session_info).await
                    }
                } => {
                    self.debug(format!("Mode returned: {:?}", should_quit));

                    match should_quit {
                        Ok(true) => {
                            self.debug("User requested quit, breaking loop");
                            break; // User wants to quit
                        }
                        Ok(false) => {
                            self.debug("Mode switch, continuing loop");
                            continue; // Mode switch, continue loop
                        }
                        Err(e) => {
                            self.debug(format!("Error occurred: {:?}", e));
                            // Ensure cleanup happens on error
                            self.cleanup();
                            return Err(e);
                        }
                    }
                }
            }
        }

        self.debug("Exiting TUI, performing cleanup");
        // Ensure cleanup happens on normal exit
        self.cleanup();
        Ok(())
    }

    fn cleanup(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(
            self.terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        );
        let _ = self.terminal.show_cursor();
    }

    async fn run_monitoring_mode(&mut self, session_info: &SessionInfo) -> Result<bool> {
        self.debug("=== ENTERING MONITORING MODE ===");

        use tokio::time::interval;
        let mut display_interval = interval(Duration::from_secs(1));
        let mut event_stream = EventStream::new();

        // Initial render
        let uptime = self.start_time.elapsed();
        self.draw(session_info, uptime)?;
        self.clear_dirty_state();

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
                                    return Ok(true); // Signal to quit
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
                                    self.terminal_size = (terminal_area.height, terminal_area.width);
                                    self.resize_pty_to_match_tui(terminal_area).await;

                                    // Re-render and exit to switch modes
                                    let uptime = self.start_time.elapsed();
                                    self.draw(session_info, uptime)?;
                                    return Ok(false); // Switch modes
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
                        Some(Ok(Event::Resize(width, height))) => {
                            self.debug(format!("Terminal resized to {}x{}", width, height));
                            // Terminal was resized, update display
                            let uptime = self.start_time.elapsed();
                            self.draw(session_info, uptime)?;
                        }
                        Some(Ok(_)) => {
                            // Other events (mouse, etc.) - ignore
                        }
                        Some(Err(e)) => {
                            self.debug(format!("Event stream error: {:?}", e));
                            // Continue trying to read events
                        }
                        None => {
                            self.debug("Event stream terminated");
                            return Ok(true); // Exit if event stream ends
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

    async fn run_interactive_mode(&mut self, session_info: &SessionInfo) -> Result<bool> {
        self.debug("=== ENTERING INTERACTIVE MODE ===");

        // Request keyframe for current terminal state when entering interactive mode
        if let Some(channels) = &self.pty_channels {
            self.debug("Requesting keyframe for TUI interactive mode");
            match channels.request_keyframe().await {
                Ok(keyframe) => {
                    self.debug("Received keyframe for TUI interactive mode");
                    // Apply keyframe to TUI terminal state
                    match keyframe {
                        crate::pty_session::GridUpdateMessage::Keyframe {
                            size,
                            cells,
                            cursor,
                            ..
                        } => {
                            // Update terminal state from keyframe and mark for full redraw
                            self.terminal_grid = cells;
                            self.terminal_cursor = cursor;
                            self.terminal_size = (size.rows, size.cols);
                            self.mark_full_redraw();

                            self.debug(format!(
                                "Applied keyframe with {} cells, cursor at ({}, {}), size {}x{}",
                                self.terminal_grid.len(),
                                cursor.0,
                                cursor.1,
                                size.rows,
                                size.cols
                            ));
                        }
                        crate::pty_session::GridUpdateMessage::Diff { .. } => {
                            self.debug("Received diff instead of keyframe (unexpected)");
                        }
                    }
                }
                Err(e) => {
                    self.debug(format!("Failed to request keyframe for TUI: {}", e));
                }
            }
        } else {
            self.debug("No PTY channels available for keyframe request");
        }

        let mut event_stream = EventStream::new();
        let mut grid_update_stream: Option<
            tokio::sync::broadcast::Receiver<crate::pty_session::GridUpdateMessage>,
        > = if let Some(channels) = &self.pty_channels {
            Some(channels.grid_tx.subscribe())
        } else {
            self.debug("No PTY channels available, using dummy stream");
            // Create dummy receiver that never receives anything
            let (_, rx) =
                tokio::sync::broadcast::channel::<crate::pty_session::GridUpdateMessage>(1);
            Some(rx)
        };

        // Add a periodic timer to keep the display updated
        use tokio::time::interval;
        let mut display_interval = interval(Duration::from_secs(1));

        // Add a rate limiter for PTY processing to prevent starvation
        let mut pty_throttle = interval(Duration::from_millis(50));

        // Initial render
        let uptime = self.start_time.elapsed();
        self.draw(session_info, uptime)?;
        self.clear_dirty_state();

        // Debug the initial terminal state
        let terminal_size = self.terminal.size()?;
        self.debug(format!(
            "Starting interactive mode loop - Terminal: {}x{}, Grid size: {}x{}",
            terminal_size.width, terminal_size.height, self.terminal_size.1, self.terminal_size.0
        ));

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
                                    return Ok(true); // Signal to quit
                                }

                                // Handle toggle back to monitoring mode
                                if key.code == KeyCode::Char('t') && key.modifiers.contains(event::KeyModifiers::CONTROL) {
                                    self.debug("SWITCHING TO MONITORING MODE");

                                    self.interactive_mode = false;
                                    self.status_message = "Interactive mode OFF - Press Ctrl+T to toggle on".to_string();

                                    // Re-render and exit to switch modes
                                    let uptime = self.start_time.elapsed();
                                    self.draw(session_info, uptime)?;
                                    return Ok(false); // Switch modes
                                }

                                // Send all other keys to PTY
                                self.send_input_to_pty(&key).await;
                            }
                        }
                        Some(Ok(Event::Resize(width, height))) => {
                            self.debug(format!("Terminal resized to {}x{} in interactive mode", width, height));

                            // Update terminal size tracking
                            let terminal_area = Rect {
                                x: 0,
                                y: 1, // Account for status bar
                                width,
                                height: height.saturating_sub(1),
                            };
                            self.terminal_size = (terminal_area.height, terminal_area.width);
                            self.mark_full_redraw(); // Terminal resize requires full redraw

                            // Resize PTY to match new terminal size
                            self.resize_pty_to_match_tui(terminal_area).await;

                            // Redraw with new size
                            let uptime = self.start_time.elapsed();
                            self.draw(session_info, uptime)?;
                            self.clear_dirty_state();
                        }
                        Some(Ok(_)) => {
                            // Other events (mouse, etc.) - ignore
                        }
                        Some(Err(e)) => {
                            self.debug(format!("Event stream error: {:?}", e));
                            // Continue trying to read events
                        }
                        None => {
                            self.debug("Event stream terminated");
                            return Ok(true); // Exit if event stream ends
                        }
                    }
                }

                // Handle grid updates from PTY session (throttled to prevent starvation)
                _ = pty_throttle.tick() => {
                    // Try to drain multiple grid updates at once, but limited per cycle
                    let mut updates_processed = 0;
                    let max_updates_per_cycle = 3; // Reduced to ensure fairness

                    if let Some(ref mut stream) = grid_update_stream {
                        while updates_processed < max_updates_per_cycle {
                            match stream.try_recv() {
                                Ok(update) => {
                                    // Apply grid update to TUI terminal state
                                    match update {
                                        crate::pty_session::GridUpdateMessage::Keyframe { size, cells, cursor, .. } => {
                                            // Keyframes require full redraw
                                            self.terminal_grid = cells;
                                            self.terminal_cursor = cursor;
                                            self.terminal_size = (size.rows, size.cols);
                                            self.mark_full_redraw();

                                            if self.debug_mode && updates_processed == 0 {
                                                self.debug(format!("GRID KEYFRAME - {} cells, cursor: ({}, {}), size: {}x{}",
                                                    self.terminal_grid.len(), cursor.0, cursor.1, size.rows, size.cols));
                                            }
                                        }
                                        crate::pty_session::GridUpdateMessage::Diff { changes, cursor, .. } => {
                                            let num_changes = changes.len();
                                            
                                            // Collect dirty cell positions for incremental rendering
                                            let dirty_positions: Vec<(u16, u16)> = changes.iter()
                                                .map(|(row, col, _)| (*row, *col))
                                                .collect();

                                            // Apply changes to terminal grid
                                            for (row, col, cell) in changes {
                                                self.terminal_grid.insert((row, col), cell);
                                            }

                                            // Mark changed cells as dirty for incremental rendering
                                            self.mark_cells_dirty(&dirty_positions);

                                            // Update cursor if specified
                                            if let Some(new_cursor) = cursor {
                                                self.mark_cursor_dirty(self.terminal_cursor, new_cursor);
                                                self.terminal_cursor = new_cursor;
                                            }

                                            if self.debug_mode && updates_processed == 0 {
                                                self.debug(format!("GRID DIFF - {} changes, cursor: ({}, {}), marked {} cells dirty",
                                                    num_changes, self.terminal_cursor.0, self.terminal_cursor.1, dirty_positions.len()));
                                            }
                                        }
                                    }

                                    updates_processed += 1;
                                }
                                Err(tokio::sync::broadcast::error::TryRecvError::Empty) => break, // No more data available
                                Err(tokio::sync::broadcast::error::TryRecvError::Lagged(_)) => {
                                    self.debug("Grid update stream lagged, some messages may have been missed");
                                    continue; // Try to get the next message
                                }
                                Err(tokio::sync::broadcast::error::TryRecvError::Closed) => {
                                    self.debug("Grid update stream closed");
                                    break;
                                }
                            }
                        }
                    }

                    // Only redraw if we have changes and enough time has passed (batching)
                    if updates_processed > 0 && self.should_redraw_now() {
                        if self.debug_mode {
                            if self.dirty_cells.is_empty() && self.needs_redraw {
                                self.debug(format!("Processed {} grid updates, performing full redraw", updates_processed));
                            } else {
                                self.debug(format!("Processed {} grid updates, redrawing {} dirty cells", 
                                    updates_processed, self.dirty_cells.len()));
                            }
                        }

                        let uptime = self.start_time.elapsed();
                        self.draw(session_info, uptime)?;
                        self.clear_dirty_state();
                    } else if updates_processed > 0 {
                        if self.debug_mode {
                            self.debug(format!("Processed {} grid updates, batching (dirty cells: {}, time since last: {}ms)", 
                                updates_processed, self.dirty_cells.len(), self.last_render_time.elapsed().as_millis()));
                        }
                    }
                }
            }
        }
    }

    fn draw(&mut self, session_info: &SessionInfo, uptime: Duration) -> Result<()> {
        // Pre-compute terminal size and update tracking if in interactive mode
        let terminal_size = self.terminal.size()?;
        if self.interactive_mode {
            let terminal_area_height = terminal_size.height.saturating_sub(1); // Account for status bar
            let terminal_area_width = terminal_size.width;

            // Update our terminal size tracking if it changed
            if self.terminal_size != (terminal_area_height, terminal_area_width) {
                self.terminal_size = (terminal_area_height, terminal_area_width);
                if self.debug_mode {
                    self.debug(format!(
                        "Updated terminal size to {}x{}",
                        terminal_area_height, terminal_area_width
                    ));
                }
            }
        }

        // Extract needed data before the draw closure to avoid borrowing issues
        let interactive_mode = self.interactive_mode;
        let terminal_grid = self.terminal_grid.clone();
        let terminal_cursor = self.terminal_cursor;
        let terminal_grid_size = self.terminal_size;
        let system_logs = self.system_logs.clone();

        self.terminal.draw(move |f| {
            let size = f.area();
            if interactive_mode {
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

                // PTY terminal area - render from grid state
                let terminal_area = chunks[1];
                // Create terminal content from grid state
                let terminal_content = render_terminal_from_grid(&terminal_grid, terminal_grid_size, terminal_cursor, terminal_area.height, terminal_area.width);
                let terminal_widget = Paragraph::new(terminal_content)
                    .block(Block::default().borders(Borders::NONE))
                    .wrap(ratatui::widgets::Wrap { trim: false });
                f.render_widget(terminal_widget, terminal_area);

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
                        Constraint::Length(5),  // System errors
                        Constraint::Min(3),     // Instructions
                    ])
                    .margin(1)
                    .split(chunks[1]);

                // Session information
                draw_session_info(f, content_chunks[0], session_info);
                // Status section
                draw_status(f, content_chunks[1], uptime, interactive_mode);
                // System logs section  
                draw_system_logs(f, content_chunks[2], &system_logs);
                // Instructions
                draw_instructions(f, content_chunks[3]);

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

    // No longer needed - moved to standalone function below
}

/// Render terminal content from grid state for display
fn render_terminal_from_grid(
    terminal_grid: &std::collections::HashMap<(u16, u16), crate::pty_session::GridCell>,
    terminal_size: (u16, u16),
    cursor_pos: (u16, u16),
    display_height: u16,
    display_width: u16,
) -> Vec<ratatui::text::Line> {
    let (grid_rows, grid_cols) = terminal_size;
    let mut lines = Vec::new();

    // Render each row of the terminal
    for row in 0..std::cmp::min(grid_rows, display_height) {
        let mut line_spans = Vec::new();
        let mut current_line = String::new();
        let mut current_style = Style::default();

        // Build line from grid cells
        for col in 0..std::cmp::min(grid_cols, display_width) {
            let is_cursor = (row, col) == cursor_pos;
            
            if let Some(cell) = terminal_grid.get(&(row, col)) {
                // Convert grid cell to styled content
                let mut cell_style = Style::default()
                    .fg(cell
                        .fg_color
                        .as_ref()
                        .and_then(|c| parse_hex_color(c))
                        .unwrap_or(Color::Reset))
                    .bg(cell
                        .bg_color
                        .as_ref()
                        .and_then(|c| parse_hex_color(c))
                        .unwrap_or(Color::Reset))
                    .add_modifier(if cell.bold {
                        Modifier::BOLD
                    } else {
                        Modifier::empty()
                    })
                    .add_modifier(if cell.italic {
                        Modifier::ITALIC
                    } else {
                        Modifier::empty()
                    })
                    .add_modifier(if cell.underline {
                        Modifier::UNDERLINED
                    } else {
                        Modifier::empty()
                    });

                // Highlight cursor position with reversed colors
                if is_cursor {
                    cell_style = cell_style.add_modifier(Modifier::REVERSED);
                }

                // If style changed, flush current span and start new one
                if cell_style != current_style && !current_line.is_empty() {
                    line_spans.push(Span::styled(current_line.clone(), current_style));
                    current_line.clear();
                }

                current_line.push_str(&cell.char);
                current_style = cell_style;
            } else {
                // Empty cell - use space, but highlight if cursor is here
                let mut empty_style = Style::default();
                if is_cursor {
                    empty_style = empty_style.add_modifier(Modifier::REVERSED);
                }
                
                // If style changed, flush current span
                if empty_style != current_style && !current_line.is_empty() {
                    line_spans.push(Span::styled(current_line.clone(), current_style));
                    current_line.clear();
                }
                
                current_line.push(' ');
                current_style = empty_style;
            }
        }

        // Add final span if there's content
        if !current_line.is_empty() {
            line_spans.push(Span::styled(current_line, current_style));
        } else if line_spans.is_empty() {
            // Completely empty line
            line_spans.push(Span::raw(" "));
        }

        lines.push(Line::from(line_spans));
    }

    // Fill remaining display lines with empty content if needed
    while lines.len() < display_height as usize {
        lines.push(Line::from(" "));
    }

    lines
}

/// Parse hex color string to ratatui Color
fn parse_hex_color(hex: &str) -> Option<Color> {
    if hex.len() != 7 || !hex.starts_with('#') {
        return None;
    }

    let r = u8::from_str_radix(&hex[1..3], 16).ok()?;
    let g = u8::from_str_radix(&hex[3..5], 16).ok()?;
    let b = u8::from_str_radix(&hex[5..7], 16).ok()?;

    Some(Color::Rgb(r, g, b))
}

// Convert crossterm KeyEvent to bytes for PTY input
fn key_to_bytes(key: &crossterm::event::KeyEvent) -> Option<Vec<u8>> {
    use crossterm::event::{KeyCode, KeyModifiers};
    use std::io::Write;

    match key.code {
        KeyCode::Enter => Some(b"\r".to_vec()),
        KeyCode::Tab => {
            if key.modifiers.contains(KeyModifiers::SHIFT) {
                Some(b"\x1b[Z".to_vec()) // Shift+Tab (CSI Z)
            } else {
                Some(b"\t".to_vec())
            }
        }
        KeyCode::BackTab => Some(b"\x1b[Z".to_vec()), // BackTab (Shift+Tab)
        KeyCode::Backspace => Some(b"\x7f".to_vec()), // DEL character
        KeyCode::Delete => Some(b"\x1b[3~".to_vec()), // Delete sequence
        KeyCode::Insert => Some(b"\x1b[2~".to_vec()), // Insert
        KeyCode::Left => {
            if key.modifiers.contains(KeyModifiers::SHIFT) {
                Some(b"\x1b[1;2D".to_vec()) // Shift+Left
            } else if key.modifiers.contains(KeyModifiers::ALT) {
                Some(b"\x1b[1;3D".to_vec()) // Alt+Left
            } else if key.modifiers.contains(KeyModifiers::CONTROL) {
                Some(b"\x1b[1;5D".to_vec()) // Ctrl+Left
            } else {
                Some(b"\x1b[D".to_vec())
            }
        }
        KeyCode::Right => {
            if key.modifiers.contains(KeyModifiers::SHIFT) {
                Some(b"\x1b[1;2C".to_vec()) // Shift+Right
            } else if key.modifiers.contains(KeyModifiers::ALT) {
                Some(b"\x1b[1;3C".to_vec()) // Alt+Right
            } else if key.modifiers.contains(KeyModifiers::CONTROL) {
                Some(b"\x1b[1;5C".to_vec()) // Ctrl+Right
            } else {
                Some(b"\x1b[C".to_vec())
            }
        }
        KeyCode::Up => {
            if key.modifiers.contains(KeyModifiers::SHIFT) {
                Some(b"\x1b[1;2A".to_vec()) // Shift+Up
            } else if key.modifiers.contains(KeyModifiers::ALT) {
                Some(b"\x1b[1;3A".to_vec()) // Alt+Up
            } else if key.modifiers.contains(KeyModifiers::CONTROL) {
                Some(b"\x1b[1;5A".to_vec()) // Ctrl+Up
            } else {
                Some(b"\x1b[A".to_vec())
            }
        }
        KeyCode::Down => {
            if key.modifiers.contains(KeyModifiers::SHIFT) {
                Some(b"\x1b[1;2B".to_vec()) // Shift+Down
            } else if key.modifiers.contains(KeyModifiers::ALT) {
                Some(b"\x1b[1;3B".to_vec()) // Alt+Down
            } else if key.modifiers.contains(KeyModifiers::CONTROL) {
                Some(b"\x1b[1;5B".to_vec()) // Ctrl+Down
            } else {
                Some(b"\x1b[B".to_vec())
            }
        }
        KeyCode::Home => {
            if key.modifiers.contains(KeyModifiers::SHIFT) {
                Some(b"\x1b[1;2H".to_vec()) // Shift+Home
            } else if key.modifiers.contains(KeyModifiers::CONTROL) {
                Some(b"\x1b[1;5H".to_vec()) // Ctrl+Home
            } else {
                Some(b"\x1b[H".to_vec())
            }
        }
        KeyCode::End => {
            if key.modifiers.contains(KeyModifiers::SHIFT) {
                Some(b"\x1b[1;2F".to_vec()) // Shift+End
            } else if key.modifiers.contains(KeyModifiers::CONTROL) {
                Some(b"\x1b[1;5F".to_vec()) // Ctrl+End
            } else {
                Some(b"\x1b[F".to_vec())
            }
        }
        KeyCode::PageUp => {
            if key.modifiers.contains(KeyModifiers::SHIFT) {
                Some(b"\x1b[5;2~".to_vec()) // Shift+PageUp
            } else if key.modifiers.contains(KeyModifiers::CONTROL) {
                Some(b"\x1b[5;5~".to_vec()) // Ctrl+PageUp
            } else {
                Some(b"\x1b[5~".to_vec())
            }
        }
        KeyCode::PageDown => {
            if key.modifiers.contains(KeyModifiers::SHIFT) {
                Some(b"\x1b[6;2~".to_vec()) // Shift+PageDown
            } else if key.modifiers.contains(KeyModifiers::CONTROL) {
                Some(b"\x1b[6;5~".to_vec()) // Ctrl+PageDown
            } else {
                Some(b"\x1b[6~".to_vec())
            }
        }
        KeyCode::Esc => Some(b"\x1b".to_vec()),
        KeyCode::F(n) => {
            // Function keys F1-F12
            match n {
                1 => Some(b"\x1bOP".to_vec()),    // F1
                2 => Some(b"\x1bOQ".to_vec()),    // F2
                3 => Some(b"\x1bOR".to_vec()),    // F3
                4 => Some(b"\x1bOS".to_vec()),    // F4
                5 => Some(b"\x1b[15~".to_vec()),  // F5
                6 => Some(b"\x1b[17~".to_vec()),  // F6
                7 => Some(b"\x1b[18~".to_vec()),  // F7
                8 => Some(b"\x1b[19~".to_vec()),  // F8
                9 => Some(b"\x1b[20~".to_vec()),  // F9
                10 => Some(b"\x1b[21~".to_vec()), // F10
                11 => Some(b"\x1b[23~".to_vec()), // F11
                12 => Some(b"\x1b[24~".to_vec()), // F12
                _ => None,
            }
        }
        KeyCode::Char(c) => {
            if key.modifiers.contains(KeyModifiers::CONTROL) {
                // Handle Ctrl+key combinations
                match c {
                    'a'..='z' => {
                        let ctrl_char = (c as u8) - b'a' + 1;
                        Some(vec![ctrl_char])
                    }
                    'A'..='Z' => {
                        // Ctrl+Shift+letter
                        let ctrl_char = (c.to_ascii_lowercase() as u8) - b'a' + 1;
                        Some(vec![ctrl_char])
                    }
                    '[' | '\\' | ']' | '^' | '_' => {
                        // Special control characters
                        let ctrl_char = c as u8 & 0x1f;
                        Some(vec![ctrl_char])
                    }
                    _ => None,
                }
            } else if key.modifiers.contains(KeyModifiers::ALT) {
                // Alt+key sends ESC followed by the key
                let mut bytes = vec![0x1b]; // ESC
                let _ = write!(&mut bytes, "{}", c);
                Some(bytes)
            } else {
                // Regular character (including with Shift)
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
            Span::styled(
                "🆔 Session ID: ",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(&session_info.id[..8]),
        ]),
        Line::from(vec![
            Span::styled(
                "🌐 Web Interface: ",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                &session_info.url,
                Style::default()
                    .fg(Color::Blue)
                    .add_modifier(Modifier::UNDERLINED),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                "📁 Working Directory: ",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(&session_info.working_dir),
        ]),
        Line::from(vec![
            Span::styled(
                "🔧 Agent: ",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                &agent_upper,
                Style::default()
                    .fg(Color::Magenta)
                    .add_modifier(Modifier::BOLD),
            ),
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
        Span::styled(
            "💬 Interactive",
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        )
    } else {
        Span::styled(
            "👁️  Monitoring",
            Style::default()
                .fg(Color::Blue)
                .add_modifier(Modifier::BOLD),
        )
    };

    let status_lines = vec![
        Line::from(vec![
            Span::styled(
                "Status: ",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "🟢 Running",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                "Mode: ",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            mode_status,
        ]),
        Line::from(vec![
            Span::styled(
                "Uptime: ",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(uptime_str),
        ]),
    ];

    let status_paragraph = Paragraph::new(status_lines).block(status_block);

    f.render_widget(status_paragraph, area);
}

fn draw_system_logs(f: &mut Frame, area: Rect, logs: &[LogEntry]) {
    let logs_block = Block::default()
        .title("📋 System Logs")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Blue));

    if logs.is_empty() {
        let no_logs = Paragraph::new("No system logs")
            .style(Style::default().fg(Color::Gray))
            .block(logs_block)
            .alignment(Alignment::Center);
        f.render_widget(no_logs, area);
    } else {
        let log_lines: Vec<Line> = logs
            .iter()
            .map(|log| {
                let timestamp = log.timestamp.format("%H:%M:%S").to_string();
                let level_color = match log.level {
                    LogLevel::Error => Color::Red,
                    LogLevel::Warn => Color::Yellow,
                    LogLevel::Info => Color::Cyan,
                    LogLevel::Debug => Color::Gray,
                    LogLevel::Trace => Color::DarkGray,
                };

                Line::from(vec![
                    Span::styled(
                        format!("[{}] ", timestamp),
                        Style::default().fg(Color::Gray),
                    ),
                    Span::styled(
                        format!("{:<5} ", log.level.as_str()),
                        Style::default()
                            .fg(level_color)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(&log.message, Style::default().fg(Color::White)),
                ])
            })
            .collect();

        let logs_paragraph = Paragraph::new(log_lines)
            .block(logs_block)
            .wrap(Wrap { trim: true })
            .scroll((logs.len().saturating_sub(3) as u16, 0)); // Auto-scroll to show latest logs

        f.render_widget(logs_paragraph, area);
    }
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
            Span::styled(
                "Tip: ",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
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
        self.cleanup();
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
