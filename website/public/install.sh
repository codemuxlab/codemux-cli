#!/bin/bash

# CodeMux Installation Script
# Usage: curl -sSfL https://codemux.dev/install.sh | sh

set -euo pipefail

# Configuration
REPO="codemuxlab/codemux-cli"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Helper functions - output to stderr to avoid mixing with function returns
log() {
    printf "${BLUE}[INFO]${NC} %s\n" "$1" >&2
}

error() {
    printf "${RED}[ERROR]${NC} %s\n" "$1" >&2
    exit 1
}

success() {
    printf "${GREEN}[SUCCESS]${NC} %s\n" "$1" >&2
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

# Main installation process
main() {
    echo "🚀 CodeMux Installer" >&2
    echo "===================" >&2
    echo >&2
    
    # Get latest version
    local version
    version=$(get_latest_version)
    
    # Use the official installer from GitHub release
    local installer_url="https://github.com/$REPO/releases/download/$version/codemux-installer.sh"
    
    log "Downloading and running official installer from $installer_url"
    
    # Download and execute the official installer
    if ! curl --proto '=https' --tlsv1.2 -LsSf "$installer_url" | sh; then
        error "Failed to run official installer"
    fi
    
    success "CodeMux installation completed!"
    echo >&2
    echo "Get started:" >&2
    echo "  codemux claude                  # Quick mode" >&2
    echo "  codemux server start           # Server mode" >&2
    echo "  codemux --help                 # See all commands" >&2
    echo >&2
    echo "Documentation: https://codemux.dev/docs" >&2
}

# Run main function
main "$@"