import { useMutation, useQuery } from "@tanstack/react-query";
import { api, handleApiError } from "../../lib/apiClient";
import {
	invalidateQueries,
	queryClient,
	queryKeys,
} from "../../lib/queryClient";
import type { CreateProjectRequest, Project } from "../../types/api";

// Hook to fetch all projects
export const useProjects = (enabled = true) => {
	return useQuery({
		queryKey: queryKeys.projects(),
		queryFn: () => api.projects.list(),
		enabled,
		staleTime: 60 * 1000, // Projects don't change as frequently as sessions
		meta: {
			errorMessage: "Failed to fetch projects",
		},
	});
};

// Hook to fetch a single project
export const useProject = (projectId: string, enabled = true) => {
	return useQuery({
		queryKey: queryKeys.project(projectId),
		queryFn: () => api.projects.get(projectId),
		enabled: enabled && !!projectId,
		staleTime: 60 * 1000,
		meta: {
			errorMessage: `Failed to fetch project ${projectId}`,
		},
	});
};

// Hook to create a new project
export const useCreateProject = () => {
	return useMutation({
		mutationFn: (projectData: CreateProjectRequest) =>
			api.projects.create(projectData),
		onSuccess: (newProject) => {
			// Invalidate projects list to include the new project
			invalidateQueries.projects();

			// Add the new project to the cache
			queryClient.setQueryData(queryKeys.project(newProject.id), newProject);
		},
		onError: (error) => {
			console.error("Failed to create project:", handleApiError(error));
		},
		meta: {
			errorMessage: "Failed to create project",
		},
	});
};

// Hook to delete a project
export const useDeleteProject = () => {
	return useMutation({
		mutationFn: (projectId: string) => api.projects.delete(projectId),
		onSuccess: (_, projectId) => {
			// Remove project from cache
			queryClient.removeQueries({ queryKey: queryKeys.project(projectId) });

			// Invalidate projects list
			invalidateQueries.projects();

			// Also invalidate sessions as they might be affected
			invalidateQueries.sessions();
		},
		onError: (error) => {
			console.error("Failed to delete project:", handleApiError(error));
		},
		meta: {
			errorMessage: "Failed to delete project",
		},
	});
};

// Hook to refetch projects manually
export const useRefetchProjects = () => {
	return () => {
		queryClient.refetchQueries({ queryKey: queryKeys.projects() });
	};
};

// Hook to get project count without subscribing to changes
export const useProjectsCount = () => {
	const projectsData = queryClient.getQueryData<Project[]>(
		queryKeys.projects(),
	);
	return projectsData?.length ?? 0;
};

// Hook to find project by path
export const useProjectByPath = (path: string) => {
	return useQuery({
		queryKey: [...queryKeys.projects(), "by-path", path],
		queryFn: async () => {
			const projects = await api.projects.list();
			return (
				projects.find((project) => project.attributes?.path === path) || null
			);
		},
		enabled: !!path,
		staleTime: 60 * 1000,
		meta: {
			errorMessage: `Failed to find project for path ${path}`,
		},
	});
};
