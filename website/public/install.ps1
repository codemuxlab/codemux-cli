# CodeMux Installation Script for Windows
# Usage: irm https://codemux.dev/install.ps1 | iex

param(
    [string]$InstallDir = "$env:USERPROFILE\.local\bin"
)

# Configuration
$Repo = "codemuxlab/codemux-cli"
$ErrorActionPreference = "Stop"

# Colors for output
function Write-Info($Message) {
    Write-Host "[INFO] $Message" -ForegroundColor Blue
}

function Write-Warn($Message) {
    Write-Host "[WARN] $Message" -ForegroundColor Yellow
}

function Write-Error($Message) {
    Write-Host "[ERROR] $Message" -ForegroundColor Red
    exit 1
}

function Write-Success($Message) {
    Write-Host "[SUCCESS] $Message" -ForegroundColor Green
}

# Detect platform and architecture
function Get-Platform {
    return "pc-windows-msvc"
}

function Get-Architecture {
    $arch = $env:PROCESSOR_ARCHITECTURE
    switch ($arch) {
        "AMD64" { return "x86_64" }
        "ARM64" { return "aarch64" }
        default { Write-Error "Unsupported architecture: $arch" }
    }
}

# Check if required tools are available
function Test-Dependencies {
    $tools = @("curl")
    foreach ($tool in $tools) {
        if (-not (Get-Command $tool -ErrorAction SilentlyContinue)) {
            Write-Error "$tool is required but not installed. Please install curl first."
        }
    }
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

# Download and extract the binary
function Install-CodeMux($Version, $Platform, $Arch) {
    $binaryName = "codemux-$Arch-$Platform"
    $downloadUrl = "https://github.com/$Repo/releases/download/$Version/$binaryName.zip"
    $tempDir = New-TemporaryFile | ForEach-Object { Remove-Item $_; New-Item -ItemType Directory -Path $_ }
    $zipPath = Join-Path $tempDir "codemux.zip"
    
    Write-Info "Downloading CodeMux from $downloadUrl"
    
    try {
        # Download the archive
        Invoke-WebRequest -Uri $downloadUrl -OutFile $zipPath
        
        # Extract the binary
        Write-Info "Extracting binary..."
        Expand-Archive -Path $zipPath -DestinationPath $tempDir -Force
        
        # Create install directory if it doesn't exist
        if (-not (Test-Path $InstallDir)) {
            New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
        }
        
        # Find the extracted binary
        $binaryPath = Get-ChildItem -Path $tempDir -Name "codemux.exe" -Recurse | Select-Object -First 1
        if (-not $binaryPath) {
            Write-Error "Binary not found in archive"
        }
        
        $sourceBinary = Join-Path $tempDir $binaryPath
        $targetBinary = Join-Path $InstallDir "codemux.exe"
        
        # Move binary to install directory
        Write-Info "Installing to $targetBinary"
        Copy-Item -Path $sourceBinary -Destination $targetBinary -Force
        
        # Cleanup
        Remove-Item -Path $tempDir -Recurse -Force
        
        Write-Success "CodeMux installed successfully to $targetBinary"
    }
    catch {
        Write-Error "Failed to download/install binary: $($_.Exception.Message)"
    }
}

# Check if install directory is in PATH
function Test-PathConfiguration {
    $currentPath = $env:PATH
    if ($currentPath -notlike "*$InstallDir*") {
        Write-Warn "Install directory $InstallDir is not in your PATH"
        Write-Host ""
        Write-Host "To add it to your PATH permanently:"
        Write-Host "  [Environment]::SetEnvironmentVariable('Path', `$env:Path + ';$InstallDir', 'User')" -ForegroundColor Cyan
        Write-Host ""
        Write-Host "Or add it to your current session:"
        Write-Host "  `$env:PATH += ';$InstallDir'" -ForegroundColor Cyan
    }
}

# Verify installation
function Test-Installation {
    $binaryPath = Join-Path $InstallDir "codemux.exe"
    
    if (Test-Path $binaryPath) {
        Write-Info "Verifying installation..."
        try {
            $versionOutput = & $binaryPath --version 2>$null
            Write-Success "Installation verified: $versionOutput"
            return $true
        }
        catch {
            Write-Warn "Binary installed but --version failed"
            return $false
        }
    }
    else {
        Write-Error "Binary not found at $binaryPath"
    }
}

# Main installation process
function Install-Main {
    Write-Host "🚀 CodeMux Installer for Windows" -ForegroundColor Magenta
    Write-Host "=================================" -ForegroundColor Magenta
    Write-Host ""
    
    # Check dependencies
    Test-Dependencies
    
    # Detect system
    $platform = Get-Platform
    $arch = Get-Architecture
    Write-Info "Detected platform: $arch-$platform"
    
    # Get latest version
    $version = Get-LatestVersion
    
    # Download and install
    Install-CodeMux $version $platform $arch
    
    # Verify installation
    if (Test-Installation) {
        Test-PathConfiguration
        Write-Host ""
        Write-Success "CodeMux is ready to use!"
        Write-Host ""
        Write-Host "Get started:"
        Write-Host "  codemux run claude              # Quick mode"
        Write-Host "  codemux server start           # Server mode" 
        Write-Host "  codemux --help                 # See all commands"
        Write-Host ""
        Write-Host "Documentation: https://codemux.dev/docs"
    }
    else {
        Write-Error "Installation verification failed"
    }
}

# Run main function
Install-Main