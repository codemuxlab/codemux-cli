use anyhow::Result;
use portable_pty::{CommandBuilder, NativePtySystem, PtySize, PtySystem};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{broadcast, mpsc, Mutex};

/// Messages that can be sent to control the PTY session
#[derive(Debug)]
pub enum PtyControlMessage {
    Resize {
        rows: u16,
        cols: u16,
    },
    Terminate,
    RequestKeyframe {
        response_tx: tokio::sync::oneshot::Sender<GridUpdateMessage>,
    },
}

/// Messages representing PTY input from clients
#[derive(Debug, Clone)]
pub struct PtyInputMessage {
    pub data: Vec<u8>,
    pub client_id: String,
}

/// Messages representing PTY output to clients
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PtyOutputMessage {
    pub data: Vec<u8>,
    pub timestamp: std::time::SystemTime,
}

/// Serializable version of PtySize for grid messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializablePtySize {
    pub rows: u16,
    pub cols: u16,
}

impl From<PtySize> for SerializablePtySize {
    fn from(size: PtySize) -> Self {
        SerializablePtySize {
            rows: size.rows,
            cols: size.cols,
        }
    }
}

/// Terminal grid cell representation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GridCell {
    pub char: String,
    pub fg_color: Option<String>, // hex color like "#ffffff"
    pub bg_color: Option<String>,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
}

/// Terminal grid update messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GridUpdateMessage {
    /// Full terminal state keyframe (sent to new clients)
    Keyframe {
        size: SerializablePtySize,
        cells: HashMap<(u16, u16), GridCell>, // (row, col) -> cell
        cursor: (u16, u16),                   // (row, col)
        timestamp: std::time::SystemTime,
    },
    /// Incremental changes (sent to existing clients)
    Diff {
        changes: Vec<(u16, u16, GridCell)>, // (row, col, new_cell)
        cursor: Option<(u16, u16)>,         // new cursor position if changed
        timestamp: std::time::SystemTime,
    },
}

/// Channel interface for communicating with PTY session
#[derive(Clone)]
pub struct PtyChannels {
    pub input_tx: mpsc::UnboundedSender<PtyInputMessage>,
    pub output_tx: broadcast::Sender<PtyOutputMessage>,
    pub control_tx: mpsc::UnboundedSender<PtyControlMessage>,
    pub size_tx: broadcast::Sender<PtySize>,
    pub grid_tx: broadcast::Sender<GridUpdateMessage>,
}

impl PtyChannels {
    /// Request a keyframe from the PTY session (for new clients)
    pub async fn request_keyframe(
        &self,
    ) -> Result<GridUpdateMessage, Box<dyn std::error::Error + Send + Sync>> {
        let (tx, rx) = tokio::sync::oneshot::channel();

        self.control_tx
            .send(PtyControlMessage::RequestKeyframe { response_tx: tx })
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

        let keyframe = rx
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

        Ok(keyframe)
    }
}

/// Standalone PTY session component that manages subprocess and I/O
pub struct PtySession {
    id: String,
    agent: String,
    args: Vec<String>,

    // Internal PTY management
    pty: Arc<Mutex<Box<dyn portable_pty::MasterPty + Send>>>,
    writer: Arc<Mutex<Box<dyn std::io::Write + Send>>>,
    current_size: Arc<Mutex<PtySize>>,

    // Terminal buffer for new client snapshots (stores recent output)
    buffer: Arc<Mutex<Vec<u8>>>,

    // VT100 terminal state and parser
    vt_parser: Arc<Mutex<vt100::Parser>>,
    grid_state: Arc<Mutex<HashMap<(u16, u16), GridCell>>>,
    cursor_pos: Arc<Mutex<(u16, u16)>>,

    // Debounce timing for keyframe generation
    last_activity: Arc<Mutex<Instant>>,

    // Channel endpoints
    input_rx: mpsc::UnboundedReceiver<PtyInputMessage>,
    output_tx: broadcast::Sender<PtyOutputMessage>,
    control_rx: mpsc::UnboundedReceiver<PtyControlMessage>,
    size_tx: broadcast::Sender<PtySize>,
    grid_tx: broadcast::Sender<GridUpdateMessage>,

    // Task handles
    reader_task: Option<tokio::task::JoinHandle<()>>,
    input_task: Option<tokio::task::JoinHandle<()>>,
    control_task: Option<tokio::task::JoinHandle<()>>,
    debounce_task: Option<tokio::task::JoinHandle<()>>,
}

impl PtySession {
    /// Create a new PTY session with the specified agent and arguments
    pub fn new(id: String, agent: String, args: Vec<String>) -> Result<(Self, PtyChannels)> {
        let pty_system = NativePtySystem::default();

        // Use environment variables for initial PTY size if available
        let initial_cols = std::env::var("COLUMNS")
            .ok()
            .and_then(|s| s.parse::<u16>().ok())
            .unwrap_or(120);
        let initial_rows = std::env::var("LINES")
            .ok()
            .and_then(|s| s.parse::<u16>().ok())
            .unwrap_or(30);

        let pty_pair = pty_system.openpty(PtySize {
            rows: initial_rows,
            cols: initial_cols,
            pixel_width: 0,
            pixel_height: 0,
        })?;

        let mut cmd = CommandBuilder::new(&agent);
        for arg in &args {
            cmd.arg(arg);
        }

        // Set working directory to current directory (project root)
        if let Ok(current_dir) = std::env::current_dir() {
            cmd.cwd(current_dir);
        }

        // Set environment variables for proper terminal behavior
        cmd.env("TERM", "xterm-256color");
        cmd.env("COLORTERM", "truecolor");
        cmd.env("FORCE_COLOR", "1");
        cmd.env("COLUMNS", initial_cols.to_string());
        cmd.env("LINES", initial_rows.to_string());

        // Preserve important environment variables
        for (key, value) in std::env::vars() {
            match key.as_str() {
                "HOME" | "USER" | "PATH" | "SHELL" | "LANG" | "LC_ALL" | "PWD" => {
                    cmd.env(key, value);
                }
                _ => {}
            }
        }

        let _child = pty_pair.slave.spawn_command(cmd)?;

        let _reader = pty_pair.master.try_clone_reader()?;
        let writer = pty_pair.master.take_writer()?;

        // Create channels
        let (input_tx, input_rx) = mpsc::unbounded_channel();
        let (output_tx, _) = broadcast::channel(1000);
        let (control_tx, control_rx) = mpsc::unbounded_channel();
        let (size_tx, _) = broadcast::channel(100);
        let (grid_tx, _) = broadcast::channel(1000);

        // Create client channel interface
        let channels = PtyChannels {
            input_tx,
            output_tx: output_tx.clone(),
            control_tx,
            size_tx: size_tx.clone(),
            grid_tx: grid_tx.clone(),
        };

        let session = PtySession {
            id,
            agent,
            args,
            pty: Arc::new(Mutex::new(pty_pair.master)),
            writer: Arc::new(Mutex::new(writer)),
            current_size: Arc::new(Mutex::new(PtySize {
                rows: initial_rows,
                cols: initial_cols,
                pixel_width: 0,
                pixel_height: 0,
            })),
            buffer: Arc::new(Mutex::new(Vec::new())),
            vt_parser: Arc::new(Mutex::new(vt100::Parser::new(
                initial_rows,
                initial_cols,
                0,
            ))),
            grid_state: Arc::new(Mutex::new(HashMap::new())),
            cursor_pos: Arc::new(Mutex::new((0, 0))),
            last_activity: Arc::new(Mutex::new(Instant::now())),
            input_rx,
            output_tx,
            control_rx,
            size_tx,
            grid_tx,
            reader_task: None,
            input_task: None,
            control_task: None,
            debounce_task: None,
        };

        Ok((session, channels))
    }

    /// Start the PTY session tasks
    pub async fn start(&mut self) -> Result<()> {
        // Start PTY reader task with VT100 parsing
        let reader = Arc::new(Mutex::new(self.pty.lock().await.try_clone_reader()?));
        let output_tx = self.output_tx.clone();
        let buffer_arc = self.buffer.clone();
        let vt_parser = self.vt_parser.clone();
        let grid_state = self.grid_state.clone();
        let cursor_pos = self.cursor_pos.clone();
        let grid_tx = self.grid_tx.clone();
        let current_size = self.current_size.clone();
        let last_activity = self.last_activity.clone();

        self.reader_task = Some(tokio::spawn(async move {
            let mut read_buffer = [0u8; 1024];
            let mut previous_grid: HashMap<(u16, u16), GridCell> = HashMap::new();

            loop {
                let mut reader_guard = reader.lock().await;
                match reader_guard.read(&mut read_buffer) {
                    Ok(0) => {
                        tracing::info!("PTY reader reached EOF");
                        break;
                    }
                    Ok(n) => {
                        let data = read_buffer[..n].to_vec();

                        // Update the terminal buffer (keep last 64KB for new clients)
                        {
                            let mut buffer_guard = buffer_arc.lock().await;
                            buffer_guard.extend_from_slice(&data);

                            // Keep buffer size reasonable (last 64KB of output)
                            if buffer_guard.len() > 65536 {
                                let drain_count = buffer_guard.len() - 65536;
                                buffer_guard.drain(0..drain_count);
                            }
                        }

                        // Process through VT100 parser
                        {
                            let mut parser_guard = vt_parser.lock().await;
                            parser_guard.process(&data);
                        }

                        // Extract grid changes and generate update message
                        let grid_update = Self::extract_grid_changes(
                            &vt_parser,
                            &grid_state,
                            &cursor_pos,
                            &current_size,
                            &mut previous_grid,
                        )
                        .await;

                        if let Some(update) = grid_update {
                            let _ = grid_tx.send(update);
                        }

                        // Update last activity time for debounce timer
                        {
                            let mut activity_guard = last_activity.lock().await;
                            *activity_guard = Instant::now();
                        }

                        // Send raw bytes to subscribers (for backward compatibility)
                        let msg = PtyOutputMessage {
                            data,
                            timestamp: std::time::SystemTime::now(),
                        };
                        if output_tx.send(msg).is_err() {
                            break; // No more receivers
                        }
                    }
                    Err(e) => {
                        tracing::error!("Error reading from PTY: {}", e);
                        break;
                    }
                }
                drop(reader_guard);
                tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            }
        }));

        // Start input handler task
        let writer = self.writer.clone();
        let mut input_rx = std::mem::replace(&mut self.input_rx, {
            let (_tx, rx) = mpsc::unbounded_channel();
            rx
        });

        self.input_task = Some(tokio::spawn(async move {
            while let Some(msg) = input_rx.recv().await {
                let mut writer_guard = writer.lock().await;
                if let Err(e) = writer_guard.write_all(&msg.data) {
                    tracing::error!("Failed to write to PTY: {}", e);
                    break;
                }
                let _ = writer_guard.flush();
            }
        }));

        // Start control handler task
        let pty = self.pty.clone();
        let current_size = self.current_size.clone();
        let size_tx = self.size_tx.clone();
        let vt_parser_for_control = self.vt_parser.clone();
        let cursor_pos_for_control = self.cursor_pos.clone();
        // Note: grid_tx not needed for control task anymore since keyframes go directly to clients
        let mut control_rx = std::mem::replace(&mut self.control_rx, {
            let (_tx, rx) = mpsc::unbounded_channel();
            rx
        });

        self.control_task = Some(tokio::spawn(async move {
            while let Some(msg) = control_rx.recv().await {
                match msg {
                    PtyControlMessage::Resize { rows, cols } => {
                        let new_size = PtySize {
                            rows,
                            cols,
                            pixel_width: 0,
                            pixel_height: 0,
                        };
                        
                        // First, try to resize the PTY
                        let pty_guard = pty.lock().await;
                        match pty_guard.resize(new_size.clone()) {
                            Ok(()) => {
                                drop(pty_guard); // Release PTY lock early
                                
                                // Update tracked size
                                {
                                    let mut size_guard = current_size.lock().await;
                                    *size_guard = new_size.clone();
                                }

                                // CRITICAL: Update VT100 parser size to match PTY
                                {
                                    let mut parser_guard = vt_parser_for_control.lock().await;
                                    parser_guard.set_size(rows, cols);
                                }

                                // Clear cursor position to prevent out-of-bounds issues
                                {
                                    let mut cursor_guard = cursor_pos_for_control.lock().await;
                                    *cursor_guard = (0, 0);
                                }

                                // Broadcast size change to all subscribers
                                let _ = size_tx.send(new_size);
                                tracing::info!("PTY successfully resized to {}x{}", cols, rows);
                            }
                            Err(e) => {
                                drop(pty_guard);
                                tracing::error!("Failed to resize PTY to {}x{}: {}", cols, rows, e);
                                // Don't update internal state if resize failed
                            }
                        }
                    }
                    PtyControlMessage::Terminate => {
                        tracing::info!("PTY session termination requested");
                        break;
                    }
                    PtyControlMessage::RequestKeyframe { response_tx } => {
                        tracing::debug!("Keyframe requested by specific client");
                        let keyframe = Self::generate_keyframe(
                            &vt_parser_for_control,
                            &cursor_pos_for_control,
                            &current_size,
                        )
                        .await;

                        // Send keyframe directly to the requesting client
                        if response_tx.send(keyframe).is_err() {
                            tracing::warn!(
                                "Failed to send keyframe to requesting client (receiver dropped)"
                            );
                        }
                    }
                }
            }
        }));

        // Start debounce timer task for automatic keyframes
        let last_activity_for_debounce = self.last_activity.clone();
        let vt_parser_for_debounce = self.vt_parser.clone();
        let cursor_pos_for_debounce = self.cursor_pos.clone();
        let current_size_for_debounce = self.current_size.clone();
        let grid_tx_for_debounce = self.grid_tx.clone();

        self.debounce_task = Some(tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(500));
            let debounce_duration = tokio::time::Duration::from_secs(3);

            loop {
                interval.tick().await;

                // Check if enough time has passed since last activity
                let should_send_keyframe = {
                    let activity_guard = last_activity_for_debounce.lock().await;
                    activity_guard.elapsed() >= debounce_duration
                };

                if should_send_keyframe {
                    tracing::debug!("Sending debounced keyframe after 3s of inactivity");
                    let keyframe = Self::generate_keyframe(
                        &vt_parser_for_debounce,
                        &cursor_pos_for_debounce,
                        &current_size_for_debounce,
                    )
                    .await;

                    if grid_tx_for_debounce.send(keyframe).is_err() {
                        tracing::debug!("No subscribers for debounced keyframe");
                        break;
                    }

                    // Update last activity to prevent rapid keyframes
                    {
                        let mut activity_guard = last_activity_for_debounce.lock().await;
                        *activity_guard = Instant::now();
                    }
                }
            }
        }));

        Ok(())
    }

    /// Get the current PTY size
    pub async fn get_size(&self) -> PtySize {
        let size_guard = self.current_size.lock().await;
        size_guard.clone()
    }

    /// Get session metadata
    pub fn get_info(&self) -> SessionInfo {
        SessionInfo {
            id: self.id.clone(),
            agent: self.agent.clone(),
            args: self.args.clone(),
        }
    }

    /// Extract grid changes from VT100 parser and generate keyframe/diff updates
    async fn extract_grid_changes(
        vt_parser: &Arc<Mutex<vt100::Parser>>,
        grid_state: &Arc<Mutex<HashMap<(u16, u16), GridCell>>>,
        cursor_pos: &Arc<Mutex<(u16, u16)>>,
        current_size: &Arc<Mutex<PtySize>>,
        previous_grid: &mut HashMap<(u16, u16), GridCell>,
    ) -> Option<GridUpdateMessage> {
        let parser_guard = vt_parser.lock().await;
        let screen = parser_guard.screen();
        let size_guard = current_size.lock().await;
        let size = size_guard.clone();
        drop(size_guard);

        let mut current_grid = HashMap::new();
        let mut changes = Vec::new();

        // Convert VT100 screen to our GridCell format
        for row in 0..size.rows {
            for col in 0..size.cols {
                if let Some(cell) = screen.cell(row, col) {
                    let grid_cell = GridCell {
                        char: cell.contents().to_string(),
                        fg_color: Self::color_to_hex(cell.fgcolor()),
                        bg_color: Self::color_to_hex(cell.bgcolor()),
                        bold: cell.bold(),
                        italic: cell.italic(),
                        underline: cell.underline(),
                    };

                    current_grid.insert((row, col), grid_cell.clone());

                    // Check if this cell changed from previous state
                    if let Some(prev_cell) = previous_grid.get(&(row, col)) {
                        if prev_cell != &grid_cell {
                            changes.push((row, col, grid_cell));
                        }
                    } else {
                        // New cell (wasn't in previous grid)
                        changes.push((row, col, grid_cell));
                    }
                }
            }
        }

        // Update cursor position
        let new_cursor = (screen.cursor_position().0, screen.cursor_position().1);
        let mut cursor_guard = cursor_pos.lock().await;
        let cursor_changed = *cursor_guard != new_cursor;
        *cursor_guard = new_cursor;
        drop(cursor_guard);

        // Update stored grid state
        {
            let mut grid_guard = grid_state.lock().await;
            *grid_guard = current_grid.clone();
        }

        let timestamp = std::time::SystemTime::now();

        // Generate appropriate update message
        if previous_grid.is_empty() {
            // First update - send keyframe
            *previous_grid = current_grid.clone();
            Some(GridUpdateMessage::Keyframe {
                size: size.into(),
                cells: current_grid,
                cursor: new_cursor,
                timestamp,
            })
        } else if !changes.is_empty() || cursor_changed {
            // Send incremental diff
            *previous_grid = current_grid;
            Some(GridUpdateMessage::Diff {
                changes,
                cursor: if cursor_changed {
                    Some(new_cursor)
                } else {
                    None
                },
                timestamp,
            })
        } else {
            // No changes
            None
        }
    }

    /// Generate a keyframe from current terminal state
    async fn generate_keyframe(
        vt_parser: &Arc<Mutex<vt100::Parser>>,
        cursor_pos: &Arc<Mutex<(u16, u16)>>,
        current_size: &Arc<Mutex<PtySize>>,
    ) -> GridUpdateMessage {
        let parser_guard = vt_parser.lock().await;
        let screen = parser_guard.screen();
        let size_guard = current_size.lock().await;
        let size = size_guard.clone();
        drop(size_guard);

        let mut current_grid = HashMap::new();

        // Convert VT100 screen to our GridCell format
        for row in 0..size.rows {
            for col in 0..size.cols {
                if let Some(cell) = screen.cell(row, col) {
                    let grid_cell = GridCell {
                        char: cell.contents().to_string(),
                        fg_color: Self::color_to_hex(cell.fgcolor()),
                        bg_color: Self::color_to_hex(cell.bgcolor()),
                        bold: cell.bold(),
                        italic: cell.italic(),
                        underline: cell.underline(),
                    };

                    current_grid.insert((row, col), grid_cell);
                }
            }
        }

        // Get cursor position
        let cursor_guard = cursor_pos.lock().await;
        let cursor = *cursor_guard;
        drop(cursor_guard);

        GridUpdateMessage::Keyframe {
            size: size.into(),
            cells: current_grid,
            cursor,
            timestamp: std::time::SystemTime::now(),
        }
    }

    /// Convert VT100 color to hex string
    fn color_to_hex(color: vt100::Color) -> Option<String> {
        match color {
            vt100::Color::Default => None,
            vt100::Color::Idx(idx) => {
                // Convert 8-bit color index to approximate hex
                // This is a simplified mapping - could be more accurate
                match idx {
                    0 => Some("#000000".to_string()),  // Black
                    1 => Some("#800000".to_string()),  // Red
                    2 => Some("#008000".to_string()),  // Green
                    3 => Some("#808000".to_string()),  // Yellow
                    4 => Some("#000080".to_string()),  // Blue
                    5 => Some("#800080".to_string()),  // Magenta
                    6 => Some("#008080".to_string()),  // Cyan
                    7 => Some("#c0c0c0".to_string()),  // White
                    8 => Some("#808080".to_string()),  // Bright Black
                    9 => Some("#ff0000".to_string()),  // Bright Red
                    10 => Some("#00ff00".to_string()), // Bright Green
                    11 => Some("#ffff00".to_string()), // Bright Yellow
                    12 => Some("#0000ff".to_string()), // Bright Blue
                    13 => Some("#ff00ff".to_string()), // Bright Magenta
                    14 => Some("#00ffff".to_string()), // Bright Cyan
                    15 => Some("#ffffff".to_string()), // Bright White
                    _ => Some(format!(
                        "#{:02x}{:02x}{:02x}",
                        ((idx - 16) / 36) * 51,
                        (((idx - 16) % 36) / 6) * 51,
                        ((idx - 16) % 6) * 51
                    )),
                }
            }
            vt100::Color::Rgb(r, g, b) => Some(format!("#{:02x}{:02x}{:02x}", r, g, b)),
        }
    }
}

impl Drop for PtySession {
    fn drop(&mut self) {
        // Clean up background tasks
        if let Some(task) = self.reader_task.take() {
            task.abort();
        }
        if let Some(task) = self.input_task.take() {
            task.abort();
        }
        if let Some(task) = self.control_task.take() {
            task.abort();
        }
        if let Some(task) = self.debounce_task.take() {
            task.abort();
        }
    }
}

#[derive(Debug, Clone)]
pub struct SessionInfo {
    pub id: String,
    pub agent: String,
    pub args: Vec<String>,
}
