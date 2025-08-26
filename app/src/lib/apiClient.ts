import type {
	GitDiff,
	GitFileDiff,
	GitStatus,
	ProjectInfo,
	Session,
} from "../types/api";

// API configuration
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

const BASE_URL = getBaseUrl();

// Debug logging for development
if (typeof window !== "undefined" && window.location.port === "8081") {
	console.log(
		`🔧 API Client configured for development - Backend: ${BASE_URL}`,
	);
} else {
	console.log(`🚀 API Client configured for production - Backend: ${BASE_URL}`);
}

// Custom error class for API errors
export class ApiClientError extends Error {
	constructor(
		message: string,
		public status?: number,
		public statusText?: string,
		public url?: string,
	) {
		super(message);
		this.name = "ApiClientError";
	}
}

// Generic API client with type safety
class ApiClient {
	private baseUrl: string;

	constructor(baseUrl: string) {
		this.baseUrl = baseUrl;
	}

	private async request<T>(
		endpoint: string,
		options: RequestInit = {},
	): Promise<T> {
		const url = `${this.baseUrl}${endpoint}`;

		// Debug logging to verify URL
		console.log(`🌐 API Request: ${options.method || "GET"} ${url}`);

		const config: RequestInit = {
			headers: {
				"Content-Type": "application/json",
				...options.headers,
			},
			...options,
		};

		try {
			const response = await fetch(url, config);

			if (!response.ok) {
				const _errorText = await response.text().catch(() => "Unknown error");
				throw new ApiClientError(
					`API request failed: ${response.status} ${response.statusText}`,
					response.status,
					response.statusText,
					url,
				);
			}

			// Handle empty responses
			const contentType = response.headers.get("content-type");
			if (!contentType?.includes("application/json")) {
				return {} as T;
			}

			const data = await response.json();
			return data;
		} catch (error) {
			if (error instanceof ApiClientError) {
				throw error;
			}

			// Network or other errors
			throw new ApiClientError(
				`Network error: ${error instanceof Error ? error.message : "Unknown error"}`,
				undefined,
				undefined,
				url,
			);
		}
	}

	// GET request
	async get<T>(endpoint: string): Promise<T> {
		return this.request<T>(endpoint, { method: "GET" });
	}

	// POST request
	async post<T>(endpoint: string, data?: unknown): Promise<T> {
		return this.request<T>(endpoint, {
			method: "POST",
			body: data ? JSON.stringify(data) : undefined,
		});
	}

	// PUT request
	async put<T>(endpoint: string, data?: unknown): Promise<T> {
		return this.request<T>(endpoint, {
			method: "PUT",
			body: data ? JSON.stringify(data) : undefined,
		});
	}

	// DELETE request
	async delete<T>(endpoint: string): Promise<T> {
		return this.request<T>(endpoint, { method: "DELETE" });
	}
}

// Create API client instance
const apiClient = new ApiClient(BASE_URL);

// API endpoints with type safety
export const api = {
	// Sessions
	sessions: {
		list: (): Promise<Session[]> => apiClient.get("/api/sessions"),
		get: (id: string): Promise<Session> => apiClient.get(`/api/sessions/${id}`),
		create: (data: Partial<Session>): Promise<Session> =>
			apiClient.post("/api/sessions", data),
		delete: (id: string): Promise<void> =>
			apiClient.delete(`/api/sessions/${id}`),
	},

	// Projects
	projects: {
		list: (): Promise<ProjectInfo[]> => apiClient.get("/api/projects"),
		get: (id: string): Promise<ProjectInfo> =>
			apiClient.get(`/api/projects/${id}`),
		create: (data: Partial<ProjectInfo>): Promise<ProjectInfo> =>
			apiClient.post("/api/projects", data),
		delete: (id: string): Promise<void> =>
			apiClient.delete(`/api/projects/${id}`),
	},

	// Git operations
	git: {
		status: (sessionId: string): Promise<GitStatus> =>
			apiClient.get(`/api/sessions/${sessionId}/git/status`),
		diff: (sessionId: string): Promise<GitDiff> =>
			apiClient.get(`/api/sessions/${sessionId}/git/diff`),
		fileDiff: (sessionId: string, filePath: string): Promise<GitFileDiff> =>
			apiClient.get(
				`/api/sessions/${sessionId}/git/diff/${encodeURIComponent(filePath)}`,
			),
	},
} as const;

// Helper function to handle API errors in components
export const handleApiError = (error: unknown): string => {
	if (error instanceof ApiClientError) {
		if (error.status === 404) {
			return "Resource not found";
		}
		if (error.status === 500) {
			return "Server error. Please try again later.";
		}
		if (error.status && error.status >= 400 && error.status < 500) {
			return "Client error. Please check your request.";
		}
		return error.message;
	}

	if (error instanceof Error) {
		return error.message;
	}

	return "An unexpected error occurred";
};

// Utility to check if error is a network error
export const isNetworkError = (error: unknown): boolean => {
	return error instanceof ApiClientError && !error.status;
};

export default apiClient;
