# CodeMux

A terminal multiplexer for AI code agents (claude, gemini, aider, etc.) with enhanced web UI support.

## Features

- **Terminal Multiplexing**: Run multiple AI coding sessions simultaneously
- **Web Interface**: Access sessions through a modern web UI
- **Smart Prompt Detection**: Intercepts interactive prompts and provides native web UI components
- **Project Management**: Organize sessions by project
- **Whitelisted Agents**: Only runs approved AI code agents for security
- **Session Persistence**: Maintain sessions across reconnections

## Installation

```bash
cargo build --release
```

## Usage

### Quick Mode - Run a single session

```bash
# Start a claude session
codemux run claude

# Start gemini with arguments
codemux run gemini --model gemini-pro
```

### Daemon Mode - Manage multiple projects

```bash
# Start the daemon
codemux daemon --port 8080

# Add a project
codemux add-project /path/to/project --name "My Project"

# List projects and sessions
codemux list

# Stop the daemon
codemux stop
```

## Web Interface

Once a session is started, open `http://localhost:8080` in your browser to access:

- Terminal output with syntax highlighting
- Native UI components for prompts:
  - Text inputs with validation
  - Multi-select checkboxes
  - File/path pickers
  - Confirmation dialogs
- Session management
- Project switching

## Supported Code Agents

By default, the following code agents are whitelisted:
- claude (Claude Code CLI)
- gemini (Google Gemini CLI)  
- aider (AI pair programming)
- cursor (Cursor CLI)
- continue (Continue dev CLI)

Add more agents by editing the config file at `~/.config/codemux/config.toml`.

## Architecture

CodeMux uses:
- **PTY (Pseudo-Terminal)** for process management
- **WebSockets** for real-time communication
- **ANSI parsing** for prompt detection
- **Rust/Tokio** for async operations

## Development

```bash
# Run tests
cargo test

# Format code
cargo fmt

# Lint
cargo clippy
```

## License

MIT