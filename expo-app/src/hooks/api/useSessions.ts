import { useMutation, useQuery } from "@tanstack/react-query";
import { api, handleApiError } from "../../lib/apiClient";
import {
	invalidateQueries,
	queryClient,
	queryKeys,
} from "../../lib/queryClient";
import type {
	CreateSessionRequest,
	Session,
	UseSessionsOptions,
} from "../../types/api";

// Hook to fetch all sessions
export const useSessions = (options: UseSessionsOptions = {}) => {
	const { refetchInterval = 5000, enabled = true } = options;

	return useQuery({
		queryKey: queryKeys.sessions(),
		queryFn: async () => {
			// Get sessions directly from project relationships to avoid deep API calls
			const projects = await api.projects.list();
			const allSessions: Session[] = [];

			for (const project of projects) {
				if (project.relationships?.recent_sessions) {
					// Use session resources from relationships directly
					allSessions.push(...project.relationships.recent_sessions);
				}
			}

			return allSessions;
		},
		refetchInterval: enabled ? refetchInterval : false,
		enabled,
		meta: {
			errorMessage: "Failed to fetch sessions",
		},
	});
};

// Hook to fetch a single session
export const useSession = (sessionId: string, enabled = true) => {
	return useQuery({
		queryKey: queryKeys.session(sessionId),
		queryFn: () => api.sessions.get(sessionId),
		enabled: enabled && !!sessionId,
		meta: {
			errorMessage: `Failed to fetch session ${sessionId}`,
		},
	});
};

// Hook to create a new session
export const useCreateSession = () => {
	return useMutation({
		mutationFn: (sessionData: CreateSessionRequest) =>
			api.sessions.create(sessionData),
		onSuccess: (newSession) => {
			// Invalidate sessions list to include the new session
			invalidateQueries.sessions();

			// Add the new session to the cache
			queryClient.setQueryData(queryKeys.session(newSession.id), newSession);
		},
		onError: (error) => {
			console.error("Failed to create session:", handleApiError(error));
		},
		meta: {
			errorMessage: "Failed to create session",
		},
	});
};

// Hook to delete a session
export const useDeleteSession = () => {
	return useMutation({
		mutationFn: (sessionId: string) => api.sessions.delete(sessionId),
		onSuccess: (_, sessionId) => {
			// Remove session from cache
			queryClient.removeQueries({ queryKey: queryKeys.session(sessionId) });

			// Invalidate sessions list
			invalidateQueries.sessions();

			// Also invalidate any git-related queries for this session
			invalidateQueries.allGit(sessionId);
		},
		onError: (error) => {
			console.error("Failed to delete session:", handleApiError(error));
		},
		meta: {
			errorMessage: "Failed to delete session",
		},
	});
};

// Hook to refetch sessions manually
export const useRefetchSessions = () => {
	return () => {
		queryClient.refetchQueries({ queryKey: queryKeys.sessions() });
	};
};

// Hook to get session count without subscribing to changes
export const useSessionsCount = () => {
	const sessionsData = queryClient.getQueryData<Session[]>(
		queryKeys.sessions(),
	);
	return sessionsData?.length ?? 0;
};

// Hook to check if a session exists in cache
export const useSessionExists = (sessionId: string) => {
	const sessionData = queryClient.getQueryData<Session>(
		queryKeys.session(sessionId),
	);
	return !!sessionData;
};
