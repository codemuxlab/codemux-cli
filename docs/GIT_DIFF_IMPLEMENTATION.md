# Git Diff Viewer Implementation

## Overview

Successfully implemented a comprehensive Git diff viewer for the codemux web interface, featuring a GitHub-style diff display with real-time updates and tabbed navigation.

## Features Implemented

### 🔧 Backend API (src/web.rs)

#### New Endpoints
- `GET /api/sessions/{session_id}/git/status` - Returns git status with file changes
- `GET /api/sessions/{session_id}/git/diff` - Returns full diff with all changed files
- `GET /api/sessions/{session_id}/git/diff/{file_path}` - Returns diff for specific file

#### Data Structures
```rust
struct GitStatus {
    files: Vec<GitFileStatus>,
    branch: Option<String>,
    clean: bool,
}

struct GitFileDiff {
    path: String,
    old_path: Option<String>, // For renamed files
    status: String,
    additions: u32,
    deletions: u32,
    diff: String,
}
```

#### Git Integration
- Executes native git commands in session working directory
- Parses `git status --porcelain` output
- Processes `git diff` with proper addition/deletion counting
- Handles file status detection (modified, added, deleted, renamed, untracked)

### 🎨 Frontend Components (app/src/components/)

#### GitDiffViewer.tsx
- Main diff viewer component with file list sidebar
- Real-time auto-refresh every 2 seconds
- Branch and change statistics display
- File selection with visual indicators
- Error handling and loading states

#### DiffLine.tsx
- Individual diff line rendering with syntax highlighting
- Proper color coding for additions (+), deletions (-), and context
- Line number display
- Header parsing for `@@` context lines

#### FileDiffHeader.tsx
- File information header with status icons
- Addition/deletion statistics with visual bar
- File path display with renamed file handling
- Refresh action button

### 🔄 Expo Router Structure

#### New Route Structure
```
/session/{sessionId}/
├── terminal (🖥️)
├── diff (📋) 
└── logs (📜)
```

#### File Organization
- `app/session/[sessionId]/(tabs)/_layout.tsx` - Tab navigation layout
- `app/session/[sessionId]/(tabs)/terminal.tsx` - Terminal tab
- `app/session/[sessionId]/(tabs)/diff.tsx` - Git diff tab
- `app/session/[sessionId]/(tabs)/logs.tsx` - Logs tab (placeholder)

### 🎯 Visual Features

#### GitHub-Style Diff Display
- Green highlighting for additions (+)
- Red highlighting for deletions (-)
- Gray highlighting for file headers and context
- File status icons (✏️ modified, ➕ added, ❌ deleted, etc.)

#### Interactive Elements
- Clickable file list with selection highlighting
- Scrollable diff content with proper typography
- Refresh buttons for manual updates
- Status indicators with colors

## Usage

### Starting the Application
```bash
just build    # Build with React app
just run-dev  # Start development server
```

### Accessing Git Diff
1. Navigate to any session URL: `http://localhost:8765/session/{sessionId}`
2. Click the "Changes" tab (📋) to view git diff
3. Select files from the sidebar to view individual diffs

### API Testing
```bash
# Get git status for a session
curl http://localhost:8765/api/sessions/test-session/git/status

# Get full diff
curl http://localhost:8765/api/sessions/test-session/git/diff

# Get diff for specific file
curl http://localhost:8765/api/sessions/test-session/git/diff/src/main.rs
```

## Technical Architecture

### Session Context
- Git operations run in the session's working directory
- For run mode: Uses current directory
- For daemon mode: Will use session-specific project directory (TODO)

### Real-time Updates
- Frontend polls git status/diff every 2 seconds
- Updates automatically when files change during AI agent work
- Efficient diff parsing with addition/deletion counting

### Error Handling
- Graceful handling of non-git repositories
- Network error recovery with retry mechanisms
- Loading states during API calls

## Future Enhancements

### Phase 2 Features (Not Yet Implemented)
- Git tree view with hierarchical file structure
- Commit staging functionality
- Multi-session workspace layouts
- Syntax highlighting for code diffs
- Side-by-side diff view toggle
- Git blame integration

### Daemon Mode Integration
- Project-specific working directories
- Session-to-project mapping
- Cross-session git status aggregation

## Files Modified/Created

### Backend
- `src/web.rs` - Added git API endpoints and handlers

### Frontend
- `app/src/components/GitDiffViewer.tsx` - Main diff viewer
- `app/src/components/DiffLine.tsx` - Individual diff line component  
- `app/src/components/FileDiffHeader.tsx` - File header component
- `app/src/app/session/[sessionId]/(tabs)/_layout.tsx` - Tab layout
- `app/src/app/session/[sessionId]/(tabs)/diff.tsx` - Diff tab route
- `app/src/app/session/[sessionId]/(tabs)/terminal.tsx` - Terminal tab route
- `app/src/app/session/[sessionId]/(tabs)/logs.tsx` - Logs tab route
- `app/src/app/session/[sessionId]/index.tsx` - Session redirect
- `app/src/app/index.tsx` - Updated main index

## Status: ✅ Complete

All planned Phase 1 features have been successfully implemented and tested. The git diff viewer is now ready for use with AI coding sessions.