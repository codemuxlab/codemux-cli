# HTTP Client with TanStack Query v5

This document describes the HTTP client implementation using TanStack Query v5 for state management, caching, and API interactions.

## Overview

The HTTP client provides a type-safe, centralized way to interact with the Codemux backend API. It includes automatic caching, background refetching, error handling, and TypeScript support.

## Architecture

### Core Components

1. **QueryClient** (`src/lib/queryClient.ts`) - TanStack Query configuration
2. **API Client** (`src/lib/apiClient.ts`) - HTTP wrapper with error handling  
3. **Types** (`src/types/api.ts`) - TypeScript interfaces for all API responses
4. **Hooks** (`src/hooks/api/`) - Custom React hooks for data fetching
5. **Provider** (`src/app/_layout.tsx`) - QueryClient provider setup

## QueryClient Configuration

### Optimized Defaults
```typescript
export const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      staleTime: 30 * 1000,        // Fresh for 30 seconds
      gcTime: 5 * 60 * 1000,       // Cache for 5 minutes
      retry: 2,                     // Retry failed requests
      refetchOnWindowFocus: false,  // Don't refetch on focus
      refetchOnReconnect: 'always', // Refetch on reconnect
    },
  },
});
```

### Query Keys Factory
```typescript
export const queryKeys = {
  sessions: () => ['api', 'sessions'] as const,
  session: (id: string) => [...queryKeys.sessions(), id] as const,
  projects: () => ['api', 'projects'] as const,
  git: {
    status: (sessionId: string) => ['api', 'git', 'status', sessionId] as const,
    diff: (sessionId: string) => ['api', 'git', 'diff', sessionId] as const,
  },
} as const;
```

## API Client Layer

### Type-Safe HTTP Wrapper
```typescript
class ApiClient {
  async get<T>(endpoint: string): Promise<T>
  async post<T>(endpoint: string, data?: unknown): Promise<T>
  async put<T>(endpoint: string, data?: unknown): Promise<T>
  async delete<T>(endpoint: string): Promise<T>
}
```

### Error Handling
```typescript
export class ApiClientError extends Error {
  constructor(
    message: string,
    public status?: number,
    public statusText?: string,
    public url?: string
  ) {
    super(message);
  }
}
```

### API Endpoints
```typescript
export const api = {
  sessions: {
    list: (): Promise<Session[]>
    get: (id: string): Promise<Session>
    create: (data: Partial<Session>): Promise<Session>
    delete: (id: string): Promise<void>
  },
  projects: { /* similar structure */ },
  git: {
    status: (sessionId: string): Promise<GitStatus>
    diff: (sessionId: string): Promise<GitDiff>
    fileDiff: (sessionId: string, filePath: string): Promise<GitFileDiff>
  },
}
```

## Custom Hooks

### Sessions
```typescript
// List all sessions with auto-refresh
const { data, isLoading, error } = useSessions({ 
  refetchInterval: 5000 
});

// Get single session
const { data: session } = useSession(sessionId);

// Create new session
const { mutate: createSession } = useCreateSession();

// Delete session
const { mutate: deleteSession } = useDeleteSession();
```

### Projects
```typescript
// List all projects
const { data: projects } = useProjects();

// Create new project
const { mutate: createProject } = useCreateProject();

// Find project by path
const { data: project } = useProjectByPath('/path/to/project');
```

### Git Operations
```typescript
// Get git status with auto-refresh
const { data: gitStatus } = useGitStatus({ 
  sessionId,
  refetchInterval: 2000 
});

// Get git diff
const { data: gitDiff } = useGitDiff({ sessionId });

// Combined hook for intelligent loading
const { status, diff, isLoading, error } = useGitData(sessionId);

// Manual refresh
const { refreshAll } = useRefreshGit(sessionId);
```

## TypeScript Types

### Core Types
```typescript
interface Session {
  id: string;
  agent: string;
  status: 'active' | 'inactive' | 'completed' | 'error';
  created?: string;
  project_id?: string;
}

interface GitStatus {
  files: GitFileStatus[];
  branch?: string;
  clean: boolean;
}

interface GitDiff {
  files: GitFileDiff[];
  summary?: {
    total_files: number;
    total_additions: number;
    total_deletions: number;
  };
}
```

### Hook Options
```typescript
interface UseSessionsOptions {
  refetchInterval?: number;
  enabled?: boolean;
}

interface UseGitStatusOptions {
  sessionId: string;
  refetchInterval?: number;
  enabled?: boolean;
}
```

## Usage Examples

### Basic Data Fetching
```typescript
function SessionsList() {
  const { data: sessions, isLoading, error } = useSessions();

  if (isLoading) return <LoadingSpinner />;
  if (error) return <ErrorMessage error={error} />;

  return (
    <div>
      {sessions?.map(session => (
        <SessionCard key={session.id} session={session} />
      ))}
    </div>
  );
}
```

### Creating Resources
```typescript
function CreateSessionButton() {
  const { mutate: createSession, isPending } = useCreateSession();

  const handleCreate = () => {
    createSession({ 
      agent: 'claude',
      project_path: '/current/project' 
    });
  };

  return (
    <button onClick={handleCreate} disabled={isPending}>
      {isPending ? 'Creating...' : 'New Session'}
    </button>
  );
}
```

### Real-time Git Status
```typescript
function GitStatusView({ sessionId }: { sessionId: string }) {
  const { data: gitStatus, isLoading } = useGitStatus({
    sessionId,
    refetchInterval: 2000, // Poll every 2 seconds
  });

  return (
    <div>
      <h3>Git Status</h3>
      {gitStatus?.clean ? (
        <p>✅ Working tree clean</p>
      ) : (
        <ul>
          {gitStatus?.files.map(file => (
            <li key={file.path}>
              {getStatusIcon(file.status)} {file.path}
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
```

### Error Handling
```typescript
function SessionView({ sessionId }: { sessionId: string }) {
  const { data: session, error, refetch } = useSession(sessionId);

  if (error) {
    return (
      <div>
        <p>Error: {handleApiError(error)}</p>
        <button onClick={() => refetch()}>Retry</button>
      </div>
    );
  }

  return <SessionDetails session={session} />;
}
```

## Cache Management

### Manual Invalidation
```typescript
import { invalidateQueries } from '@/hooks/api';

// Invalidate specific queries
invalidateQueries.sessions();
invalidateQueries.gitStatus(sessionId);
invalidateQueries.allGit(sessionId);
```

### Optimistic Updates
```typescript
const { mutate: deleteSession } = useDeleteSession();

// Automatically removes from cache and invalidates queries
deleteSession(sessionId);
```

### Background Refetching
```typescript
import { refetchQueries } from '@/hooks/api';

// Manual background refetch
const handleRefresh = () => {
  refetchQueries.gitStatus(sessionId);
  refetchQueries.gitDiff(sessionId);
};
```

## Performance Features

### Automatic Benefits
- **Request Deduplication**: Multiple components using same query share single request
- **Background Updates**: Stale data updates in background while showing cached version
- **Retry Logic**: Failed requests automatically retry with exponential backoff
- **Memory Management**: Unused queries garbage collected after 5 minutes

### Intelligent Loading States
```typescript
const { status, diff, isLoading, error, hasChanges } = useGitData(sessionId);

// Only fetches diff if git status shows changes
// Provides combined loading state for better UX
```

## Best Practices

### 1. Use Appropriate Stale Times
- **Frequently changing data** (git status): 1-2 seconds
- **Moderately changing data** (sessions): 30 seconds  
- **Rarely changing data** (projects): 60+ seconds

### 2. Handle Loading and Error States
```typescript
if (isLoading) return <Skeleton />;
if (error) return <ErrorBoundary error={error} />;
return <DataComponent data={data} />;
```

### 3. Use Query Keys Consistently
```typescript
// Good
const { data } = useQuery({
  queryKey: queryKeys.session(sessionId),
  queryFn: () => api.sessions.get(sessionId),
});

// Avoid
const { data } = useQuery({
  queryKey: ['session', sessionId], // Inconsistent with factory
  queryFn: () => api.sessions.get(sessionId),
});
```

### 4. Leverage Optimistic Updates
```typescript
const { mutate } = useMutation({
  mutationFn: updateSession,
  onMutate: async (newData) => {
    // Cancel outgoing refetches
    await queryClient.cancelQueries({ queryKey: queryKeys.session(id) });
    
    // Snapshot previous value
    const previous = queryClient.getQueryData(queryKeys.session(id));
    
    // Optimistically update
    queryClient.setQueryData(queryKeys.session(id), newData);
    
    return { previous };
  },
  onError: (err, newData, context) => {
    // Rollback on error
    queryClient.setQueryData(queryKeys.session(id), context.previous);
  },
});
```

## Troubleshooting

### Common Issues

**Stale Data**: Check `staleTime` and `refetchInterval` settings
**Memory Leaks**: Ensure proper component unmounting and query cleanup  
**Network Errors**: Verify backend connectivity and CORS configuration
**Type Errors**: Update API types to match backend response format

### Debug Tools

- React Query DevTools (development only)
- Browser Network tab for HTTP requests
- Console logs for API client errors
- QueryClient inspection: `queryClient.getQueryData(key)`