#!/bin/bash

# CodeMux Installation Script
# Usage: curl -sSf https://codemux.dev/install.sh | sh

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
REPO="codemuxlab/codemux-cli"
INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/bin}"

# Helper functions
log() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

error() {
    echo -e "${RED}[ERROR]${NC} $1"
    exit 1
}

success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

# Detect platform and architecture
detect_platform() {
    local platform
    case "$(uname -s)" in
        Darwin*) platform="apple-darwin" ;;
        Linux*) platform="unknown-linux-gnu" ;;
        *) error "Unsupported operating system: $(uname -s)" ;;
    esac
    echo "$platform"
}

detect_arch() {
    local arch
    case "$(uname -m)" in
        x86_64|amd64) arch="x86_64" ;;
        arm64|aarch64) arch="aarch64" ;;
        *) error "Unsupported architecture: $(uname -m)" ;;
    esac
    echo "$arch"
}

# Check if required tools are available
check_dependencies() {
    for tool in curl tar; do
        if ! command -v "$tool" >/dev/null 2>&1; then
            error "$tool is required but not installed"
        fi
    done
}

# Get the latest release version from GitHub
get_latest_version() {
    log "Fetching latest version..."
    local version
    version=$(curl -sSf "https://api.github.com/repos/$REPO/releases/latest" | \
              grep '"tag_name"' | \
              sed -E 's/.*"tag_name": *"([^"]+)".*/\1/')
    
    if [ -z "$version" ]; then
        error "Failed to fetch latest version"
    fi
    
    log "Latest version: $version"
    echo "$version"
}

# Download and extract the binary
download_and_install() {
    local version="$1"
    local platform="$2"
    local arch="$3"
    
    local binary_name="codemux-${arch}-${platform}"
    local download_url="https://github.com/$REPO/releases/download/$version/$binary_name.tar.gz"
    local temp_dir
    temp_dir=$(mktemp -d)
    
    log "Downloading CodeMux from $download_url"
    
    # Download the archive
    if ! curl -sSfL "$download_url" -o "$temp_dir/codemux.tar.gz"; then
        error "Failed to download CodeMux binary"
    fi
    
    # Extract the binary
    log "Extracting binary..."
    if ! tar -xzf "$temp_dir/codemux.tar.gz" -C "$temp_dir"; then
        error "Failed to extract binary"
    fi
    
    # Create install directory if it doesn't exist
    mkdir -p "$INSTALL_DIR"
    
    # Move binary to install directory
    log "Installing to $INSTALL_DIR/codemux"
    if ! mv "$temp_dir/codemux" "$INSTALL_DIR/codemux"; then
        error "Failed to install binary"
    fi
    
    # Make binary executable
    chmod +x "$INSTALL_DIR/codemux"
    
    # Cleanup
    rm -rf "$temp_dir"
    
    success "CodeMux installed successfully to $INSTALL_DIR/codemux"
}

# Check if install directory is in PATH
check_path() {
    if [[ ":$PATH:" != *":$INSTALL_DIR:"* ]]; then
        warn "Install directory $INSTALL_DIR is not in your PATH"
        echo
        echo "Add the following to your shell profile (~/.bashrc, ~/.zshrc, etc.):"
        echo "  export PATH=\"$INSTALL_DIR:\$PATH\""
        echo
        echo "Or run this command now:"
        echo "  echo 'export PATH=\"$INSTALL_DIR:\$PATH\"' >> ~/.$(basename $SHELL)rc"
    fi
}

# Verify installation
verify_installation() {
    if [ -x "$INSTALL_DIR/codemux" ]; then
        log "Verifying installation..."
        local version_output
        if version_output=$("$INSTALL_DIR/codemux" --version 2>/dev/null); then
            success "Installation verified: $version_output"
            return 0
        else
            warn "Binary installed but --version failed"
            return 1
        fi
    else
        error "Binary not found at $INSTALL_DIR/codemux"
    fi
}

# Main installation process
main() {
    echo "🚀 CodeMux Installer"
    echo "==================="
    echo
    
    # Check dependencies
    check_dependencies
    
    # Detect system
    local platform arch version
    platform=$(detect_platform)
    arch=$(detect_arch)
    log "Detected platform: $arch-$platform"
    
    # Get latest version
    version=$(get_latest_version)
    
    # Download and install
    download_and_install "$version" "$platform" "$arch"
    
    # Verify installation
    if verify_installation; then
        check_path
        echo
        success "CodeMux is ready to use!"
        echo
        echo "Get started:"
        echo "  codemux claude                  # Quick mode"
        echo "  codemux server start           # Server mode"
        echo "  codemux --help                 # See all commands"
        echo
        echo "Documentation: https://codemux.dev/docs"
    else
        error "Installation verification failed"
    fi
}

# Run main function
main "$@"