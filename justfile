# Codemux Build System

# Default recipe - show available commands
default:
    @just --list

# Development build (fast, skips React app)
dev:
    cargo build

# Production build with React app
build:
    CODEMUX_BUILD_APP=1 cargo build

# Release build with optimizations
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

# Install to local system
install:
    cargo install --path .

# Development workflow - build and run
run-dev: dev
    cargo run

# Production workflow - build and run
run-prod: build
    cargo run

# Run with debug logging
run-debug:
    cargo run -- run claude --debug

# Quick development iteration with file watching
watch:
    cargo watch -x 'run'

# Watch and run tests
watch-test:
    cargo watch -x test

# Full CI pipeline
ci: fmt clippy test build

# Setup development environment
setup:
    @echo "Setting up development environment..."
    @echo "Installing Rust dependencies..."
    cargo build
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
    cargo run -- daemon

# Add project to daemon
add-project path:
    cargo run -- add-project {{path}}

# List daemon projects
list:
    cargo run -- list