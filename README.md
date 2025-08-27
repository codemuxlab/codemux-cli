# CodeMux

A specialized terminal multiplexer for AI coding CLIs (claude, gemini, aider, etc.) with cross-platform React Native UI. Unlike generic terminal multiplexers, CodeMux uses a server-client architecture optimized for AI code agents and provides native mobile and web interfaces for CLI interactions.

## Features

- **Server-Client Architecture**: tmux-like session management with persistent AI agent sessions
- **Mobile-Ready Interface**: Code from anywhere with React Native UI that runs on phones, tablets, and desktops
- **Smart Prompt Detection**: Intercepts interactive prompts and provides native web UI components
- **Session Persistence**: Sessions survive client disconnection and can be reattached
- **Multi-Client Access**: Multiple clients can connect to the same session simultaneously
- **Unified Web Interface**: Single web server manages all sessions across projects
- **Project Management**: Organize sessions by project with centralized management
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

### Server-Client Architecture (0.1+)

CodeMux uses a server-client model similar to tmux. The server manages all AI agent sessions and the client connects to these sessions.

```bash
# Start a Claude session (auto-starts server if needed)
codemux run claude

# Server management
codemux server                    # Start server explicitly
codemux server status            # Check server status  
codemux server stop             # Stop server

# Session management
codemux list                    # List all active sessions
codemux attach <session-id>     # Attach to existing session
codemux kill-session <session-id>  # Terminate specific session

# Session continuity options
codemux run claude --continue           # Continue most recent session
codemux run claude --resume <session>   # Resume specific session

# Additional options
codemux run claude --open               # Auto-open web interface
```

### Project Management

```bash
# Add a project  
codemux add-project /path/to/project --name "My Project"

# List projects
codemux list-projects

# Create session in project context
codemux run claude --project <project-id>
```

#### Running server as system service (Optional)

For persistent server operation, you can install the server as a system service:

**With PM2:**
```bash
# Install PM2 globally
npm install -g pm2

# Start server with PM2
pm2 start codemux --name "codemux-server" -- server

# Auto-start server on system boot
pm2 startup
pm2 save
```

**With systemd (Linux):**
```bash
# Create systemd service file
sudo tee /etc/systemd/system/codemux.service > /dev/null <<EOF
[Unit]
Description=CodeMux Server
After=network.target

[Service]
Type=simple
User=$USER
ExecStart=$(which codemux) server
Restart=always

[Install]
WantedBy=multi-user.target
EOF

# Enable and start service
sudo systemctl enable codemux
sudo systemctl start codemux
```

## Web Interface

Once the server is running, open `http://localhost:8080` in your browser to access all sessions (or use `--open` to open automatically):

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