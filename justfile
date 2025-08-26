# Codemux Build System

# Default recipe - show available commands
default:
    @just --list

# Development build with React app (debug Rust + latest Expo)
build:
    CODEMUX_BUILD_APP=1 cargo build

# Release build with optimizations (includes React app automatically)
release:
    cargo build --release

# Build capture binary only (fast, skips React app)
capture:
    SKIP_WEB_BUILD=1 cargo build --release --bin codemux-capture

# Build React Native Web app only
app-build:
    cd app && npm run build

# Start React Native Web development server
app-dev:
    cd app && npm start

# Install dependencies for React app
app-install:
    cd app && npm install

# Clean all build artifacts
clean:
    cargo clean
    cd app && rm -rf dist _expo node_modules/.cache

# Run tests
test:
    cargo test
    cargo test -- --nocapture

# Run clippy linter
clippy:
    cargo clippy

# Format code
fmt:
    cargo fmt

# Lint React Native app
app-lint:
    cd app && npm run lint

# Lint both Rust and React app
lint-all: clippy app-lint

# Install to local system
install:
    cargo install --path .

# Development workflow - debug mode
dev *args:
    cargo run --bin codemux {{ args }}

# Production workflow - release mode (includes React app automatically)
run *args:
    cargo run --release --bin codemux {{ args }}

# Run with debug logging
run-debug:
    cargo run --bin codemux -- run claude --debug

# Quick development iteration with file watching
watch:
    cargo watch -x 'run --bin codemux'

# Watch and run tests
watch-test:
    cargo watch -x test

# Full CI pipeline
ci: fmt lint-all test release

# Setup development environment
setup:
    @echo "Setting up development environment..."
    @echo "Installing Rust dependencies..."
    just build
    @echo "Installing Node.js dependencies..."
    cd app && npm install
    @echo "✅ Setup complete!"

# Run capture session recording
capture-record agent output:
    SKIP_WEB_BUILD=1 cargo run --bin codemux-capture -- --agent {{agent}} --output {{output}}

# Run capture analysis
capture-analyze file:
    SKIP_WEB_BUILD=1 cargo run --bin codemux-capture -- --analyze {{file}} --verbose

# Start daemon mode
daemon:
    cargo run --bin codemux -- daemon

# Add project to daemon
add-project path:
    cargo run --bin codemux -- add-project {{path}}

# List daemon projects
list:
    cargo run --bin codemux -- list