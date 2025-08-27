# CodeMux

A specialized terminal multiplexer for AI coding CLIs (claude, gemini, aider, etc.) with cross-platform React Native UI. Unlike generic terminal multiplexers, CodeMux is optimized for AI code agents and provides native mobile and web interfaces for CLI interactions.

## Features

- **AI-Focused Terminal Multiplexing**: Run multiple AI coding sessions simultaneously
- **Mobile-Ready Interface**: Code from anywhere with React Native UI that runs on phones, tablets, and desktops
- **Smart Prompt Detection**: Intercepts interactive prompts and provides native web UI components
- **Project Management**: Organize sessions by project with daemon mode
- **Session Persistence**: Maintain sessions across reconnections
- **Real-time Updates**: WebSocket-based communication for responsive interactions

## Quick Start

### Installation

#### Homebrew (Recommended)

```bash
# Install from our Homebrew tap
brew install codemuxlab/tap/codemux
```

#### npm

```bash
# Install globally via npm
npm install -g codemux

# Or run directly without installing
npx codemux run claude
```

#### From Source

For development or if you prefer building from source:

```bash
# Clone the repository
git clone https://github.com/codemuxlab/codemux-cli
cd codemux

# Setup development environment (installs all dependencies)
just setup

# Or manually:
cargo build --release
cd app && npm install
```

> **Prerequisites for building from source**: [Rust](https://rustup.rs/) (latest stable), [Node.js](https://nodejs.org/) 18+, and optionally [just](https://github.com/casey/just) command runner.

## Usage

### Quick Mode - Run a single session

```bash
# Start a Claude session
codemux run claude

# Session continuity options
codemux run claude --continue           # Continue most recent session
codemux run claude --resume <session>   # Resume specific session

# Additional options
codemux run claude --open               # Auto-open web interface
codemux run claude --port 3000          # Use custom port
codemux run claude --debug              # Enable debug logging
```

### Daemon Mode - Manage multiple projects

```bash
# Start the daemon
codemux daemon

# Add a project  
codemux add-project /path/to/project

# List projects and sessions
codemux list

# Stop the daemon
codemux stop
```

#### Running daemon with PM2 (Recommended)

For production use, run the daemon with [PM2](https://pm2.keymetrics.io/) for automatic restarts and process management:

```bash
# Install PM2 globally
npm install -g pm2

# Start daemon with PM2
pm2 start codemux --name "codemux-daemon" -- daemon

# View daemon status
pm2 status

# View daemon logs
pm2 logs codemux-daemon

# Restart daemon
pm2 restart codemux-daemon

# Stop daemon
pm2 stop codemux-daemon

# Auto-start daemon on system boot
pm2 startup
pm2 save
```

## Web Interface

Once a session is started, open `http://localhost:8765` in your browser to access (or use `--open` to open automatically):

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

## Supported Code Agents

### Currently Supported
- **claude** (Claude Code CLI) - Full support with session continuity

### Coming Soon
- gemini (Google Gemini CLI)  
- aider (AI pair programming)
- cursor (Cursor CLI)
- continue (Continue dev CLI)

> **Note**: While the codebase includes configurations for multiple agents, only Claude is fully supported and tested at this time. Other agents are available in the whitelist but may have limited functionality.

Add more agents by editing the config file at `~/.config/codemux/config.toml`.

## Development

For development setup, building, and contributing, see [DEVELOPMENT.md](DEVELOPMENT.md).

## License

MIT