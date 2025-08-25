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

        tracing::info!("Spawning command: {} with args: {:?}", agent, args);
        let _child = pty_pair.slave.spawn_command(cmd)?;
        tracing::debug!("Command spawned successfully");

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
        };

        Ok((session, channels))
    }

    /// Start the PTY session tasks - runs until completion or error
    pub async fn start(self) -> Result<()> {
        tracing::info!("Starting PTY session tasks for agent: {}", self.agent);

        // Extract all channels and state before creating tasks
        let PtySession {
            pty,
            writer,
            current_size,
            buffer,
            vt_parser,
            grid_state,
            cursor_pos,
            last_activity,
            input_rx,
            output_tx,
            control_rx,
            size_tx,
            grid_tx,
            ..
        } = self;

        // Clone the reader for the reader task - use std::sync::Mutex for blocking context
        let reader = Arc::new(std::sync::Mutex::new(pty.lock().await.try_clone_reader()?));
        tracing::debug!("PTY reader cloned successfully");

        // Create channel for sending raw data from blocking reader to async processor
        let (raw_data_tx, mut raw_data_rx) = mpsc::unbounded_channel::<Vec<u8>>();

        // Create the blocking PTY reader task
        let reader_task = tokio::task::spawn_blocking(move || {
            tracing::debug!("PTY reader task started, beginning read loop");
            let mut read_buffer = [0u8; 1024];
            let mut read_count = 0u64;

            loop {
                let read_result = {
                    let mut reader_guard = reader.lock().expect("Failed to lock reader");
                    read_count += 1;
                    tracing::debug!("PTY read attempt #{}", read_count);
                    reader_guard.read(&mut read_buffer)
                };

                match read_result {
                    Ok(0) => {
                        tracing::info!("PTY reader reached EOF");
                        break;
                    }
                    Ok(n) => {
                        let data = read_buffer[..n].to_vec();

                        // Debug PTY output
                        let data_str = String::from_utf8_lossy(&data);
                        let printable: String = data_str
                            .chars()
                            .take(100)
                            .map(|c| {
                                if c.is_control() && c != '\n' && c != '\r' && c != '\t' {
                                    format!("\\x{:02x}", c as u8)
                                } else {
                                    c.to_string()
                                }
                            })
                            .collect();
                        tracing::debug!("PTY read {} bytes: '{}'", n, printable);

                        // Send data to async processor
                        if raw_data_tx.send(data).is_err() {
                            tracing::error!("Failed to send PTY data to async processor");
                            break;
                        }
                    }
                    Err(e) => {
                        tracing::error!(
                            "Error reading from PTY: {}, error kind: {:?}",
                            e,
                            e.kind()
                        );
                        // Don't break immediately on some recoverable errors
                        if e.kind() == std::io::ErrorKind::Interrupted
                            || e.kind() == std::io::ErrorKind::WouldBlock
                        {
                            tracing::debug!("Recoverable PTY read error, continuing");
                            std::thread::sleep(std::time::Duration::from_millis(50));
                            continue;
                        }
                        break;
                    }
                }

                // Small sleep to avoid busy waiting
                std::thread::sleep(std::time::Duration::from_millis(10));
            }

            tracing::info!("PTY reader task exiting");
        });

        // Create async data processor task
        let processor_buffer = buffer.clone();
        let processor_vt_parser = vt_parser.clone();
        let processor_grid_state = grid_state.clone();
        let processor_cursor_pos = cursor_pos.clone();
        let processor_current_size = current_size.clone();
        let processor_last_activity = last_activity.clone();
        let processor_output_tx = output_tx.clone();
        let processor_grid_tx = grid_tx.clone();

        let processor_task = tokio::spawn(async move {
            let mut previous_grid: HashMap<(u16, u16), GridCell> = HashMap::new();
            let mut pending_data: Vec<Vec<u8>> = Vec::new();
            let mut last_data_time = std::time::Instant::now();
            let debounce_delay = tokio::time::Duration::from_millis(16); // True debounce: wait for inactivity

            loop {
                tokio::select! {
                    // Collect incoming data
                    data = raw_data_rx.recv() => {
                        match data {
                            Some(data) => {
                                pending_data.push(data);
                                last_data_time = std::time::Instant::now(); // Update last activity time
                            }
                            None => break, // Channel closed
                        }
                    }

                    // True debouncing: process after period of inactivity
                    _ = tokio::time::sleep_until(tokio::time::Instant::from_std(last_data_time + debounce_delay)) => {
                        if pending_data.is_empty() {
                            continue;
                        }

                        // Only process if there's been no new data for the debounce period
                        if last_data_time.elapsed() >= debounce_delay {
                            // Process all accumulated data at once
                            tracing::debug!("Processing {} accumulated data chunks after {}ms of inactivity",
                                pending_data.len(), last_data_time.elapsed().as_millis());

                            // First, update buffer and parse all data through VT100
                        let mut all_data = Vec::new();
                        for data in pending_data.drain(..) {
                            // Update the terminal buffer
                            {
                                let mut buffer_guard = processor_buffer.lock().await;
                                buffer_guard.extend_from_slice(&data);

                                // Keep buffer size reasonable (last 64KB of output)
                                if buffer_guard.len() > 65536 {
                                    let drain_count = buffer_guard.len() - 65536;
                                    buffer_guard.drain(0..drain_count);
                                }
                            }

                            // Process through VT100 parser
                            {
                                let mut parser_guard = processor_vt_parser.lock().await;
                                parser_guard.process(&data);
                            }

                            all_data.extend_from_slice(&data);
                        }

                        // Log first 100 chars of processed data for debugging
                        let data_sample = String::from_utf8_lossy(&all_data[..all_data.len().min(100)]).replace('\x1b', "\\x1b");
                        tracing::debug!("VT100 parser processed {} total bytes: '{}'", all_data.len(), data_sample);

                        // Now generate a single grid update for all changes
                        let grid_update = Self::extract_grid_changes(
                            &processor_vt_parser,
                            &processor_grid_state,
                            &processor_cursor_pos,
                            &processor_current_size,
                            &mut previous_grid,
                        )
                        .await;

                        if let Some(update) = &grid_update {
                            // Categorize the types of changes for debugging
                            match update {
                                GridUpdateMessage::Keyframe { size, cells, cursor, .. } => {
                                    tracing::debug!(
                                        "Generated keyframe: {} total cells, size {}x{}, cursor: ({}, {})",
                                        cells.len(),
                                        size.rows,
                                        size.cols,
                                        cursor.0,
                                        cursor.1
                                    );
                                }
                                GridUpdateMessage::Diff { changes, cursor, .. } => {
                                    let mut clear_changes = 0;
                                    let mut text_changes = 0;
                                    let mut style_changes = 0;

                                    // Log first 10 changes for debugging
                                    let mut sample_changes = Vec::new();

                                    for (row, col, cell) in changes {
                                        if cell.char == " " {
                                            clear_changes += 1;
                                        } else if cell.char.chars().all(|c| c.is_ascii_graphic() || c.is_ascii_whitespace()) {
                                            text_changes += 1;
                                        } else {
                                            style_changes += 1;
                                        }

                                        // Collect sample of changes for detailed analysis
                                        if sample_changes.len() < 10 {
                                            let char_repr = if cell.char == " " {
                                                "[SPACE]".to_string()
                                            } else if cell.char.chars().any(|c| c.is_control()) {
                                                format!("[CTRL:{:?}]", cell.char.chars().collect::<Vec<_>>())
                                            } else {
                                                cell.char.clone()
                                            };

                                            // Show style info for debugging
                                            let style_info = if cell.bold || cell.italic || cell.underline || cell.fg_color.is_some() || cell.bg_color.is_some() {
                                                format!("(b:{},i:{},u:{},fg:{:?},bg:{:?})",
                                                    cell.bold, cell.italic, cell.underline,
                                                    cell.fg_color, cell.bg_color)
                                            } else {
                                                "".to_string()
                                            };

                                            sample_changes.push(format!("({},{})='{}'{}", row, col, char_repr, style_info));
                                        }
                                    }

                                    let cursor_info = if let Some(c) = cursor {
                                        format!("({}, {})", c.0, c.1)
                                    } else {
                                        "unchanged".to_string()
                                    };

                                    tracing::debug!(
                                        "Generated grid diff: {} total changes ({} clears, {} text, {} style), cursor: {}",
                                        changes.len(),
                                        clear_changes,
                                        text_changes,
                                        style_changes,
                                        cursor_info
                                    );

                                    if !sample_changes.is_empty() {
                                        tracing::debug!("Sample changes: {}", sample_changes.join(", "));
                                    }

                                    // Show which screen regions are changing most
                                    let mut region_counts = std::collections::HashMap::new();
                                    for (row, _col, _cell) in changes {
                                        let region = match *row {
                                            0..=5 => "top",
                                            6..=15 => "upper-mid",
                                            16..=35 => "middle",
                                            36..=45 => "lower-mid",
                                            _ => "bottom",
                                        };
                                        *region_counts.entry(region).or_insert(0) += 1;
                                    }

                                    let region_summary: Vec<String> = region_counts.iter()
                                        .map(|(region, count)| format!("{}:{}", region, count))
                                        .collect();

                                    if !region_summary.is_empty() {
                                        tracing::debug!("Changes by region: {}", region_summary.join(", "));
                                    }
                                }
                            }
                            let _ = processor_grid_tx.send(update.clone());
                        } else {
                            tracing::debug!("No grid update generated (no changes)");
                        }

                        // Update last activity time for debounce timer
                        {
                            let mut activity_guard = processor_last_activity.lock().await;
                            *activity_guard = Instant::now();
                        }

                        // Send raw bytes to subscribers (for backward compatibility)
                        if !all_data.is_empty() {
                            let msg = PtyOutputMessage {
                                data: all_data,
                                timestamp: std::time::SystemTime::now(),
                            };
                            let _ = processor_output_tx.send(msg);
                        }
                        } else {
                            // Still receiving data, keep waiting
                            continue;
                        }
                    }
                }
            }

            tracing::info!("PTY data processor task exiting");
        });

        // Create input handler task
        let input_writer = writer.clone();
        let input_task = tokio::spawn(async move {
            let mut input_rx = input_rx;
            while let Some(msg) = input_rx.recv().await {
                let mut writer_guard = input_writer.lock().await;
                if let Err(e) = writer_guard.write_all(&msg.data) {
                    tracing::error!("Failed to write to PTY: {}", e);
                    break;
                }
                let _ = writer_guard.flush();
            }
        });

        // Create control handler task
        let control_pty = pty.clone();
        let control_current_size = current_size.clone();
        let control_size_tx = size_tx.clone();
        let control_vt_parser = vt_parser.clone();
        let control_cursor_pos = cursor_pos.clone();

        let control_task = tokio::spawn(async move {
            let mut control_rx = control_rx;
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
                        let pty_guard = control_pty.lock().await;
                        match pty_guard.resize(new_size) {
                            Ok(()) => {
                                drop(pty_guard); // Release PTY lock early

                                // Update tracked size
                                {
                                    let mut size_guard = control_current_size.lock().await;
                                    *size_guard = new_size;
                                }

                                // CRITICAL: Update VT100 parser size to match PTY
                                {
                                    let mut parser_guard = control_vt_parser.lock().await;
                                    parser_guard.set_size(rows, cols);
                                }

                                // Clear cursor position to prevent out-of-bounds issues
                                {
                                    let mut cursor_guard = control_cursor_pos.lock().await;
                                    *cursor_guard = (0, 0);
                                }

                                // Broadcast size change to all subscribers
                                let _ = control_size_tx.send(new_size);
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
                            &control_vt_parser,
                            &control_cursor_pos,
                            &control_current_size,
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
        });

        // Note: Automatic keyframes removed - keyframes are only sent on client request
        // via the request_keyframe() method to avoid unnecessary full redraws

        // Send a newline to Claude to wake it up and show the initial prompt
        // tracing::debug!("Sending initial newline to wake up Claude");
        // {
        //     let mut writer_guard = writer.lock().await;
        //     if let Err(e) = writer_guard.write_all(b"\n") {
        //         tracing::warn!("Failed to send initial newline to Claude: {}", e);
        //     } else {
        //         let _ = writer_guard.flush();
        //         tracing::debug!("Initial newline sent to Claude");
        //     }
        // }

        // Run all tasks concurrently and return when any fails or all complete
        tracing::debug!("Starting all PTY tasks concurrently");
        tokio::select! {
            result = reader_task => {
                tracing::info!("PTY reader task completed");
                result.map_err(|e| anyhow::anyhow!("Reader task failed: {}", e))?;
            }
            result = processor_task => {
                tracing::info!("PTY processor task completed");
                result.map_err(|e| anyhow::anyhow!("Processor task failed: {}", e))?;
            }
            result = input_task => {
                tracing::info!("PTY input task completed");
                result.map_err(|e| anyhow::anyhow!("Input task failed: {}", e))?;
            }
            result = control_task => {
                tracing::info!("PTY control task completed");
                result.map_err(|e| anyhow::anyhow!("Control task failed: {}", e))?;
            }
        }

        tracing::info!("PTY session completed");
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

        // Get regions that likely changed by comparing to VT100 dirty state
        // For performance, we'll check all cells but optimize the comparison
        let mut cells_to_check = std::collections::HashSet::new();

        // First pass: collect all positions that currently have content
        for row in 0..size.rows {
            for col in 0..size.cols {
                if let Some(cell) = screen.cell(row, col) {
                    let content = cell.contents().to_string();
                    if !content.trim().is_empty() {
                        cells_to_check.insert((row, col));
                    }
                }
            }
        }

        // Second pass: add all positions that previously had content (to detect cleared cells)
        for &(row, col) in previous_grid.keys() {
            cells_to_check.insert((row, col));
        }

        // Third pass: process only the cells we need to check
        for &(row, col) in &cells_to_check {
            if let Some(cell) = screen.cell(row, col) {
                let content = cell.contents().to_string();

                if !content.trim().is_empty() {
                    let grid_cell = GridCell {
                        char: content,
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
                } else if previous_grid.contains_key(&(row, col)) {
                    // Cell is empty now but was previously non-empty - this is a change
                    changes.push((
                        row,
                        col,
                        GridCell {
                            char: " ".to_string(),
                            fg_color: None,
                            bg_color: None,
                            bold: false,
                            italic: false,
                            underline: false,
                        },
                    ));
                }
            } else if previous_grid.contains_key(&(row, col)) {
                // Cell no longer exists but was previously present - cleared
                changes.push((
                    row,
                    col,
                    GridCell {
                        char: " ".to_string(),
                        fg_color: None,
                        bg_color: None,
                        bold: false,
                        italic: false,
                        underline: false,
                    },
                ));
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
            tracing::debug!("Sending keyframe with {} cells", current_grid.len());
            Some(GridUpdateMessage::Keyframe {
                size: size.into(),
                cells: current_grid,
                cursor: new_cursor,
                timestamp,
            })
        } else if !changes.is_empty() || cursor_changed {
            // Send incremental diff
            *previous_grid = current_grid;
            tracing::debug!(
                "Sending diff with {} changes, cursor_changed: {}",
                changes.len(),
                cursor_changed
            );
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
            tracing::trace!("No grid changes detected");
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

        // Debug keyframe generation
        let non_empty_count = current_grid
            .values()
            .filter(|cell| !cell.char.trim().is_empty())
            .count();
        let sample_content: String = current_grid
            .values()
            .filter_map(|cell| {
                if !cell.char.trim().is_empty() {
                    Some(cell.char.as_str())
                } else {
                    None
                }
            })
            .take(10)
            .collect::<Vec<_>>()
            .join("");

        tracing::debug!(
            "Generated keyframe: {} total cells, {} non-empty, size {}x{}, cursor=({},{}), sample: '{}'",
            current_grid.len(),
            non_empty_count,
            size.rows,
            size.cols,
            cursor.0,
            cursor.1,
            sample_content.replace('\n', "\\n").replace('\r', "\\r")
        );

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

#[derive(Debug, Clone)]
pub struct SessionInfo {
    pub id: String,
    pub agent: String,
    pub args: Vec<String>,
}
