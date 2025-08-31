import type {
  CreateSessionRequest,
  GitDiff,
  GitFileDiff,
  GitStatus,
  Project,
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
      if (
        !contentType?.includes("application/json") &&
        !contentType?.includes("application/vnd.api+json")
      ) {
        return {} as T;
      }

      let data: unknown;
      try {
        data = await response.json();
      } catch (_parseError) {
        throw new ApiClientError(
          "Failed to parse JSON response",
          response.status,
          response.statusText,
          url,
        );
      }

      // Check if response is JSON API format
      if (data && typeof data === "object" && "data" in data) {
        const jsonApiData = data as {
          data: unknown;
          errors?: Array<{ status?: string; title?: string; detail?: string }>;
        };

        // Handle JSON API errors
        if (jsonApiData.errors && jsonApiData.errors.length > 0) {
          const error = jsonApiData.errors[0];
          throw new ApiClientError(
            error.detail || error.title || "API error",
            Number(error.status) || response.status,
            error.title || response.statusText,
            url,
          );
        }

        const extractedData = jsonApiData.data;
        // Return the data field directly if it's not a resource structure
        return extractedData as T;
      }

      // Return raw data if not JSON API format (backwards compatibility)
      return data as T;
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
    create: (data: CreateSessionRequest): Promise<Session> =>
      apiClient.post("/api/sessions", data),
    delete: (id: string): Promise<void> =>
      apiClient.delete(`/api/sessions/${id}`),
  },

  // Projects
  projects: {
    list: (): Promise<Project[]> => apiClient.get("/api/projects"),
    get: (id: string): Promise<Project> => apiClient.get(`/api/projects/${id}`),
    create: (data: { name: string; path: string }): Promise<Project> =>
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
