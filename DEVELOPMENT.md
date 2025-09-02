# Development Guide

This guide covers development setup, building, and contributing to CodeMux.

## Prerequisites

- [Rust](https://rustup.rs/) (latest stable)
- [Node.js](https://nodejs.org/) 18+ (for web UI)
- [just](https://github.com/casey/just) command runner (optional but recommended)

## Development Setup

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

## Development Commands

### Using Just (Recommended)

```bash
just                      # Show all available commands
just setup               # Setup development environment
just build               # Development build with React app (debug Rust + latest Expo)
just release             # Optimized release build (includes React app automatically)
just dev                 # Development workflow - debug mode (fast startup)
just run                 # Production workflow - release mode (includes React app automatically)
just app-dev             # Start React Native Web dev server
just watch               # Watch mode for continuous development
just test                # Run all tests
just fmt                 # Format code
just clippy              # Run linter
just ci                  # Full CI pipeline
```

### Manual Commands

```bash
# Development builds
cargo build                               # Development build (skips React app by default)
CODEMUX_BUILD_APP=1 cargo build         # Force React app build in dev mode
cargo build --release                    # Production build (includes React app automatically)

# Running
cargo run --bin codemux                  # Development run mode (debug, no React app by default)
cargo run --release --bin codemux       # Production run mode (includes React app automatically)

# React Native Web development
cd app && npm start      # Development server
cd app && npm run build  # Build for production

# Testing and quality
cargo test
cargo fmt
cargo clippy
```

## Project Structure

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
├── build.rs           # Rust build script
└── justfile           # Development commands
```

## Architecture

### Web Interface

The web interface uses:
- **React Native Web** with NativeWind (Tailwind CSS)
- **Zustand** for state management with granular subscriptions  
- **WebSocket** communication with optimized grid updates
- **VT100 Terminal Emulation** with proper ANSI escape sequence handling

### Backend

- **Axum** web server with WebSocket support
- **Portable PTY** for cross-platform terminal management
- **Tokio** async runtime
- **Channel-based architecture** for PTY communication

## Build System

The build system automatically handles React app inclusion:

- **Development mode** (`cargo build`): Skips React app by default for faster builds
- **Release mode** (`cargo build --release`): Always includes React app
- **Force React app** (`CODEMUX_BUILD_APP=1`): Explicitly includes React app in dev builds
- **Skip React app** (`SKIP_WEB_BUILD=1`): Always skips React app (for capture binary)

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Run tests and linting: `just ci`
5. Submit a pull request

## Code Quality

Before submitting changes:

```bash
just fmt        # Format code
just clippy     # Run Rust linter  
just app-lint   # Run React app linter
just test       # Run all tests
just ci         # Full CI pipeline
```

## Debugging

### Debug Logging

```bash
# Run with debug logging (uses RUST_LOG=debug)
just run-debug

# Or manually with log file
cargo run --bin codemux -- run claude --logfile debug.log

# Or with environment variable for verbose output
RUST_LOG=debug cargo run --bin codemux -- run claude
```

For TUI mode, use `--logfile` to write logs to a file. For server mode, logs go to stderr and can be redirected:
```bash
RUST_LOG=debug cargo run --bin codemux -- server start 2> server.log
```

### Session Capture

```bash
# Record a session to JSONL
just capture-record claude session.jsonl

# Analyze captured session
just capture-analyze session.jsonl
```

## Release Process

Releases are automated via cargo-dist:

1. Create a version tag: `git tag v0.1.1`
2. Push the tag: `git push origin v0.1.1`
3. GitHub Actions will build and publish to:
   - GitHub Releases
   - Homebrew tap (`codemuxlab/homebrew-tap`)