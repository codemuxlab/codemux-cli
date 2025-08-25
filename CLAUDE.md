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
cargo run                    # Normal run mode
cargo run -- run claude --debug  # Debug mode (logs to /tmp/codemux-debug.log)
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

3. **PTY Session Management**:
   - **Channel-based Architecture**: PTY sessions communicate via channels (tokio::sync in run mode, WebSocket in daemon mode)
   - **PTY Session Component**: Independent component managing subprocess and PTY I/O, not tied to TUI or web UI
   - **Multiple Client Support**: Both TUI and Web UI are equal clients consuming PTY output and sending input
   - **Unified Interface**: Same channel abstraction works for both run mode (local) and daemon mode (WebSocket-based)

4. **Client Architecture**:
   - **TUI Client**: Full terminal interface sending complete input stream (keystrokes, control sequences, resize events)
   - **Web UI Client**: Translates web interactions (form inputs, dropdowns, buttons) into appropriate terminal input sequences
   - **Input Routing**: Both clients send input to PTY session via input channel
   - **Output Consumption**: Both clients subscribe to PTY output broadcast channel
   - **PTY Control**: Both clients can send control messages (resize, etc.) via control channel

5. **Web Interface**:
   - Grid-based terminal emulation using VT100 parser state from TUI
   - Native HTML components for interactive prompts:
     - Text inputs with proper validation
     - Multi-select checkboxes/dropdowns
     - File pickers for path inputs
     - Confirmation dialogs
   - Project/session management UI
   - Real-time synchronization with TUI terminal state

6. **Session Management**:
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

### Core Architecture
- **Channel Abstraction**: PTY sessions expose input/output/control channels. TUI and Web UI are both clients using same channel interface.
- **PTY Session Independence**: PTY sessions are standalone components, not owned by TUI or web server. They manage subprocess lifecycle independently.
- **Client Equality**: TUI and Web UI are equal clients. Both can resize PTY, send input, receive output.
- **Mode Consistency**: Same architecture for run mode (local channels) and daemon mode (WebSocket channels).

### Input/Output Handling  
- **TUI Input**: Sends complete input stream (raw keystrokes, escape sequences, control characters) directly to PTY
- **Web UI Input**: Translates web form interactions (text inputs, selects, buttons) into corresponding terminal input sequences
- **Output Distribution**: PTY session broadcasts output to all connected clients via channels
- **Grid Synchronization**: Web UI receives grid updates from TUI's VT100 parser state, not raw PTY output

### Technical Details
- **Prompt Detection**: Parse ANSI escape codes and common prompt patterns from AI CLIs
- **UI Enhancement**: When detecting prompts, send structured JSON to web client instead of raw terminal output
- **Security**: Validate all commands against whitelist before execution
- **State Management**: In daemon mode, persist project list and session state to SQLite
- **PTY Sizing**: Both TUI and Web UI can control PTY size. PTY session arbitrates resize requests (last-writer-wins).
  - **TODO**: Size consensus mechanism needed. Currently both clients can keep changing size and see their size not met, leading to resize conflicts. Need to implement consensus algorithm (priority-based, negotiation, or coordinator pattern) to handle multiple clients requesting different sizes simultaneously.
- **Process Management**: Properly handle SIGTERM/SIGINT for graceful shutdown
- **Debug Logging**: In debug mode (`--debug` flag), all tracing output is written to `/tmp/codemux-debug.log` to avoid interfering with TUI display. In normal mode, only ERROR level messages are logged and discarded.
- **Output to Terminal**: Use `eprintln!` instead of `println!` to avoid interfering with the TUI display. The TUI uses stdout for rendering, so any `println!` calls will corrupt the display. Use `eprintln!` for debugging or error messages that need to go to stderr.