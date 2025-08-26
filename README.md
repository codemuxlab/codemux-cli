# CodeMux

A specialized terminal multiplexer for AI coding CLIs (claude, gemini, aider, etc.) with enhanced web UI support. Unlike generic terminal multiplexers, CodeMux only runs whitelisted AI code agents and provides rich web interfaces for CLI interactions.

## Features

- **AI-Focused Terminal Multiplexing**: Run multiple AI coding sessions simultaneously
- **Rich Web Interface**: Modern React Native Web UI with terminal emulation
- **Smart Prompt Detection**: Intercepts interactive prompts and provides native web UI components
- **Independent Cell Rendering**: Optimized terminal rendering with granular updates
- **Project Management**: Organize sessions by project with daemon mode
- **Security-First**: Only runs approved AI code agents from whitelist
- **Session Persistence**: Maintain sessions across reconnections
- **Real-time Updates**: WebSocket-based communication with optimized payloads

## Quick Start

### Prerequisites

- [Rust](https://rustup.rs/) (latest stable)
- [Node.js](https://nodejs.org/) 18+ (for web UI)
- [just](https://github.com/casey/just) command runner (optional but recommended)

### Installation

```bash
# Clone the repository
git clone https://github.com/anthropics/codemux
cd codemux

# Setup development environment (installs all dependencies)
just setup

# Or manually:
cargo build --release
cd app && npm install
```

## Usage

### Quick Mode - Run a single session

```bash
# Using just (recommended)
just run-dev              # Build and run with development settings
just run-debug            # Run with debug logging enabled

# Or directly with cargo
cargo run -- run claude   # Start a claude session
cargo run -- run claude --debug  # With debug logging
```

### Daemon Mode - Manage multiple projects

```bash
# Start the daemon
just daemon
# Or: cargo run -- daemon

# Add a project  
just add-project /path/to/project
# Or: cargo run -- add-project /path/to/project

# List projects and sessions
just list
# Or: cargo run -- list
```

## Web Interface

Once a session is started, open `http://localhost:8080` in your browser to access:

- **High-Performance Terminal**: Grid-based rendering with independent cell updates
- **Native UI Components** for interactive prompts:
  - Text inputs with proper Enter key handling
  - Multi-select checkboxes and dropdowns
  - File/path pickers
  - Confirmation dialogs
- **Real-time Terminal Updates**: WebSocket-based with optimized JSON payloads
- **Session Management**: Switch between multiple AI agent sessions
- **Project Organization**: Group sessions by development projects
- **Debug Tools**: JSONL session capture and analysis

### Architecture

The web interface uses:
- **React Native Web** with NativeWind (Tailwind CSS)
- **Zustand** for state management with granular subscriptions  
- **WebSocket** communication with optimized grid updates
- **VT100 Terminal Emulation** with proper ANSI escape sequence handling

## Supported Code Agents

By default, the following code agents are whitelisted:
- claude (Claude Code CLI)
- gemini (Google Gemini CLI)  
- aider (AI pair programming)
- cursor (Cursor CLI)
- continue (Continue dev CLI)

Add more agents by editing the config file at `~/.config/codemux/config.toml`.

## Development

### Using Just (Recommended)

```bash
just                      # Show all available commands
just setup               # Setup development environment
just dev                 # Fast development build (skips React app)
just build               # Production build (includes React app)
just app-dev             # Start React Native Web dev server
just watch               # Watch mode for continuous development
just test                # Run all tests
just fmt                 # Format code
just clippy              # Run linter
just ci                  # Full CI pipeline
```

### Manual Commands

```bash
# Development builds (fast)
cargo build
cargo run

# Production builds (includes React Native Web build)
CODEMUX_BUILD_APP=1 cargo build
cargo build --release

# React Native Web development
cd app && npm start      # Development server
cd app && npm run build  # Build for production

# Testing and quality
cargo test
cargo fmt
cargo clippy
```

### Project Structure

```
codemux/
├── src/                 # Rust backend source
│   ├── main.rs         # CLI entry point
│   ├── web.rs          # WebSocket server
│   ├── pty_session.rs  # PTY management
│   └── ...
├── app/                # React Native Web frontend
│   ├── src/
│   │   ├── components/ # React components
│   │   └── stores/     # Zustand state management
│   └── package.json
├── static/             # Static web assets
├── build.rs           # Rust build script
└── justfile           # Development commands
```

## License

MIT