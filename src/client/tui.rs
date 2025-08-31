use crate::core::pty_session::GridCell as PtyGridCell;
use crate::core::pty_session::{
    ConnectionStatus as PtyConnectionStatus, GridUpdateMessage, PtyChannels, PtyControlMessage, 
    PtyInput, PtyInputMessage, ScrollDirection, TerminalColor,
};
use crate::utils::tui_writer::{LogEntry, LogLevel};
use anyhow::Result;
use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, EventStream, KeyCode, KeyEventKind,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use futures_util::StreamExt;
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame, Terminal,
};

// UI Layout constants
const STATUS_BAR_HEIGHT: u16 = 1;
use serde::{Deserialize, Serialize};
use std::io;
use tokio::time::{Duration, Instant};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct GridCell {
    pub char: char,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fg_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bg_color: Option<String>,
    #[serde(skip_serializing_if = "is_false")]
    pub bold: bool,
    #[serde(skip_serializing_if = "is_false")]
    pub italic: bool,
    #[serde(skip_serializing_if = "is_false")]
    pub underline: bool,
    #[serde(skip_serializing_if = "is_false")]
    pub reverse: bool,
}

impl GridCell {
    /// Check if this cell is just an empty space with no styling
    pub fn is_empty_space(&self) -> bool {
        self.char == ' '
            && self.fg_color.is_none()
            && self.bg_color.is_none()
            && !self.bold
            && !self.italic
            && !self.underline
            && !self.reverse
    }
}

// Convert PTY GridCell to TUI GridCell
impl From<PtyGridCell> for GridCell {
    fn from(pty_cell: PtyGridCell) -> Self {
        GridCell {
            char: pty_cell.char.chars().next().unwrap_or(' '),
            fg_color: pty_cell.fg_color.map(|c| terminal_color_to_string(&c)),
            bg_color: pty_cell.bg_color.map(|c| terminal_color_to_string(&c)),
            bold: pty_cell.bold,
            italic: pty_cell.italic,
            underline: pty_cell.underline,
            reverse: pty_cell.reverse,
        }
    }
}

// Helper function to convert TerminalColor to String
fn terminal_color_to_string(color: &TerminalColor) -> String {
    match color {
        TerminalColor::Default => "default".to_string(),
        TerminalColor::Indexed(idx) => match *idx {
            0 => "black".to_string(),
            1 => "red".to_string(),
            2 => "green".to_string(),
            3 => "yellow".to_string(),
            4 => "blue".to_string(),
            5 => "magenta".to_string(),
            6 => "cyan".to_string(),
            7 => "white".to_string(),
            8 => "darkgray".to_string(),
            9 => "lightred".to_string(),
            10 => "lightgreen".to_string(),
            11 => "lightyellow".to_string(),
            12 => "lightblue".to_string(),
            13 => "lightmagenta".to_string(),
            14 => "lightcyan".to_string(),
            15 => "gray".to_string(),
            _ => format!("indexed-{}", idx),
        },
        TerminalColor::Palette(idx) => format!("palette-{}", idx),
        TerminalColor::Rgb { r, g, b } => format!("#{:02x}{:02x}{:02x}", r, g, b),
    }
}

// Helper function for serde skip_serializing_if
fn is_false(b: &bool) -> bool {
    !b
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
    system_logs: Vec<LogEntry>,
    // Terminal state from PTY session grid updates
    terminal_grid: std::collections::HashMap<(u16, u16), GridCell>,
    terminal_cursor: (u16, u16),
    terminal_cursor_visible: bool,
    // New channel-based PTY communication (optional until WebSocket connects)
    pty_channels: Option<PtyChannels>,
    // Keyframe state tracking
    has_received_keyframe: bool,
    // Incremental rendering state
    needs_redraw: bool,
    dirty_cells: std::collections::HashSet<(u16, u16)>,
    cursor_dirty: bool,
    last_render_time: std::time::Instant,
    // Session ID for generating URLs
    session_id: String,
    // Connection state tracking
    connection_status: PtyConnectionStatus,
    last_connection_attempt: Option<Instant>,
}

pub struct SessionInfo {
    pub id: String,
    pub agent: String,
    pub _port: u16,
    pub working_dir: String,
    pub url: String,
}

impl SessionTui {
    pub fn new(session_id: String) -> Result<Self> {
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
            system_logs: Vec::new(),
            terminal_grid: std::collections::HashMap::new(),
            terminal_cursor: (0, 0),
            terminal_cursor_visible: true, // Default to visible
            pty_channels: None,            // Will be set when WebSocket connects
            has_received_keyframe: Default::default(), // false
            needs_redraw: true,
            dirty_cells: std::collections::HashSet::new(),
            cursor_dirty: false,
            last_render_time: std::time::Instant::now(),
            session_id,
            connection_status: PtyConnectionStatus::Disconnected,
            last_connection_attempt: None,
        })
    }

    pub fn set_pty_channels(&mut self, pty_channels: PtyChannels) {
        self.pty_channels = Some(pty_channels);
    }

    fn get_web_url(&self) -> String {
        format!("http://localhost:8765/session/{}", self.session_id)
    }

    /// Create terminal area with standard calculation (single source of truth)
    fn create_terminal_area(width: u16, height: u16) -> Rect {
        Rect {
            x: 0,
            y: 0,
            width,
            height: height.saturating_sub(STATUS_BAR_HEIGHT),
        }
    }

    /// Get terminal area for PTY sizing from current terminal
    fn get_pty_terminal_area(&self) -> Result<Rect> {
        let terminal_size = self.terminal.size()?;
        Ok(Self::create_terminal_area(
            terminal_size.width,
            terminal_size.height,
        ))
    }

    pub async fn connect_websocket(&mut self) -> Result<()> {
        use crate::client::http::CodeMuxClient;

        self.last_connection_attempt = Some(Instant::now());
        
        tracing::info!("Connecting to WebSocket for session {}", self.session_id);

        // Create client and connect to WebSocket (this now includes auto-reconnection)
        let client = CodeMuxClient::new("http://localhost:8765".to_string());
        let session_connection = client.connect_to_session(&self.session_id).await?;

        // Convert SessionConnection to PtyChannels
        let pty_channels = session_connection.into_pty_channels();

        // Store the channels
        self.set_pty_channels(pty_channels);
        // Connection status will be updated via the connection_status channel

        tracing::info!("WebSocket connected for session {}", self.session_id);
        Ok(())
    }

    pub fn disconnect_websocket(&mut self) {
        // Dropping pty_channels will close the WebSocket connection
        self.pty_channels = None;
        self.has_received_keyframe = false; // Reset keyframe state
        self.connection_status = PtyConnectionStatus::Disconnected;
        self.status_message =
            "WebSocket disconnected - Press Ctrl+T for interactive mode".to_string();
    }

    fn get_pty_channels(&self) -> Result<&PtyChannels> {
        self.pty_channels.as_ref().ok_or_else(|| {
            anyhow::anyhow!("PTY channels not available - WebSocket not connected yet")
        })
    }

    /// Centralized handler for GridUpdateMessage with keyframe state tracking
    fn handle_grid_update(&mut self, update: GridUpdateMessage) -> bool {
        match update {
            GridUpdateMessage::Keyframe {
                size,
                cells,
                cursor,
                cursor_visible,
                ..
            } => {
                tracing::debug!(
                    "Processing keyframe: {} cells, size {}x{}, cursor ({}, {}), first_keyframe: {}",
                    cells.len(), size.cols, size.rows, cursor.0, cursor.1, !self.has_received_keyframe
                );

                // Update terminal state from keyframe and mark for full redraw
                self.terminal_grid = cells
                    .into_iter()
                    .map(|((row, col), pty_cell)| ((row, col), GridCell::from(pty_cell)))
                    .collect();
                self.terminal_cursor = cursor;
                self.terminal_cursor_visible = cursor_visible;
                self.mark_full_redraw();

                // Mark that we've received our first keyframe
                if !self.has_received_keyframe {
                    self.has_received_keyframe = true;
                    tracing::info!("First keyframe received - terminal state initialized");
                }

                true // Keyframe processed
            }
            GridUpdateMessage::Diff {
                changes, cursor, ..
            } => {
                // Drop diff messages if we haven't received initial keyframe
                if !self.has_received_keyframe {
                    tracing::debug!("Dropping diff update - no initial keyframe received yet");
                    // return false;
                }

                tracing::debug!("Processing diff: {} changes", changes.len());

                // Collect dirty cell positions for incremental rendering
                let dirty_positions: Vec<(u16, u16)> =
                    changes.iter().map(|(row, col, _)| (*row, *col)).collect();

                // Apply changes to terminal grid
                for (row, col, cell) in changes {
                    self.terminal_grid.insert((row, col), GridCell::from(cell));
                }

                // Mark changed cells as dirty for incremental rendering
                self.mark_cells_dirty(&dirty_positions);

                // Update cursor if specified
                if let Some(new_cursor) = cursor {
                    self.mark_cursor_dirty(self.terminal_cursor, new_cursor);
                    self.terminal_cursor = new_cursor;
                }

                true // Diff processed
            }
        }
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
        let channels = match self.get_pty_channels() {
            Ok(channels) => channels,
            Err(_) => {
                // PTY not connected yet, skip resize
                tracing::debug!("PTY not connected yet, skipping resize");
                return;
            }
        };
        let resize_msg = PtyControlMessage::Resize {
            rows: terminal_area.height,
            cols: terminal_area.width,
        };

        if let Err(e) = channels.control_tx.send(resize_msg) {
            tracing::warn!("Failed to send PTY resize command: {}", e);
        } else {
            tracing::debug!(
                "Sent PTY resize command to {}x{}",
                terminal_area.width,
                terminal_area.height
            );
        }
    }

    async fn send_input_to_pty(&self, key: &crossterm::event::KeyEvent) {
        tracing::trace!("send_input_to_pty called with key: {:?}", key);

        let channels = match self.get_pty_channels() {
            Ok(channels) => channels,
            Err(_) => {
                tracing::warn!("PTY not connected yet, ignoring input");
                return;
            }
        };
        // Convert crossterm KeyEvent to our KeyEvent format
        let key_event = crate::core::pty_session::KeyEvent {
            code: convert_key_code(key.code),
            modifiers: crate::core::pty_session::KeyModifiers {
                shift: key
                    .modifiers
                    .contains(crossterm::event::KeyModifiers::SHIFT),
                ctrl: key
                    .modifiers
                    .contains(crossterm::event::KeyModifiers::CONTROL),
                alt: key.modifiers.contains(crossterm::event::KeyModifiers::ALT),
                meta: key
                    .modifiers
                    .contains(crossterm::event::KeyModifiers::SUPER),
            },
        };

        let input_msg = PtyInputMessage {
            input: PtyInput::Key {
                event: key_event,
                client_id: "tui".to_string(),
            },
        };

        if let Err(e) = channels.input_tx.send(input_msg) {
            tracing::warn!("Failed to send input to PTY: {}", e);
        } else {
            // For debugging: if this is Enter, also log that we sent a line terminator
            if matches!(key.code, crossterm::event::KeyCode::Enter) {
                tracing::debug!("SENT ENTER - line should be processed now");
            }
        }
    }

    async fn send_scroll_to_pty(&self, direction: ScrollDirection, lines: u16) {
        tracing::debug!(
            "send_scroll_to_pty called with direction: {:?}, lines: {}",
            direction,
            lines
        );

        let channels = match self.get_pty_channels() {
            Ok(channels) => channels,
            Err(_) => {
                tracing::debug!("PTY not connected yet, ignoring scroll");
                return;
            }
        };

        let input_msg = PtyInputMessage {
            input: PtyInput::Scroll {
                direction,
                lines,
                client_id: "tui".to_string(),
            },
        };

        if let Err(e) = channels.input_tx.send(input_msg) {
            tracing::warn!("Failed to send scroll to PTY: {}", e);
        }
    }

    pub async fn run(
        &mut self,
        session_info: SessionInfo,
        mut log_rx: tokio::sync::mpsc::UnboundedReceiver<LogEntry>,
    ) -> Result<()> {
        self.interactive_mode = false;
        self.status_message = "Ready - Press Ctrl+T for interactive mode".to_string();

        loop {
            let should_quit = if self.interactive_mode {
                self.run_interactive_mode(&session_info, &mut log_rx).await
            } else {
                self.run_monitoring_mode(&session_info, &mut log_rx).await
            };

            match should_quit {
                Ok(true) => {
                    tracing::info!("User requested quit, breaking loop");
                    break; // User wants to quit
                }
                Ok(false) => {
                    tracing::debug!("Mode switch detected, yielding to prevent infinite loop");
                    // Just yield to let other tasks run, avoid problematic sleep
                    tokio::task::yield_now().await;
                    continue; // Mode switch, continue loop
                }
                Err(e) => {
                    tracing::error!("Error occurred: {:?}", e);
                    // Ensure cleanup happens on error
                    self.cleanup();
                    return Err(e);
                }
            }
        }

        tracing::info!("Exiting TUI, performing cleanup");
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

    async fn run_monitoring_mode(
        &mut self,
        session_info: &SessionInfo,
        log_rx: &mut tokio::sync::mpsc::UnboundedReceiver<crate::utils::tui_writer::LogEntry>,
    ) -> Result<bool> {
        tracing::info!("=== ENTERING MONITORING MODE ===");

        let mut display_interval = tokio::time::interval(Duration::from_secs(10));
        let mut event_stream = EventStream::new();

        // Initial render
        let uptime = self.start_time.elapsed();
        match self.draw(session_info, uptime) {
            Ok(_) => tracing::debug!("MONITORING: Initial draw succeeded"),
            Err(e) => {
                tracing::error!("MONITORING: Initial draw FAILED - returning error: {}", e);
                return Err(e);
            }
        }

        self.clear_dirty_state();

        tracing::debug!("MONITORING: Starting main event loop");
        loop {
            // tracing::trace!("MONITORING: iterate");
            tokio::select! {
                biased; // Ensure keyboard events get priority over display updates
                // Handle keyboard events from async stream (prioritize user input)
                maybe_event = event_stream.next() => {
                    match maybe_event {
                        Some(Ok(Event::Key(key))) => {
                            if key.kind == KeyEventKind::Press {
                                tracing::debug!("MONITORING: Key pressed: {:?} modifiers: {:?}", key.code, key.modifiers);

                                // Handle quit
                                if key.code == KeyCode::Char('c') && key.modifiers.contains(event::KeyModifiers::CONTROL) {
                                    tracing::info!("MONITORING: Exiting due to Ctrl+C");
                                    return Ok(true); // Signal to quit
                                }

                                // Handle toggle to interactive mode
                                if key.code == KeyCode::Char('t') && key.modifiers.contains(event::KeyModifiers::CONTROL) {
                                    tracing::info!("SWITCHING TO INTERACTIVE MODE");

                                    self.interactive_mode = true;
                                    self.status_message = "Interactive mode ON - Direct PTY input (Ctrl+T to toggle off)".to_string();

                                    // Get terminal area for PTY sizing
                                    // Don't resize PTY in monitoring mode - only in interactive mode

                                    // Re-render and exit to switch modes
                                    let uptime = self.start_time.elapsed();
                                    self.draw(session_info, uptime)?;
                                    tracing::info!("MONITORING: Exiting to switch to interactive mode (Ctrl+T)");
                                    return Ok(false); // Switch modes
                                }

                                // Handle other monitoring mode keys
                                match key.code {
                                    KeyCode::Char('i') => {
                                        // Switch to interactive mode
                                        self.interactive_mode = true;
                                        self.status_message = "Switching to interactive mode...".to_string();

                                        // Get proper terminal dimensions for interactive mode
                                        // Don't resize PTY in monitoring mode - only in interactive mode

                                        let uptime = self.start_time.elapsed();
                                        self.draw(session_info, uptime)?;
                                        tracing::info!("MONITORING: Exiting to switch to interactive mode (i key)");
                                        return Ok(false); // Switch modes
                                    }
                                    KeyCode::Char('o') => {
                                        // Open web interface
                                        self.status_message = "Opening web interface...".to_string();
                                        if let Err(e) = open::that(self.get_web_url()) {
                                            self.status_message = format!("Failed to open browser: {}", e);
                                        } else {
                                            self.status_message = "Web interface opened".to_string();
                                        }
                                        let uptime = self.start_time.elapsed();
                                        self.draw(session_info, uptime)?;
                                    }
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
                            tracing::debug!("Terminal resized to {}x{}", width, height);
                            // Terminal was resized, update display
                            let uptime = self.start_time.elapsed();
                            self.draw(session_info, uptime)?;
                        }
                        Some(Ok(_)) => {
                            // Other events (mouse, etc.) - ignore
                        }
                        Some(Err(e)) => {
                            tracing::warn!("Event stream error: {:?}", e);
                            // Continue trying to read events
                        }
                        None => {
                            tracing::info!("Event stream terminated");
                            return Ok(true); // Exit if event stream ends
                        }
                    }
                }

                // Handle log entries
                log_entry = log_rx.recv() => {
                    if let Some(entry) = log_entry {
                        self.system_logs.push(entry);
                        // Keep only recent logs
                        if self.system_logs.len() > 50 {
                            self.system_logs.drain(0..(self.system_logs.len() - 50));
                        }
                    }
                }

                // Update display every second (lower priority)
                _ = display_interval.tick() => {
                    let uptime = self.start_time.elapsed();
                    match self.draw(session_info, uptime) {
                        Ok(_) => {
                            // Log less frequently - every 30 seconds
                            if uptime.as_secs() % 30 == 0 {
                                tracing::trace!("Display update - uptime: {}s", uptime.as_secs());
                            }
                        }
                        Err(e) => {
                            tracing::error!("Draw failed in monitoring mode: {}", e);
                            return Err(e);
                        }
                    }
                }
            }
        }
    }

    async fn run_interactive_mode(
        &mut self,
        session_info: &SessionInfo,
        log_rx: &mut tokio::sync::mpsc::UnboundedReceiver<crate::utils::tui_writer::LogEntry>,
    ) -> Result<bool> {
        tracing::debug!("=== ENTERING INTERACTIVE MODE ===");

        // Connect WebSocket if not already connected
        if self.pty_channels.is_none() {
            self.status_message = "Connecting to session via WebSocket...".to_string();
            match self.connect_websocket().await {
                Ok(()) => {
                    tracing::info!("WebSocket connected successfully");
                    self.status_message = "Connected - Interactive mode active".to_string();

                    // Send initial resize to match current terminal size
                    let terminal_area = self.get_pty_terminal_area()?;
                    self.resize_pty_to_match_tui(terminal_area).await;
                }
                Err(e) => {
                    tracing::error!("Failed to connect WebSocket: {}", e);
                    self.status_message = format!("Connection failed: {}", e);
                    return Ok(false);
                }
            }
        }

        // Clone grid_tx and connection_status_tx for receiving updates - server will automatically send keyframe
        let (grid_tx, connection_status_tx) = {
            let channels = match self.get_pty_channels() {
                Ok(channels) => channels,
                Err(e) => {
                    tracing::error!("Cannot enter interactive mode - PTY not connected: {}", e);
                    return Ok(false);
                }
            };

            (channels.grid_tx.clone(), channels.connection_status_tx.clone())
        };

        tracing::debug!("Keyframe handling complete, setting up interactive mode");
        let mut event_stream = EventStream::new();
        let mut grid_update_stream = grid_tx.subscribe();
        let mut connection_status_stream = connection_status_tx.subscribe();

        // Add a periodic timer to keep the display updated
        use tokio::time::interval;
        let mut display_interval = interval(Duration::from_secs(10));

        // Add a rate limiter for PTY processing to prevent starvation
        let mut pty_throttle = interval(Duration::from_millis(16));

        // Initial render after keyframe
        let uptime = self.start_time.elapsed();
        tracing::debug!("Performing initial draw after keyframe");
        self.draw(session_info, uptime)?;
        self.clear_dirty_state();
        tracing::debug!("Initial draw complete, dirty state cleared");

        // Debug the initial terminal state
        let terminal_size = self.terminal.size()?;
        tracing::debug!(
            "Starting interactive mode loop - Terminal: {}x{}",
            terminal_size.width,
            terminal_size.height
        );

        tracing::debug!("About to enter interactive mode main loop");
        tracing::debug!("Starting interactive mode loop");

        loop {
            tokio::select! {
                biased; // Process branches in order, ensuring timer gets a chance

                // Handle log entries
                log_entry = log_rx.recv() => {
                    if let Some(entry) = log_entry {
                        self.system_logs.push(entry);
                        // Keep only recent logs
                        if self.system_logs.len() > 50 {
                            self.system_logs.drain(0..(self.system_logs.len() - 50));
                        }
                    }
                }

                // Periodic display update (also serves as heartbeat)
                _ = display_interval.tick() => {
                    let uptime = self.start_time.elapsed();
                    tracing::trace!("Interactive mode heartbeat - uptime: {}s", uptime.as_secs());
                    self.draw(session_info, uptime)?;
                }
                
                // Handle connection status updates
                Ok(status) = connection_status_stream.recv() => {
                    tracing::debug!("Connection status updated: {:?}", status);
                    self.connection_status = status;
                    self.needs_redraw = true;
                }

                // Handle keyboard events from async stream (prioritize user input)
                maybe_event = event_stream.next() => {
                    match maybe_event {
                        Some(Ok(Event::Key(key))) => {
                            if key.kind == KeyEventKind::Press {
                                tracing::debug!("INTERACTIVE MODE - Key: {:?} modifiers: {:?}", key.code, key.modifiers);

                                // Handle quit
                                if key.code == KeyCode::Char('c') && key.modifiers.contains(event::KeyModifiers::CONTROL) {
                                    return Ok(true); // Signal to quit
                                }

                                // Handle toggle back to monitoring mode
                                if key.code == KeyCode::Char('t') && key.modifiers.contains(event::KeyModifiers::CONTROL) {
                                    tracing::info!("SWITCHING TO MONITORING MODE");

                                    self.interactive_mode = false;
                                    self.disconnect_websocket();
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
                        Some(Ok(Event::Mouse(mouse))) => {
                            match mouse {
                                crossterm::event::MouseEvent {
                                    kind: crossterm::event::MouseEventKind::ScrollUp,
                                    ..
                                } => {
                                    self.send_scroll_to_pty(ScrollDirection::Up, 1).await;
                                }
                                crossterm::event::MouseEvent {
                                    kind: crossterm::event::MouseEventKind::ScrollDown,
                                    ..
                                } => {
                                    self.send_scroll_to_pty(ScrollDirection::Down, 1).await;
                                }
                                _ => {
                                    // Ignore other mouse events
                                }
                            }
                        }
                        Some(Ok(Event::Resize(width, height))) => {
                            tracing::debug!("Terminal resized to {}x{} in interactive mode", width, height);

                            // Update terminal size tracking
                            let terminal_area = Self::create_terminal_area(width, height);
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
                            tracing::warn!("Event stream error: {:?}", e);
                            // Continue trying to read events
                        }
                        None => {
                            tracing::info!("Event stream terminated");
                            return Ok(true); // Exit if event stream ends
                        }
                    }
                }

                // Handle grid updates from PTY session (throttled to prevent starvation)
                _ = pty_throttle.tick() => {
                    // Try to drain multiple grid updates at once, but limited per cycle
                    let mut updates_processed = 0;
                    let max_updates_per_cycle = 10; // Reduced to ensure fairness

                    {
                        while updates_processed < max_updates_per_cycle {
                            match grid_update_stream.try_recv() {
                                Ok(update) => {
                                    // Process grid update using centralized handler
                                    if self.handle_grid_update(update) {
                                        updates_processed += 1;
                                    }
                                    // If handle_grid_update returns false, update was dropped (e.g., diff before keyframe)
                                }
                                Err(tokio::sync::broadcast::error::TryRecvError::Empty) => break, // No more data available
                                Err(tokio::sync::broadcast::error::TryRecvError::Lagged(_)) => {
                                    tracing::warn!("Grid update stream lagged, some messages may have been missed");
                                    continue; // Try to get the next message
                                }
                                Err(tokio::sync::broadcast::error::TryRecvError::Closed) => {
                                    tracing::info!("Grid update stream closed");
                                    break;
                                }
                            }
                        }
                    }

                    // Only redraw if we have changes and enough time has passed (batching)
                    if updates_processed > 0 && self.should_redraw_now() {
                        if self.dirty_cells.is_empty() && self.needs_redraw {
                            tracing::debug!("Processed {} grid updates, performing full redraw", updates_processed);
                        } else {
                            tracing::debug!("Processed {} grid updates, redrawing {} dirty cells",
                                updates_processed, self.dirty_cells.len());
                        }

                        let uptime = self.start_time.elapsed();
                        self.draw(session_info, uptime)?;
                        self.clear_dirty_state();
                    } else if updates_processed > 0 {
                        tracing::debug!("Processed {} grid updates, batching (dirty cells: {}, time since last: {}ms)",
                            updates_processed, self.dirty_cells.len(), self.last_render_time.elapsed().as_millis());
                    }
                }
            }
        }
    }

    fn draw(&mut self, session_info: &SessionInfo, uptime: Duration) -> Result<()> {
        // Pre-compute terminal size and update tracking if in interactive mode
        let terminal_size = self.terminal.size()?;

        // Extract needed data before the draw closure to avoid borrowing issues
        let interactive_mode = self.interactive_mode;
        let terminal_grid = self.terminal_grid.clone();
        let terminal_cursor = self.terminal_cursor;
        let cursor_visible = self.terminal_cursor_visible;
        let _terminal_grid_size = (
            terminal_size.height.saturating_sub(STATUS_BAR_HEIGHT),
            terminal_size.width,
        );
        let system_logs = self.system_logs.clone();
        let connection_status = self.connection_status.clone();

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

                // Debug: Log grid info before rendering
                if terminal_grid.is_empty() {
                    tracing::warn!("terminal_grid is empty during draw!");
                } else {
                    // Count non-empty cells for debugging
                    let non_empty = terminal_grid.values()
                        .filter(|cell| cell.char != ' ')
                        .count();
                    if non_empty == 0 {
                        tracing::warn!("All {} grid cells are empty/whitespace during draw!", terminal_grid.len());
                    } else {
                        tracing::trace!("Drawing {} cells, {} non-empty", terminal_grid.len(), non_empty);
                    }
                }

                // Create terminal content from grid state - calculate dimensions from grid
                let grid_dimensions = calculate_grid_dimensions(&terminal_grid);
                let terminal_content = render_terminal_from_grid(&terminal_grid, grid_dimensions, terminal_cursor, cursor_visible, terminal_area.height, terminal_area.width);
                let terminal_widget = Paragraph::new(terminal_content)
                    .block(Block::default().borders(Borders::NONE));
                    // No wrapping - each line should be rendered exactly as provided
                f.render_widget(terminal_widget, terminal_area);
                
                // Draw disconnection overlay if not connected
                // Use the full screen size for proper centering
                if !matches!(connection_status, PtyConnectionStatus::Connected) {
                    draw_connection_overlay(f, f.area(), &connection_status);
                }

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
                draw_status(f, content_chunks[1], uptime, interactive_mode, &connection_status);
                // System logs section
                draw_system_logs(f, content_chunks[2], &system_logs);
                // Instructions
                draw_instructions(f, content_chunks[3]);

                // Footer
                let footer = Paragraph::new("Ctrl+C: Stop | i: Interactive Mode | o: Open Web | r: Refresh | Ctrl+T: Interactive Mode")
                    .style(Style::default().fg(Color::Gray))
                    .alignment(Alignment::Center)
                    .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::Gray)));
                f.render_widget(footer, chunks[2]);
                
                // Draw disconnection overlay if not connected (for monitoring mode too)
                if !matches!(connection_status, PtyConnectionStatus::Connected) {
                    draw_connection_overlay(f, f.area(), &connection_status);
                }
            }
        })?;

        Ok(())
    }

    // No longer needed - moved to standalone function below
}

/// Calculate actual grid dimensions from the grid data
fn calculate_grid_dimensions(
    terminal_grid: &std::collections::HashMap<(u16, u16), GridCell>,
) -> (u16, u16) {
    if terminal_grid.is_empty() {
        return (0, 0);
    }

    let max_row = terminal_grid.keys().map(|(row, _)| *row).max().unwrap_or(0);
    let max_col = terminal_grid.keys().map(|(_, col)| *col).max().unwrap_or(0);

    // Add 1 because grid uses 0-based indexing
    (max_row + 1, max_col + 1)
}

/// Render terminal content from grid state for display
fn render_terminal_from_grid(
    terminal_grid: &std::collections::HashMap<(u16, u16), GridCell>,
    terminal_size: (u16, u16),
    cursor_pos: (u16, u16),
    cursor_visible: bool,
    display_height: u16,
    display_width: u16,
) -> Vec<ratatui::text::Line> {
    let (grid_rows, grid_cols) = terminal_size;
    let mut lines = Vec::new();

    let actual_rows = std::cmp::min(grid_rows, display_height);

    // Render each row of the terminal - use server PTY size but trim to local display
    for row in 0..actual_rows {
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
                        .and_then(|c| string_color_to_ratatui(c))
                        .unwrap_or(Color::Reset))
                    .bg(cell
                        .bg_color
                        .as_ref()
                        .and_then(|c| string_color_to_ratatui(c))
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
                    })
                    .add_modifier(if cell.reverse {
                        Modifier::REVERSED
                    } else {
                        Modifier::empty()
                    });

                // Highlight cursor position with reversed colors (only if cursor is visible)
                if is_cursor && cursor_visible {
                    cell_style = cell_style.add_modifier(Modifier::REVERSED);
                }

                // If style changed, flush current span and start new one
                if cell_style != current_style && !current_line.is_empty() {
                    line_spans.push(Span::styled(current_line.clone(), current_style));
                    current_line.clear();
                }

                // Filter out newlines and other control characters that shouldn't be rendered
                let char_to_render = if cell.char == '\n' || cell.char == '\r' {
                    ' '
                } else {
                    cell.char
                };
                current_line.push(char_to_render);
                current_style = cell_style;
            } else {
                // Empty cell - use space, but highlight if cursor is here and visible
                let mut empty_style = Style::default();
                if is_cursor && cursor_visible {
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

    // Don't add empty lines - let the Paragraph widget handle the remaining space
    // Only ensure we have at least one line to avoid empty widget
    if lines.is_empty() {
        lines.push(Line::from(" "));
    }

    lines
}

/// Convert color string to ratatui Color
fn string_color_to_ratatui(color_str: &str) -> Option<Color> {
    if color_str.starts_with('#') && color_str.len() == 7 {
        // Parse hex color like #ff0000
        if let (Ok(r), Ok(g), Ok(b)) = (
            u8::from_str_radix(&color_str[1..3], 16),
            u8::from_str_radix(&color_str[3..5], 16),
            u8::from_str_radix(&color_str[5..7], 16),
        ) {
            return Some(Color::Rgb(r, g, b));
        }
    }

    // Try parsing named colors
    match color_str.to_lowercase().as_str() {
        "black" => Some(Color::Black),
        "red" => Some(Color::Red),
        "green" => Some(Color::Green),
        "yellow" => Some(Color::Yellow),
        "blue" => Some(Color::Blue),
        "magenta" => Some(Color::Magenta),
        "cyan" => Some(Color::Cyan),
        "white" => Some(Color::White),
        "gray" | "grey" => Some(Color::Gray),
        "darkgray" | "darkgrey" => Some(Color::DarkGray),
        "lightred" => Some(Color::LightRed),
        "lightgreen" => Some(Color::LightGreen),
        "lightyellow" => Some(Color::LightYellow),
        "lightblue" => Some(Color::LightBlue),
        "lightmagenta" => Some(Color::LightMagenta),
        "lightcyan" => Some(Color::LightCyan),
        _ => None,
    }
}

/// Convert crossterm KeyCode to our KeyCode
fn convert_key_code(code: crossterm::event::KeyCode) -> crate::core::pty_session::KeyCode {
    use crate::core::pty_session::KeyCode;
    use crossterm::event::KeyCode as CrosstermKeyCode;

    match code {
        CrosstermKeyCode::Backspace => KeyCode::Backspace,
        CrosstermKeyCode::Enter => KeyCode::Enter,
        CrosstermKeyCode::Left => KeyCode::Left,
        CrosstermKeyCode::Right => KeyCode::Right,
        CrosstermKeyCode::Up => KeyCode::Up,
        CrosstermKeyCode::Down => KeyCode::Down,
        CrosstermKeyCode::Home => KeyCode::Home,
        CrosstermKeyCode::End => KeyCode::End,
        CrosstermKeyCode::PageUp => KeyCode::PageUp,
        CrosstermKeyCode::PageDown => KeyCode::PageDown,
        CrosstermKeyCode::Tab => KeyCode::Tab,
        CrosstermKeyCode::BackTab => KeyCode::Tab, // Map BackTab to Tab, modifiers will handle it
        CrosstermKeyCode::Delete => KeyCode::Delete,
        CrosstermKeyCode::Insert => KeyCode::Insert,
        CrosstermKeyCode::F(n) => KeyCode::F(n),
        CrosstermKeyCode::Char(c) => KeyCode::Char(c),
        CrosstermKeyCode::Null => KeyCode::Char('\0'), // Map to null char
        CrosstermKeyCode::Esc => KeyCode::Esc,
        // Unsupported keys - map to reasonable alternatives
        CrosstermKeyCode::CapsLock => KeyCode::Char('\0'),
        CrosstermKeyCode::ScrollLock => KeyCode::Char('\0'),
        CrosstermKeyCode::NumLock => KeyCode::Char('\0'),
        CrosstermKeyCode::PrintScreen => KeyCode::Char('\0'),
        CrosstermKeyCode::Pause => KeyCode::Char('\0'),
        CrosstermKeyCode::Menu => KeyCode::Char('\0'),
        CrosstermKeyCode::KeypadBegin => KeyCode::Char('\0'),
        CrosstermKeyCode::Media(_) => KeyCode::Char('\0'), // Not supported
        CrosstermKeyCode::Modifier(_) => KeyCode::Char('\0'), // Not supported
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

fn draw_status(f: &mut Frame, area: Rect, uptime: Duration, interactive_mode: bool, connection_status: &PtyConnectionStatus) {
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

    let connection_span = match connection_status {
        PtyConnectionStatus::Connected => Span::styled(
            "🟢 Connected",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        ),
        PtyConnectionStatus::Disconnected => Span::styled(
            "🔴 Disconnected",
            Style::default()
                .fg(Color::Red)
                .add_modifier(Modifier::BOLD),
        ),
        PtyConnectionStatus::Reconnecting { attempt, max_attempts } => Span::styled(
            format!("🟡 Reconnecting ({}/{})", attempt, max_attempts),
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
    };

    let mut status_lines = vec![
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
                "Connection: ",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            connection_span,
        ]),
    ];
    
    // Only show uptime if we have space (at least 4 lines in area)
    if area.height >= 6 {
        status_lines.push(Line::from(vec![
            Span::styled(
                "Uptime: ",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(uptime_str),
        ]));
    }

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
        Line::from("• Press 'i' to enter interactive mode and control the agent directly"),
        Line::from("• Press 'o' to open the web interface in your browser"),
        Line::from("• Press 'r' to refresh the display"),
        Line::from("• Press Ctrl+C to stop the session"),
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

fn draw_connection_overlay(f: &mut Frame, area: Rect, connection_status: &PtyConnectionStatus) {
    use ratatui::widgets::Clear;
    
    // Calculate center position for overlay
    let overlay_width = 50;
    let overlay_height = 7;
    
    // Ensure we don't overflow the screen
    let overlay_width = overlay_width.min(area.width);
    let overlay_height = overlay_height.min(area.height);
    
    let overlay_x = area.width.saturating_sub(overlay_width) / 2;
    let overlay_y = area.height.saturating_sub(overlay_height) / 2;
    
    let overlay_area = Rect::new(overlay_x, overlay_y, overlay_width, overlay_height);
    
    // Determine style and content based on connection status
    let (title, message, style) = match connection_status {
        PtyConnectionStatus::Disconnected => (
            " ⚠️  DISCONNECTED ",
            vec![
                Line::from(""),
                Line::from(vec![
                    Span::styled("WebSocket connection lost", Style::default().fg(Color::White))
                ]),
                Line::from(""),
                Line::from(vec![
                    Span::styled("Attempting to reconnect...", Style::default().fg(Color::Gray))
                ]),
            ],
            Style::default().bg(Color::Red).fg(Color::White).add_modifier(Modifier::BOLD)
        ),
        PtyConnectionStatus::Reconnecting { attempt, max_attempts } => (
            " 🔄 RECONNECTING ",
            vec![
                Line::from(""),
                Line::from(vec![
                    Span::styled(
                        format!("Reconnection attempt {} of {}", attempt, max_attempts),
                        Style::default().fg(Color::White)
                    )
                ]),
                Line::from(""),
                Line::from(vec![
                    Span::styled("Please wait...", Style::default().fg(Color::Gray))
                ]),
            ],
            Style::default().bg(Color::Yellow).fg(Color::Black).add_modifier(Modifier::BOLD)
        ),
        PtyConnectionStatus::Connected => {
            // This shouldn't happen as we only show overlay when not connected
            return;
        }
    };
    
    // Create the overlay block with a clear background
    let overlay_block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(style)
        .style(Style::default().bg(Color::Black));
    
    // Create the content paragraph
    let overlay_content = Paragraph::new(message)
        .block(overlay_block)
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::White));
    
    // Clear the area behind the overlay first (optional, for better visibility)
    f.render_widget(Clear, overlay_area);
    
    // Render the overlay
    f.render_widget(overlay_content, overlay_area);
}
