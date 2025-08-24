# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Codemux is a specialized terminal multiplexer for AI coding CLIs (claude, gemini, aider, etc.) with enhanced web UI support. Unlike generic terminal multiplexers, it:
- **Only runs whitelisted AI code agents** for security
- **Detects and intercepts interactive prompts** (text input, multi-select, confirmations) to provide native web UI components
- **Provides rich web interfaces** for CLI interactions instead of raw terminal emulation

Operating modes:
- **Quick mode**: Launch a single AI session immediately
- **Daemon mode**: Background service managing multiple project sessions

## Development Commands

### Build
```bash
cargo build
cargo build --release  # For production builds
```

### Run
```bash
cargo run
```

### Test
```bash
cargo test
cargo test -- --nocapture  # To see println! output during tests
```

### Format and Lint
```bash
cargo fmt        # Format code
cargo clippy     # Lint code
```

## Architecture Components

1. **CLI Interface** (using clap):
   - `codemux run <tool> [args]` - Quick launch mode
   - `codemux daemon` - Start daemon mode
   - `codemux add-project <path>` - Register a project
   - `codemux list` - List projects/sessions
   - `codemux stop` - Stop daemon

2. **Whitelist System**: 
   - Configurable list of allowed AI CLI tools (claude, gemini, aider, etc.)
   - Tool-specific prompt detection patterns

3. **PTY Management**:
   - Spawn and manage pseudo-terminals for each AI CLI session
   - Parse output to detect interactive prompts
   - Intercept and handle special sequences

4. **Web Interface**:
   - WebSocket for real-time terminal output
   - Native HTML components for:
     - Text inputs with proper validation
     - Multi-select checkboxes/dropdowns
     - File pickers for path inputs
     - Confirmation dialogs
   - Project/session management UI

5. **Session Management**:
   - Multiple concurrent AI sessions
   - Session persistence and reconnection
   - Project-based organization

## Key Dependencies to Consider

When implementing features, consider using:
- `clap` for CLI argument parsing
- `tokio` for async runtime
- `axum` or `actix-web` for web server
- `tokio-tungstenite` for WebSocket support
- `portable-pty` for cross-platform PTY management
- `serde` + `serde_json` for serialization
- `regex` for prompt pattern detection
- `notify` for file system watching (project changes)
- `sqlx` with SQLite for daemon state persistence

## Implementation Notes

- **Prompt Detection**: Parse ANSI escape codes and common prompt patterns from AI CLIs
- **UI Enhancement**: When detecting prompts, send structured JSON to web client instead of raw terminal output
- **Security**: Validate all commands against whitelist before execution
- **State Management**: In daemon mode, persist project list and session state to SQLite
- **Web UI**: Use modern web components, avoid terminal emulator for interactive prompts
- **Process Management**: Properly handle SIGTERM/SIGINT for graceful shutdown