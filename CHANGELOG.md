# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

### Changed
- Simplified installation scripts to use official cargo-dist installers
- Installation scripts now automatically detect and use latest version

### Fixed
- Fixed installation script compatibility with cargo-dist .tar.xz format
- Fixed shell output redirection to prevent URL corruption in installers


## [0.1.7] - 2025-09-02

### Added
- WSL support documentation and installation compatibility
- Smart platform-aware installation button with inline copy functionality
- Installation scripts with automatic latest version detection

### Changed
- Updated website to use Fumadocs framework for better documentation
- Migrated from custom Next.js setup to comprehensive documentation site

### Fixed
- Installation script curl redirect handling (added -L flag)
- Shell compatibility issues with echo commands (replaced with printf)
- Terminal input color support for dark mode
- Light mode contrast issues across UI components

## [0.1.5] - 2024-08-26

### Added
- Debug build port configuration (18765 for debug, 8765 for release)
- Enhanced terminal input styling with theme-aware colors
- Comprehensive documentation migration to Fumadocs
- Professional installation scripts for macOS/Linux and Windows

### Changed
- Updated command syntax documentation from outdated formats
- Improved visual design and contrast for light mode

### Fixed
- Removed outdated references to non-existent --debug flag
- Corrected command examples throughout documentation

## [0.1.4] - 2024-08-20

### Added
- Initial release with core functionality
- Terminal multiplexer for AI coding assistants
- Web UI support with VT100 parsing
- Session management and project organization

### Fixed
- Initial bug fixes and stability improvements