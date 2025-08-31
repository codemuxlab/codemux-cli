use anyhow::Result;
use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEventKind},
    execute, terminal,
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, Paragraph},
    Frame, Terminal,
};
use std::collections::HashMap;
use std::io::{self, Stdout};
use std::time::{Duration, Instant};
use tokio::time::interval;

use crate::capture::session_data::{GridCell, SessionEvent, SessionRecording};
use tui_term::vt100::Parser;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PlaybackState {
    Playing,
    Paused,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PlaybackSpeed {
    Normal, // 1x
    Double, // 2x
}

impl PlaybackSpeed {
    fn multiplier(self) -> f64 {
        match self {
            PlaybackSpeed::Normal => 1.0,
            PlaybackSpeed::Double => 2.0,
        }
    }

    fn toggle(self) -> Self {
        match self {
            PlaybackSpeed::Normal => PlaybackSpeed::Double,
            PlaybackSpeed::Double => PlaybackSpeed::Normal,
        }
    }
}

pub struct ReplaySession {
    terminal: Terminal<CrosstermBackend<Stdout>>,
    recording: SessionRecording,

    // Playback state
    current_time: u32, // milliseconds since start
    playback_state: PlaybackState,
    playback_speed: PlaybackSpeed,

    // Terminal state for rendering
    terminal_grid: HashMap<(u16, u16), GridCell>,
    terminal_cursor: (u16, u16),
    terminal_size: (u16, u16),

    // VT100 parser for raw PTY data
    vt_parser: Option<Parser>,

    // UI state
    last_update: Instant,
    current_event_index: usize,
}

impl ReplaySession {
    pub fn new(recording: SessionRecording, start_time: u32, auto_play: bool) -> Result<Self> {
        // Setup terminal
        terminal::enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, terminal::EnterAlternateScreen, cursor::Hide)?;
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;

        let playback_state = if auto_play {
            PlaybackState::Playing
        } else {
            PlaybackState::Paused
        };

        // Find starting event index
        let current_event_index = recording.find_event_at_timestamp(start_time).unwrap_or(0);

        // Check if recording contains raw PTY output
        let has_raw_output = recording
            .events
            .iter()
            .any(|e| matches!(e, SessionEvent::RawPtyOutput { .. }));

        // Create VT100 parser if we have raw output
        let vt_parser = if has_raw_output {
            Some(Parser::new(30, 120, 0))
        } else {
            None
        };

        Ok(Self {
            terminal,
            recording,
            current_time: start_time,
            playback_state,
            playback_speed: PlaybackSpeed::Normal,
            terminal_grid: HashMap::new(),
            terminal_cursor: (0, 0),
            terminal_size: (30, 120),
            vt_parser,
            last_update: Instant::now(),
            current_event_index,
        })
    }

    pub async fn start_playback(&mut self) -> Result<()> {
        println!("▶️ Starting playback...");
        println!("🎮 Controls: Space=Play/Pause, ←→=Seek, 2=Speed, Q=Quit");

        // Apply initial state up to current time
        self.apply_state_up_to_time(self.current_time).await;

        let mut tick_interval = interval(Duration::from_millis(50)); // 20 FPS updates
        let mut should_quit = false;

        loop {
            tokio::select! {
                // Handle UI updates and playback
                _ = tick_interval.tick() => {
                    if self.playback_state == PlaybackState::Playing {
                        self.update_playback_time().await;
                        self.apply_current_events().await;
                    }

                    self.draw_ui()?;
                }

                // Handle keyboard input
                _ = async {
                    if event::poll(Duration::from_millis(10)).unwrap_or(false) {
                        if let Ok(Event::Key(key)) = event::read() {
                                if key.kind == KeyEventKind::Press {
                                    match key.code {
                                        KeyCode::Char('q') | KeyCode::Char('Q') => {
                                            should_quit = true;
                                        }
                                        KeyCode::Char(' ') => {
                                            self.toggle_playback();
                                        }
                                        KeyCode::Char('2') => {
                                            self.playback_speed = self.playback_speed.toggle();
                                        }
                                        KeyCode::Left => {
                                            self.seek_backward().await;
                                        }
                                        KeyCode::Right => {
                                            self.seek_forward().await;
                                        }
                                        KeyCode::Home => {
                                            self.seek_to_start().await;
                                        }
                                        KeyCode::End => {
                                            self.seek_to_end().await;
                                        }
                                        _ => {}
                                    }
                                }
                        }
                    }
                } => {}
            }

            if should_quit {
                break;
            }
        }

        // Cleanup
        self.cleanup()?;
        println!("✅ Playback completed");

        Ok(())
    }

    async fn update_playback_time(&mut self) {
        let elapsed = self.last_update.elapsed().as_millis() as u32;
        let adjusted = (elapsed as f64 * self.playback_speed.multiplier()) as u32;
        self.current_time = self.current_time.saturating_add(adjusted);
        self.last_update = Instant::now();

        // Don't go past the end
        let max_time = self.recording.total_duration();
        if self.current_time >= max_time {
            self.current_time = max_time;
            self.playback_state = PlaybackState::Paused;
        }
    }

    async fn apply_current_events(&mut self) {
        // Apply all events up to current time that haven't been applied
        let events_to_apply: Vec<SessionEvent> = self
            .recording
            .events
            .iter()
            .skip(self.current_event_index)
            .take_while(|event| self.get_event_timestamp(event) <= self.current_time)
            .cloned()
            .collect();

        for event in events_to_apply {
            self.apply_event(&event).await;
            self.current_event_index += 1;
        }
    }

    async fn apply_state_up_to_time(&mut self, time: u32) {
        // Reset state
        self.terminal_grid.clear();
        self.terminal_cursor = (0, 0);
        self.current_event_index = 0;

        // Collect events to apply
        let events_to_apply: Vec<(usize, SessionEvent)> = self
            .recording
            .events
            .iter()
            .enumerate()
            .take_while(|(_, event)| self.get_event_timestamp(event) <= time)
            .map(|(i, event)| (i, event.clone()))
            .collect();

        // Apply all events up to the target time
        for (i, event) in events_to_apply {
            self.apply_event(&event).await;
            self.current_event_index = i + 1;
        }
    }

    async fn apply_event(&mut self, event: &SessionEvent) {
        match event {
            SessionEvent::GridUpdate {
                size,
                cells,
                cursor,
                ..
            } => {
                self.terminal_size = *size;
                // Convert Vec<GridCellWithPos> back to HashMap
                self.terminal_grid = cells
                    .iter()
                    .map(|cell_with_pos| {
                        (
                            (cell_with_pos.row, cell_with_pos.col),
                            cell_with_pos.cell.clone(),
                        )
                    })
                    .collect();
                self.terminal_cursor = *cursor;
            }
            SessionEvent::Resize { rows, cols, .. } => {
                self.terminal_size = (*rows, *cols);
                // Update VT100 parser size if present
                if let Some(parser) = &mut self.vt_parser {
                    parser.set_size(*rows, *cols);
                }
            }
            SessionEvent::RawPtyOutput { data, .. } => {
                // Process raw PTY data through VT100 parser
                if let Some(parser) = &mut self.vt_parser {
                    parser.process(data);
                    let screen = parser.screen();

                    // Convert VT100 screen to our grid format
                    self.terminal_grid.clear();
                    let (rows, cols) = self.terminal_size;

                    for row in 0..rows {
                        for col in 0..cols {
                            if let Some(cell) = screen.cell(row, col) {
                                if !cell.contents().is_empty() {
                                    let grid_cell = GridCell {
                                        char: cell.contents().to_string(),
                                        fg_color: None, // TODO: Extract colors
                                        bg_color: None,
                                        bold: cell.bold(),
                                        italic: cell.italic(),
                                        underline: cell.underline(),
                                        reverse: cell.inverse(),
                                    };
                                    self.terminal_grid.insert((row, col), grid_cell);
                                }
                            }
                        }
                    }

                    // Update cursor position
                    let cursor_pos = screen.cursor_position();
                    self.terminal_cursor = (cursor_pos.0, cursor_pos.1);
                }
            }
            // Input/Output events could be shown in a log panel if needed
            _ => {}
        }
    }

    fn get_event_timestamp(&self, event: &SessionEvent) -> u32 {
        match event {
            SessionEvent::Input { timestamp, .. } => *timestamp,
            SessionEvent::Output { timestamp, .. } => *timestamp,
            SessionEvent::Resize { timestamp, .. } => *timestamp,
            SessionEvent::GridUpdate {
                timestamp_begin, ..
            } => *timestamp_begin,
            SessionEvent::RawPtyOutput {
                timestamp_begin, ..
            } => *timestamp_begin,
        }
    }

    fn toggle_playback(&mut self) {
        self.playback_state = match self.playback_state {
            PlaybackState::Playing => PlaybackState::Paused,
            PlaybackState::Paused => PlaybackState::Playing,
        };
        self.last_update = Instant::now(); // Reset timing
    }

    async fn seek_forward(&mut self) {
        if let Some(next_time) = self.recording.next_timestamp(self.current_time) {
            self.current_time = next_time;
            self.apply_state_up_to_time(self.current_time).await;
        }
    }

    async fn seek_backward(&mut self) {
        if let Some(prev_time) = self.recording.prev_timestamp(self.current_time) {
            self.current_time = prev_time;
            self.apply_state_up_to_time(self.current_time).await;
        }
    }

    async fn seek_to_start(&mut self) {
        self.current_time = 0;
        self.apply_state_up_to_time(self.current_time).await;
    }

    async fn seek_to_end(&mut self) {
        self.current_time = self.recording.total_duration();
        self.apply_state_up_to_time(self.current_time).await;
    }

    fn draw_ui(&mut self) -> Result<()> {
        let recording_agent = self.recording.metadata.agent.clone();
        let current_time = self.current_time;
        let total_duration = self.recording.total_duration();
        let playback_state = self.playback_state;
        let playback_speed = self.playback_speed;
        let terminal_grid = self.terminal_grid.clone();
        let terminal_cursor = self.terminal_cursor;
        let terminal_size = self.terminal_size;

        self.terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3), // Controls bar
                    Constraint::Min(0),    // Terminal content
                    Constraint::Length(3), // Progress bar
                ])
                .split(f.area());

            // Controls bar
            Self::draw_controls_bar_static(
                f,
                chunks[0],
                &recording_agent,
                current_time,
                total_duration,
                playback_state,
                playback_speed,
            );

            // Terminal content
            Self::draw_terminal_content_static(
                f,
                chunks[1],
                &terminal_grid,
                terminal_cursor,
                terminal_size,
            );

            // Progress bar
            Self::draw_progress_bar_static(f, chunks[2], current_time, total_duration);
        })?;

        Ok(())
    }

    fn draw_controls_bar_static(
        f: &mut Frame,
        area: Rect,
        agent: &str,
        current_time: u32,
        total_duration: u32,
        playback_state: PlaybackState,
        playback_speed: PlaybackSpeed,
    ) {
        let state_symbol = match playback_state {
            PlaybackState::Playing => "▶️",
            PlaybackState::Paused => "⏸️",
        };

        let speed_text = match playback_speed {
            PlaybackSpeed::Normal => "1x",
            PlaybackSpeed::Double => "2x",
        };

        let controls_text = format!(
            "{} {} | Agent: {} | Time: {:.1}s/{:.1}s",
            state_symbol,
            speed_text,
            agent,
            current_time as f64 / 1000.0,
            total_duration as f64 / 1000.0
        );

        let controls = Paragraph::new(controls_text)
            .style(Style::default().fg(Color::Cyan))
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("🎮 Controls: Space=Play/Pause, ←→=Seek, 2=Speed, Q=Quit"),
            );

        f.render_widget(controls, area);
    }

    fn draw_terminal_content_static(
        f: &mut Frame,
        area: Rect,
        terminal_grid: &HashMap<(u16, u16), GridCell>,
        terminal_cursor: (u16, u16),
        terminal_size: (u16, u16),
    ) {
        // Convert terminal grid to ratatui content
        let terminal_content = Self::render_terminal_grid_static(
            terminal_grid,
            terminal_cursor,
            terminal_size,
            area.height,
            area.width,
        );

        let terminal_widget = Paragraph::new(terminal_content).block(
            Block::default()
                .title("📺 Terminal Output")
                .borders(Borders::ALL),
        );

        f.render_widget(terminal_widget, area);
    }

    fn render_terminal_grid_static(
        terminal_grid: &HashMap<(u16, u16), GridCell>,
        terminal_cursor: (u16, u16),
        terminal_size: (u16, u16),
        display_height: u16,
        display_width: u16,
    ) -> Vec<Line> {
        let mut lines = Vec::new();
        let (grid_rows, grid_cols) = terminal_size;

        // Render each row of the terminal
        for row in 0..std::cmp::min(grid_rows, display_height) {
            let mut line_spans = Vec::new();
            let mut current_line = String::new();
            let mut current_style = Style::default();

            // Build line from grid cells
            for col in 0..std::cmp::min(grid_cols, display_width) {
                let is_cursor = (row, col) == terminal_cursor;

                if let Some(cell) = terminal_grid.get(&(row, col)) {
                    // Convert grid cell to styled content
                    let mut cell_style = Style::default()
                        .fg(cell
                            .fg_color
                            .as_ref()
                            .and_then(|c| Self::parse_hex_color_static(c))
                            .unwrap_or(Color::Reset))
                        .bg(cell
                            .bg_color
                            .as_ref()
                            .and_then(|c| Self::parse_hex_color_static(c))
                            .unwrap_or(Color::Reset));

                    if cell.bold {
                        cell_style = cell_style.add_modifier(Modifier::BOLD);
                    }
                    if cell.italic {
                        cell_style = cell_style.add_modifier(Modifier::ITALIC);
                    }
                    if cell.underline {
                        cell_style = cell_style.add_modifier(Modifier::UNDERLINED);
                    }

                    // Highlight cursor position
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

        // Fill remaining display lines if needed
        while lines.len() < display_height as usize {
            lines.push(Line::from(" "));
        }

        lines
    }

    fn parse_hex_color_static(hex: &str) -> Option<Color> {
        if hex.starts_with('#') && hex.len() == 7 {
            if let Ok(r) = u8::from_str_radix(&hex[1..3], 16) {
                if let Ok(g) = u8::from_str_radix(&hex[3..5], 16) {
                    if let Ok(b) = u8::from_str_radix(&hex[5..7], 16) {
                        return Some(Color::Rgb(r, g, b));
                    }
                }
            }
        }
        None
    }

    fn draw_progress_bar_static(f: &mut Frame, area: Rect, current_time: u32, total_duration: u32) {
        let progress = if total_duration > 0 {
            (current_time as f64 / total_duration as f64).min(1.0)
        } else {
            0.0
        };

        let progress_bar = Gauge::default()
            .block(Block::default().borders(Borders::ALL).title("⏰ Progress"))
            .gauge_style(Style::default().fg(Color::Green))
            .ratio(progress);

        f.render_widget(progress_bar, area);
    }

    fn cleanup(&mut self) -> Result<()> {
        terminal::disable_raw_mode()?;
        execute!(
            self.terminal.backend_mut(),
            terminal::LeaveAlternateScreen,
            cursor::Show
        )?;
        Ok(())
    }
}

impl Drop for ReplaySession {
    fn drop(&mut self) {
        let _ = self.cleanup();
    }
}
