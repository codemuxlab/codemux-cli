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

### Just Commands (Recommended - uses justfile)
```bash
just                      # Show all available commands
just setup               # Setup development environment (installs all deps)

# Build commands
just dev                 # Development build (fast, skips React app)
just build               # Production build (includes React app)
just release             # Optimized release build
just capture             # Build capture binary only (fast)

# Run commands  
just run-dev             # Build and run in development mode
just run-prod            # Build and run in production mode
just run-debug           # Run with debug logging
just daemon              # Start daemon mode

# Capture system
just capture-record claude session.jsonl  # Record session to JSONL
just capture-analyze session.jsonl        # Analyze captured session

# React Native app
just app-dev             # Start React app dev server
just app-build           # Build React app only
just app-install         # Install React app dependencies

# Development workflow
just watch               # Watch mode for development iteration
just watch-test          # Watch mode for tests
just ci                  # Full CI pipeline (fmt, clippy, test, build)

# Maintenance
just fmt                 # Format code
just clippy              # Lint code
just test                # Run tests
just clean               # Clean all build artifacts
```

### Direct Cargo Commands
```bash
# Build
cargo build                               # Development build (skips React app)
cargo build --release                    # Production build (includes React app)  
CODEMUX_BUILD_APP=1 cargo build         # Force React app build in dev
SKIP_WEB_BUILD=1 cargo build --bin codemux-capture  # Capture binary only

# Run
cargo run                                # Normal run mode
cargo run -- run claude --debug         # Debug mode (logs to /tmp/codemux-debug.log)
cargo run -- daemon                     # Daemon mode

# Capture system
SKIP_WEB_BUILD=1 cargo run --bin codemux-capture -- --agent claude --output session.jsonl
SKIP_WEB_BUILD=1 cargo run --bin codemux-capture -- --analyze session.jsonl --verbose

# Test & Quality
cargo test                               # Run tests
cargo test -- --nocapture               # Show println! output during tests
cargo fmt                                # Format code  
cargo clippy                             # Lint code
```

### React Native App (Expo)
```bash
cd app
npm install              # Install dependencies
npx expo start          # Start development server
npx expo start --web    # Start web development server
npx expo export         # Export for production
```

**Note**: The React Native app uses:
- **NativeWind** for Tailwind CSS styling in React Native
- **Zustand** for state management
- **Expo** for cross-platform development

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
   - Grid-based terminal emulation using VT100 parser state from PTY session
   - Fixed-size terminal with scaling modes:
     - **Fit mode**: Scale terminal to fit available space with proper centering
     - **Original mode**: Show terminal at actual size with scrollbars
   - Native HTML components for interactive prompts:
     - Text inputs with proper Enter key handling (preventDefault)
     - Multi-select checkboxes/dropdowns
     - File pickers for path inputs
     - Confirmation dialogs
   - Project/session management UI
   - Real-time terminal updates via WebSocket grid messages
   - JSONL session streaming for debugging and analysis

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
- **TUI Input**: Sends individual keystrokes including `\r` for Enter key directly to PTY
- **Web UI Input**: Sends message text and `\r` separately to mimic terminal behavior (text content + submission signal)
- **Input Processing**: AI agents expect text content and carriage return (`\r`) as separate events for proper input processing
- **Output Distribution**: PTY session broadcasts output to all connected clients via channels
- **Grid Synchronization**: Web UI receives grid updates from PTY's VT100 parser state with proper cursor visibility tracking
- **Cursor Handling**: Real cursor is often hidden by Claude (`\x1b[?25l`), fake cursor created with reverse video styling (`\x1b[7m \x1b[27m`)

### Technical Details
- **Prompt Detection**: Parse ANSI escape codes and common prompt patterns from AI CLIs
- **UI Enhancement**: When detecting prompts, send structured JSON to web client instead of raw terminal output
- **Security**: Validate all commands against whitelist before execution
- **State Management**: In daemon mode, persist project list and session state to SQLite
- **PTY Sizing**: Both TUI and Web UI can control PTY size. PTY session arbitrates resize requests (last-writer-wins).
  - **Web UI Scaling**: Implements proper scaling with `translate()` + `scale()` transforms, dimension validation, and centering
  - **Resize Handling**: Clear transforms during resize operations to prevent conflicts, use proper timing with requestAnimationFrame
- **Process Management**: Properly handle SIGTERM/SIGINT for graceful shutdown
- **Debug Logging**: In debug mode (`--debug` flag), all tracing output is written to `/tmp/codemux-debug.log` to avoid interfering with TUI display. In normal mode, only ERROR level messages are logged and discarded.
- **Output to Terminal**: Use `eprintln!` instead of `println!` to avoid interfering with the TUI display. The TUI uses stdout for rendering, so any `println!` calls will corrupt the display. Use `eprintln!` for debugging or error messages that need to go to stderr.

### Grid Cell Structure
The `GridCell` struct represents terminal content with full styling support:
```rust
pub struct GridCell {
    pub char: String,           // Character content
    pub fg_color: Option<String>, // Foreground color (hex)
    pub bg_color: Option<String>, // Background color (hex)
    pub bold: bool,             // Bold text
    pub italic: bool,           // Italic text
    pub underline: bool,        // Underlined text
    pub reverse: bool,          // Reverse video (for fake cursors)
}
```

### Capture and Analysis System
- **JSONL Recording**: Real-time session capture to JSON Lines format for debugging and analysis
- **VT100 Processing**: Compare different chunking strategies (immediate vs batched) to debug cursor positioning
- **Event Types**: Support for `RawPtyOutput`, `GridUpdate`, `Input`, and `Resize` events with precise timestamps
- **Analysis Tools**: Built-in tools to analyze cursor movement patterns, timing, and VT100 sequence processing