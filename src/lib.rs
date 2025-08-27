// CodeMux Library
// Terminal multiplexer for AI coding CLIs with server-client architecture

pub mod assets;
pub mod capture;
pub mod cli;
pub mod client;
pub mod core;
pub mod server;
pub mod utils;

// Re-export commonly used types
pub use client::http::CodeMuxClient;
pub use core::{Config, ProjectInfo, ProjectWithSessions, SessionInfo};
pub use server::SessionManagerHandle;

// Error handling
pub use anyhow::{Error, Result};
