import { QueryClient } from "@tanstack/react-query";

// Create a client instance with optimized defaults
export const queryClient = new QueryClient({
	defaultOptions: {
		queries: {
			// Data is considered fresh for 30 seconds
			staleTime: 30 * 1000,
			// Cache data for 5 minutes
			gcTime: 5 * 60 * 1000,
			// Retry failed requests up to 2 times
			retry: (failureCount, error) => {
				// Don't retry on 4xx errors (client errors)
				if (error instanceof Error && "status" in error) {
					const status = (error as { status: number }).status;
					if (status >= 400 && status < 500) {
						return false;
					}
				}
				return failureCount < 2;
			},
			// Retry delay with exponential backoff
			retryDelay: (attemptIndex) => Math.min(1000 * 2 ** attemptIndex, 30000),
			// Refetch on window focus for critical data
			refetchOnWindowFocus: false,
			// Don't refetch on reconnect by default
			refetchOnReconnect: "always",
		},
		mutations: {
			// Retry mutations once on network errors
			retry: (failureCount, error) => {
				if (error instanceof Error && error.message.includes("NetworkError")) {
					return failureCount < 1;
				}
				return false;
			},
		},
	},
});

// Query keys factory for consistent key management
export const queryKeys = {
	all: ["api"] as const,
	sessions: () => [...queryKeys.all, "sessions"] as const,
	session: (id: string) => [...queryKeys.sessions(), id] as const,
	projects: () => [...queryKeys.all, "projects"] as const,
	project: (id: string) => [...queryKeys.projects(), id] as const,
	git: {
		all: () => [...queryKeys.all, "git"] as const,
		status: (sessionId: string) =>
			[...queryKeys.git.all(), "status", sessionId] as const,
		diff: (sessionId: string) =>
			[...queryKeys.git.all(), "diff", sessionId] as const,
		fileDiff: (sessionId: string, filePath: string) =>
			[...queryKeys.git.diff(sessionId), filePath] as const,
	},
} as const;

// Utility functions for cache management
export const invalidateQueries = {
	sessions: () =>
		queryClient.invalidateQueries({ queryKey: queryKeys.sessions() }),
	projects: () =>
		queryClient.invalidateQueries({ queryKey: queryKeys.projects() }),
	gitStatus: (sessionId: string) =>
		queryClient.invalidateQueries({
			queryKey: queryKeys.git.status(sessionId),
		}),
	gitDiff: (sessionId: string) =>
		queryClient.invalidateQueries({ queryKey: queryKeys.git.diff(sessionId) }),
	allGit: (sessionId: string) =>
		queryClient.invalidateQueries({
			queryKey: [...queryKeys.git.all(), sessionId],
		}),
};

// Background refetch for real-time data
export const refetchQueries = {
	gitStatus: (sessionId: string) =>
		queryClient.refetchQueries({ queryKey: queryKeys.git.status(sessionId) }),
	gitDiff: (sessionId: string) =>
		queryClient.refetchQueries({ queryKey: queryKeys.git.diff(sessionId) }),
};
