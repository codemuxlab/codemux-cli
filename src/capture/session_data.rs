use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufReader, BufWriter, Write};
use std::path::Path;
use std::time::{Duration, SystemTime};

/// Represents a single I/O event in the session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SessionEvent {
    /// Input from user to the agent
    Input {
        timestamp: u32, // milliseconds since start
        data: Vec<u8>,
    },
    /// Output from agent to user
    Output {
        timestamp: u32, // milliseconds since start
        data: Vec<u8>,
    },
    /// Terminal resize event
    Resize {
        timestamp: u32, // milliseconds since start
        rows: u16,
        cols: u16,
    },
    /// Grid update from PTY session
    GridUpdate {
        timestamp_begin: u32, // milliseconds since start when processing began
        timestamp_end: u32,   // milliseconds since start when processing completed
        size: (u16, u16),
        cells: Vec<GridCellWithPos>, // Changed from HashMap to Vec for JSON compatibility
        cursor: (u16, u16),
    },
    /// Raw PTY output for direct capture
    RawPtyOutput {
        timestamp_begin: u32, // milliseconds since start when data received
        timestamp_end: u32,   // milliseconds since start when data processed
        data: Vec<u8>,        // Raw bytes from PTY including ANSI sequences
    },
}

/// Terminal grid cell representation (same as pty_session)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GridCell {
    pub char: String,
    pub fg_color: Option<String>,
    pub bg_color: Option<String>,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub reverse: bool,
}

/// Grid cell with position for JSON serialization
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GridCellWithPos {
    pub row: u16,
    pub col: u16,
    pub cell: GridCell,
}

/// Complete session recording with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionRecording {
    pub metadata: SessionMetadata,
    pub events: Vec<SessionEvent>,
}

/// Metadata about the recorded session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMetadata {
    pub agent: String,
    pub args: Vec<String>,
    pub start_time: SystemTime,
    pub duration: Duration,
    pub total_events: usize,
    pub version: String,
}

impl SessionRecording {
    /// Create a new empty recording
    pub fn new(agent: String, args: Vec<String>) -> Self {
        Self {
            metadata: SessionMetadata {
                agent,
                args,
                start_time: SystemTime::now(),
                duration: Duration::ZERO,
                total_events: 0,
                version: "1.0".to_string(),
            },
            events: Vec::new(),
        }
    }

    /// Add an event to the recording
    pub fn add_event(&mut self, event: SessionEvent) {
        self.events.push(event);
        self.metadata.total_events = self.events.len();
    }

    /// Finalize the recording by calculating duration
    pub fn finalize(&mut self) {
        if let Some(first_event) = self.events.first() {
            if let Some(last_event) = self.events.last() {
                let first_ts = self.get_event_timestamp(first_event);
                let last_ts = self.get_event_timestamp(last_event);
                self.metadata.duration = Duration::from_millis((last_ts - first_ts) as u64);
            }
        }
    }

    /// Get timestamp from any event
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

    /// Save recording to file
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let file = File::create(path)?;
        let writer = BufWriter::new(file);
        serde_json::to_writer_pretty(writer, self)?;
        Ok(())
    }

    /// Load recording from file
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let recording = serde_json::from_reader(reader)?;
        Ok(recording)
    }

    /// Get events in a time range (in milliseconds)
    pub fn get_events_in_range(&self, start: u32, end: u32) -> Vec<&SessionEvent> {
        self.events
            .iter()
            .filter(|event| {
                let ts = self.get_event_timestamp(event);
                ts >= start && ts <= end
            })
            .collect()
    }

    /// Get total duration in milliseconds
    pub fn total_duration(&self) -> u32 {
        self.metadata.duration.as_millis() as u32
    }

    /// Find the closest event at or before a timestamp (in milliseconds)
    pub fn find_event_at_timestamp(&self, timestamp: u32) -> Option<usize> {
        for (i, event) in self.events.iter().enumerate() {
            if self.get_event_timestamp(event) >= timestamp {
                return if i == 0 { Some(0) } else { Some(i - 1) };
            }
        }
        if !self.events.is_empty() {
            Some(self.events.len() - 1)
        } else {
            None
        }
    }

    /// Get the next significant timestamp (for seeking, in milliseconds)
    pub fn next_timestamp(&self, current: u32) -> Option<u32> {
        self.events
            .iter()
            .map(|e| self.get_event_timestamp(e))
            .find(|&ts| ts > current + 100) // At least 100ms difference
    }

    /// Get the previous significant timestamp (for seeking, in milliseconds)
    pub fn prev_timestamp(&self, current: u32) -> Option<u32> {
        self.events
            .iter()
            .map(|e| self.get_event_timestamp(e))
            .filter(|&ts| ts < current.saturating_sub(100)) // At least 100ms difference
            .last()
    }
}

/// JSONL streaming writer for real-time event recording
pub struct JsonlRecorder {
    writer: BufWriter<File>,
    metadata: SessionMetadata,
    start_time: SystemTime,
}

impl JsonlRecorder {
    /// Create a new JSONL recorder
    pub fn new<P: AsRef<Path>>(path: P, agent: String, args: Vec<String>) -> Result<Self> {
        let file = File::create(path)?;
        let mut writer = BufWriter::new(file);

        let metadata = SessionMetadata {
            agent,
            args,
            start_time: SystemTime::now(),
            duration: Duration::ZERO,
            total_events: 0,
            version: "1.0".to_string(),
        };

        // Write metadata as first line
        let metadata_json = serde_json::to_string(&metadata)?;
        writeln!(writer, "{}", metadata_json)?;
        writer.flush()?;

        Ok(Self {
            writer,
            metadata,
            start_time: SystemTime::now(),
        })
    }

    /// Write an event to the JSONL file
    pub fn write_event(&mut self, event: &SessionEvent) -> Result<()> {
        let event_json = serde_json::to_string(event)?;
        writeln!(self.writer, "{}", event_json)?;
        self.writer.flush()?;
        Ok(())
    }

    /// Get session metadata
    pub fn metadata(&self) -> &SessionMetadata {
        &self.metadata
    }

    /// Get session start time
    pub fn start_time(&self) -> SystemTime {
        self.start_time
    }

    /// Get elapsed time since session start
    pub fn elapsed_time(&self) -> Duration {
        self.start_time.elapsed().unwrap_or(Duration::ZERO)
    }

    /// Finalize the recording
    pub fn finalize(mut self) -> Result<()> {
        self.writer.flush()?;
        Ok(())
    }
}
