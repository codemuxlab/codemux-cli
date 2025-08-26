# API Configuration

This document describes the API client configuration for connecting to the Codemux backend.

## Port Configuration

The Codemux application uses different ports for different services:

### Backend Services (Quick Mode)
- **HTTP API & WebSocket**: `http://localhost:8765` - Both REST API endpoints and WebSocket connections
  - REST API endpoints for sessions, projects, and git operations
  - Real-time terminal WebSocket connections

### Frontend Development
- **Expo Dev Server**: `http://localhost:8081` - React Native Web development server

## API Client Configuration

The API client (`src/lib/apiClient.ts`) automatically detects the environment and uses the appropriate backend URL:

### Development Mode
When running `npm start` (Expo dev server on port 8081):
- API requests → `http://localhost:8765`
- WebSocket connections → `ws://localhost:8765`

### Production Mode  
When served by the backend (port 8765):
- API requests → Same host on port 8765
- WebSocket connections → Same host on port 8765

## Configuration Logic

```typescript
const getBaseUrl = (): string => {
  // Use port 8765 which is the default port for quick mode
  // Check if we're running on the Expo dev server (port 8081)
  if (typeof window !== "undefined" && window.location.port === "8081") {
    // Development: Expo dev server on 8081, backend on 8765
    return "http://localhost:8765";
  }
  // In production or when served by backend, use the current host with port 8765
  if (typeof window !== "undefined") {
    return `${window.location.protocol}//${window.location.hostname}:8765`;
  }
  // Fallback for SSR or other environments
  return "http://localhost:8765";
};
```

## API Endpoints

The following REST API endpoints are available:

### Sessions
- `GET /api/sessions` - List all sessions
- `GET /api/sessions/{id}` - Get session details
- `POST /api/sessions` - Create new session
- `DELETE /api/sessions/{id}` - Delete session

### Projects  
- `GET /api/projects` - List all projects
- `GET /api/projects/{id}` - Get project details
- `POST /api/projects` - Create new project
- `DELETE /api/projects/{id}` - Delete project

### Git Operations
- `GET /api/sessions/{id}/git/status` - Get git status for session
- `GET /api/sessions/{id}/git/diff` - Get git diff for session
- `GET /api/sessions/{id}/git/diff/{file}` - Get diff for specific file

## WebSocket Connection

Terminal connections use WebSocket at:
- Development: `ws://localhost:8765/ws/{sessionId}`
- Production: `ws://{hostname}:8765/ws/{sessionId}`

## Testing the Configuration

### Development
1. Start the backend: `cargo run` (defaults to port 8765)
2. Start the frontend: `npm start` (runs on port 8081)
3. Verify API calls go to port 8765
4. Verify WebSocket connects to port 8765

### Production
1. Build the app: `npm run build`
2. Serve via backend on port 8765
3. Verify all connections use port 8765

## Troubleshooting

### Common Issues

**CORS Errors**: Ensure the backend allows requests from `http://localhost:8081` in development.

**Connection Refused**: Verify the backend is running and listening on port 8765.

**WebSocket Failures**: Check that port 8765 is not blocked by firewalls and is serving both HTTP and WebSocket.

### Debug Information

The API client logs errors with detailed information including:
- Request URL
- HTTP status codes  
- Error messages
- Network connectivity issues

Check the browser console for API client debug information.