use std::io;
use tokio::sync::mpsc;
use tracing_subscriber::fmt::MakeWriter;

#[derive(Debug, Clone, PartialEq)]
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl LogLevel {
    pub fn from_str(s: &str) -> Self {
        match s.to_uppercase().as_str() {
            "ERROR" => LogLevel::Error,
            "WARN" => LogLevel::Warn,
            "INFO" => LogLevel::Info,
            "DEBUG" => LogLevel::Debug,
            "TRACE" => LogLevel::Trace,
            _ => LogLevel::Info, // Default fallback
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            LogLevel::Error => "ERROR",
            LogLevel::Warn => "WARN",
            LogLevel::Info => "INFO",
            LogLevel::Debug => "DEBUG",
            LogLevel::Trace => "TRACE",
        }
    }
}

#[derive(Debug, Clone)]
pub struct LogEntry {
    pub level: LogLevel,
    pub message: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// A custom writer that captures tracing output and sends it to the TUI
pub struct TuiWriter {
    sender: mpsc::UnboundedSender<LogEntry>,
}

impl TuiWriter {
    pub fn new() -> (Self, mpsc::UnboundedReceiver<LogEntry>) {
        let (sender, receiver) = mpsc::unbounded_channel();
        (TuiWriter { sender }, receiver)
    }
}

impl io::Write for TuiWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let log_text = String::from_utf8_lossy(buf);

        // Parse the tracing output format
        // Expected format: "2025-08-24T16:43:07.498408Z ERROR codemux::web: WebSocket: Session ... not found"
        if let Some(parsed) = parse_tracing_line(&log_text) {
            let _ = self.sender.send(parsed);
        }

        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl<'a> MakeWriter<'a> for TuiWriter {
    type Writer = Self;

    fn make_writer(&'a self) -> Self::Writer {
        self.clone()
    }
}

impl Clone for TuiWriter {
    fn clone(&self) -> Self {
        TuiWriter {
            sender: self.sender.clone(),
        }
    }
}

fn parse_tracing_line(line: &str) -> Option<LogEntry> {
    let line = line.trim();
    if line.is_empty() {
        return None;
    }

    // Try to extract timestamp, level, and message
    let parts: Vec<&str> = line.splitn(3, ' ').collect();
    if parts.len() < 3 {
        // Fallback for lines that don't match expected format
        return Some(LogEntry {
            level: LogLevel::Info,
            message: line.to_string(),
            timestamp: chrono::Utc::now(),
        });
    }

    let timestamp_str = parts[0];
    let level_str = parts[1];
    let message_part = parts[2];

    // Parse timestamp - if it fails, treat the whole line as a simple message
    let timestamp = if let Ok(parsed) = chrono::DateTime::parse_from_rfc3339(timestamp_str) {
        parsed.with_timezone(&chrono::Utc)
    } else {
        // This doesn't look like a tracing line, treat as simple message
        return Some(LogEntry {
            level: LogLevel::Info,
            message: line.to_string(),
            timestamp: chrono::Utc::now(),
        });
    };

    // Extract level using enum
    let level = LogLevel::from_str(level_str);

    // Clean up message (remove module path if present)
    let message = if let Some(colon_pos) = message_part.find(": ") {
        message_part[colon_pos + 2..].to_string()
    } else {
        message_part.to_string()
    };

    Some(LogEntry {
        level,
        message,
        timestamp,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_tracing_line() {
        let line =
            "2025-08-24T16:43:07.498408Z ERROR codemux::web: WebSocket: Session abc not found";
        let parsed = parse_tracing_line(line).unwrap();

        assert_eq!(parsed.level, LogLevel::Error);
        assert_eq!(parsed.message, "WebSocket: Session abc not found");
    }

    #[test]
    fn test_parse_simple_line() {
        let line = "A simple log message with multiple words";
        let parsed = parse_tracing_line(line).unwrap();

        assert_eq!(parsed.level, LogLevel::Info);
        assert_eq!(parsed.message, "A simple log message with multiple words");
    }
}
