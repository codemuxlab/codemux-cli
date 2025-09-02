# CodeMux Web Interface

The React Native Web frontend for CodeMux terminal multiplexer. Built with Expo Router and NativeWind (Tailwind CSS).

## Architecture

- **React Native Web**: Cross-platform UI framework
- **Expo Router**: File-based routing system
- **NativeWind v4**: Tailwind CSS integration for React Native
- **Zustand**: State management with granular subscriptions
- **TanStack Query**: API state management and caching
- **WebSocket**: Real-time terminal communication

## Development

### Prerequisites

- Node.js 18+
- Expo CLI

### Setup

```bash
# Install dependencies
npm install

# Start development server
npm start

# Or start web development server directly
npm run web
```

### Available Scripts

```bash
npm start          # Start Expo development server
npm run web        # Start web development server
npm run build      # Build for production
npm run lint       # Run Biome linter
npm run lint:fix   # Auto-fix linting issues
```

## Code Quality

**Important**: Always run linting before commits:

```bash
npm run lint       # Check for issues
npm run lint:fix   # Auto-fix where possible
```

The project uses **Biome** for strict TypeScript and React best practices:
- No unused imports or variables
- Proper TypeScript typing (avoid `any`)
- React best practices (exhaustive dependencies, key props)
- Modern JavaScript patterns

## Project Structure

```
app/
├── src/
│   ├── components/     # Reusable React components
│   │   ├── Terminal/   # Terminal emulation components
│   │   └── UI/         # Common UI components
│   ├── stores/         # Zustand state management
│   ├── hooks/          # Custom React hooks
│   ├── utils/          # Utility functions
│   └── types/          # TypeScript type definitions
├── app/                # Expo Router file-based routing
│   ├── (tabs)/         # Tab navigation routes
│   └── _layout.tsx     # Root layout
├── assets/             # Static assets (images, fonts)
└── tailwind.config.js  # Tailwind CSS configuration
```

## Key Features

### Terminal Emulation
- Grid-based terminal rendering with VT100 support
- Real-time WebSocket communication
- Optimized cell updates for performance
- Proper ANSI escape sequence handling

### State Management
- Zustand stores for terminal state, sessions, and projects
- TanStack Query for API caching and background updates
- Granular subscriptions to minimize re-renders

### UI Components
- Native web components for interactive prompts
- File/path pickers with proper validation
- Multi-select dropdowns and checkboxes
- Responsive layout with proper scaling

## WebSocket Communication

The app communicates with the Rust backend via WebSocket:

```typescript
// Terminal grid updates
{
  type: "grid_update",
  grid: GridCell[][],
  cursor: { row: number, col: number }
}

// User input
{
  type: "input", 
  data: string
}
```

## Development Notes

- Built specifically for CodeMux - not a generic terminal emulator
- Optimized for AI CLI interactions with enhanced prompt handling
- Web-first design that works across platforms via React Native Web
- Integrated with Rust backend for session and project management

For the main project development guide, see [../DEVELOPMENT.md](../DEVELOPMENT.md).
