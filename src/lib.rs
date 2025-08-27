// CodeMux Library
// Terminal multiplexer for AI coding CLIs with server-client architecture

pub mod cli;
pub mod server;
pub mod client;
pub mod core;
pub mod utils;
pub mod assets;
pub mod capture;

// Re-export commonly used types
pub use core::{Config, SessionInfo, ProjectInfo, ProjectWithSessions};
pub use client::http::CodeMuxClient;
pub use server::SessionManager;

// Error handling
pub use anyhow::{Result, Error};