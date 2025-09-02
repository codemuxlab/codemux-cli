import { useQuery } from "@tanstack/react-query";
import { api } from "../../lib/apiClient";
import { queryKeys, refetchQueries } from "../../lib/queryClient";
import type { UseGitDiffOptions, UseGitStatusOptions } from "../../types/api";

// Hook to fetch git status for a session
export const useGitStatus = (options: UseGitStatusOptions) => {
	const { sessionId, refetchInterval = 2000, enabled = true } = options;

	return useQuery({
		queryKey: queryKeys.git.status(sessionId),
		queryFn: () => api.git.status(sessionId),
		enabled: enabled && !!sessionId && sessionId.trim() !== "",
		refetchInterval: enabled ? refetchInterval : false,
		// Git status changes frequently, so keep it fresh
		staleTime: 1000,
		// Retry on network errors since git operations can be flaky
		retry: (failureCount, error) => {
			// Don't retry on 404 errors (session not found)
			if (
				error &&
				typeof error === "object" &&
				"status" in error &&
				(error as { status: number }).status === 404
			) {
				return false;
			}
			return failureCount < 3;
		},
		meta: {
			errorMessage: `Failed to fetch git status for session ${sessionId}`,
		},
	});
};

// Hook to fetch git diff for a session
export const useGitDiff = (options: UseGitDiffOptions) => {
	const { sessionId, refetchInterval = 5000, enabled = true } = options;

	return useQuery({
		queryKey: queryKeys.git.diff(sessionId),
		queryFn: () => api.git.diff(sessionId),
		enabled: enabled && !!sessionId && sessionId.trim() !== "",
		refetchInterval: enabled ? refetchInterval : false,
		// Diff can be expensive to compute, so cache it a bit longer
		staleTime: 2000,
		retry: (failureCount, error) => {
			// Don't retry on 404 errors (session not found)
			if (
				error &&
				typeof error === "object" &&
				"status" in error &&
				(error as { status: number }).status === 404
			) {
				return false;
			}
			return failureCount < 2;
		},
		meta: {
			errorMessage: `Failed to fetch git diff for session ${sessionId}`,
		},
	});
};

// Hook to fetch diff for a specific file
export const useGitFileDiff = (
	sessionId: string,
	filePath: string,
	enabled = true,
) => {
	return useQuery({
		queryKey: queryKeys.git.fileDiff(sessionId, filePath),
		queryFn: () => api.git.fileDiff(sessionId, filePath),
		enabled:
			enabled &&
			!!sessionId &&
			!!filePath &&
			sessionId.trim() !== "" &&
			filePath.trim() !== "",
		// File diffs are usually viewed once and don't change as often
		staleTime: 10000,
		retry: (failureCount, error) => {
			// Don't retry on 404 errors
			if (
				error &&
				typeof error === "object" &&
				"status" in error &&
				(error as { status: number }).status === 404
			) {
				return false;
			}
			return failureCount < 2;
		},
		meta: {
			errorMessage: `Failed to fetch diff for file ${filePath}`,
		},
	});
};

// Combined hook for git status and diff with intelligent loading
export const useGitData = (sessionId: string, enabled = true) => {
	const statusQuery = useGitStatus({ sessionId, enabled });

	// Only fetch diff if status shows there are changes
	const hasDirtyFiles = statusQuery.data && !statusQuery.data.clean;
	const diffQuery = useGitDiff({
		sessionId,
		enabled: enabled && hasDirtyFiles,
		refetchInterval: hasDirtyFiles ? 5000 : undefined,
	});

	return {
		status: statusQuery,
		diff: diffQuery,
		// Combined loading state
		isLoading: statusQuery.isPending || (hasDirtyFiles && diffQuery.isPending),
		// Combined error state
		error: statusQuery.error || diffQuery.error,
		// Whether any data is available
		hasData: !!statusQuery.data,
		// Whether there are changes to show
		hasChanges: hasDirtyFiles,
		// Refetch both queries
		refetchAll: () => {
			refetchQueries.gitStatus(sessionId);
			if (hasDirtyFiles) {
				refetchQueries.gitDiff(sessionId);
			}
		},
	};
};

// Hook for manual refresh of git data
export const useRefreshGit = (sessionId: string) => {
	return {
		refreshStatus: () => refetchQueries.gitStatus(sessionId),
		refreshDiff: () => refetchQueries.gitDiff(sessionId),
		refreshAll: () => {
			refetchQueries.gitStatus(sessionId);
			refetchQueries.gitDiff(sessionId);
		},
	};
};

// Hook to get git file changes summary
export const useGitSummary = (sessionId: string, enabled = true) => {
	const { data: gitDiff } = useGitDiff({ sessionId, enabled });

	if (!gitDiff) {
		return null;
	}

	const summary = {
		totalFiles: gitDiff.files.length,
		totalAdditions: gitDiff.files.reduce(
			(sum, file) => sum + file.additions,
			0,
		),
		totalDeletions: gitDiff.files.reduce(
			(sum, file) => sum + file.deletions,
			0,
		),
		filesByStatus: gitDiff.files.reduce(
			(acc, file) => {
				acc[file.status] = (acc[file.status] || 0) + 1;
				return acc;
			},
			{} as Record<string, number>,
		),
	};

	return summary;
};

// Hook to check if session has git changes
export const useHasGitChanges = (sessionId: string) => {
	const { data: gitStatus } = useGitStatus({
		sessionId,
		refetchInterval: 3000,
	});
	return gitStatus ? !gitStatus.clean : false;
};
