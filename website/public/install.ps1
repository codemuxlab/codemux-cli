# CodeMux Installation Script for Windows
# Usage: irm https://codemux.dev/install.ps1 | iex

param()

# Configuration
$Repo = "codemuxlab/codemux-cli"
$ErrorActionPreference = "Stop"

# Colors for output
function Write-Info($Message) {
    Write-Host "[INFO] $Message" -ForegroundColor Blue
}

function Write-Error($Message) {
    Write-Host "[ERROR] $Message" -ForegroundColor Red
    exit 1
}

function Write-Success($Message) {
    Write-Host "[SUCCESS] $Message" -ForegroundColor Green
}

# Get the latest release version from GitHub
function Get-LatestVersion {
    Write-Info "Fetching latest version..."
    
    try {
        $response = Invoke-RestMethod -Uri "https://api.github.com/repos/$Repo/releases/latest"
        $version = $response.tag_name
        
        if ([string]::IsNullOrEmpty($version)) {
            Write-Error "Failed to fetch latest version"
        }
        
        Write-Info "Latest version: $version"
        return $version
    }
    catch {
        Write-Error "Failed to fetch latest version: $($_.Exception.Message)"
    }
}

# Main installation process
function Install-Main {
    Write-Host "🚀 CodeMux Installer for Windows" -ForegroundColor Magenta
    Write-Host "=================================" -ForegroundColor Magenta
    Write-Host ""
    
    # Get latest version
    $version = Get-LatestVersion
    
    # Use the official installer from GitHub release
    $installerUrl = "https://github.com/$Repo/releases/download/$version/codemux-installer.ps1"
    
    Write-Info "Downloading and running official installer from $installerUrl"
    
    try {
        # Download and execute the official installer
        $installer = Invoke-WebRequest -Uri $installerUrl -UseBasicParsing
        Invoke-Expression $installer.Content
        
        Write-Success "CodeMux installation completed!"
        Write-Host ""
        Write-Host "Get started:"
        Write-Host "  codemux claude                  # Quick mode"
        Write-Host "  codemux server start           # Server mode" 
        Write-Host "  codemux --help                 # See all commands"
        Write-Host ""
        Write-Host "Documentation: https://codemux.dev/docs"
    }
    catch {
        Write-Error "Failed to run official installer: $($_.Exception.Message)"
    }
}

# Run main function
Install-Main