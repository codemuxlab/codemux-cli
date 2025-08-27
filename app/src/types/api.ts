import type { GridCell } from "./bindings";

// Base types for API responses
export interface ApiResponse<T = unknown> {
	success: boolean;
	data: T;
	message?: string;
}

export interface ApiError {
	error: string;
	message?: string;
	status?: number;
}

// Session related types
export interface Session {
	id: string;
	agent: string;
	status: "running" | "active" | "inactive" | "completed" | "error";
	created?: string;
	updated?: string;
	project?: string; // Changed from project_id to project to match backend
	project_path?: string;
}

export interface CreateSessionRequest {
	agent?: string;
	project_id?: string;
	project_path?: string;
}

// Project related types
export interface ProjectInfo {
	id: string;
	name: string;
	path: string;
	created?: string;
	updated?: string;
	active_sessions?: number;
	sessions: Session[]; // Now includes sessions array
}

export interface CreateProjectRequest {
	name: string;
	path: string;
}

// Git status types
export interface GitFileStatus {
	path: string;
	status: "modified" | "added" | "deleted" | "renamed" | "untracked" | "staged";
	additions?: number;
	deletions?: number;
	old_path?: string; // For renamed files
}

export interface GitStatus {
	files: GitFileStatus[];
	branch?: string;
	clean: boolean;
	ahead?: number;
	behind?: number;
	staged_files?: GitFileStatus[];
	unstaged_files?: GitFileStatus[];
	untracked_files?: GitFileStatus[];
}

// Git diff types
export interface GitFileDiff {
	path: string;
	old_path?: string; // For renamed files
	status: "modified" | "added" | "deleted" | "renamed";
	additions: number;
	deletions: number;
	diff: string; // Raw diff content
	binary?: boolean;
	chunks?: DiffChunk[];
}

export interface DiffChunk {
	old_start: number;
	old_lines: number;
	new_start: number;
	new_lines: number;
	header: string;
	lines: DiffLine[];
}

export interface DiffLine {
	type: "context" | "addition" | "deletion" | "header" | "hunk";
	content: string;
	old_line?: number;
	new_line?: number;
}

export interface GitDiff {
	files: GitFileDiff[];
	summary?: {
		total_files: number;
		total_additions: number;
		total_deletions: number;
	};
}

// WebSocket message types for real-time updates
export interface WebSocketMessage {
	type: string;
	data: unknown;
	timestamp?: string;
}

// Use generated types for WebSocket communication
export type {
	ClientMessage,
	GridUpdateMessage,
	ServerMessage,
} from "./bindings";

export interface TerminalMessage extends WebSocketMessage {
	type: "terminal";
	data: {
		content: string;
		sessionId: string;
	};
}

export interface GitStatusMessage extends WebSocketMessage {
	type: "git_status";
	data: {
		sessionId: string;
		status: GitStatus;
	};
}

export interface LegacyGridUpdateMessage extends WebSocketMessage {
	type: "grid";
	data: {
		sessionId: string;
		grid: GridCell[][];
		cursor: CursorPosition;
	};
}

// Use the generated GridCell type from bindings
export type { GridCell } from "./bindings";

// Keep CursorPosition for backwards compatibility, but align with generated types
export interface CursorPosition {
	row: number;
	col: number;
	visible: boolean;
}

// Query result types for TanStack Query
export interface QueryResult<T> {
	data: T;
	isLoading: boolean;
	isPending: boolean;
	error: Error | null;
	isError: boolean;
	isSuccess: boolean;
}

// Mutation types for TanStack Query
export interface MutationResult<T, TVariables = unknown> {
	mutate: (variables: TVariables) => void;
	mutateAsync: (variables: TVariables) => Promise<T>;
	isPending: boolean;
	error: Error | null;
	isError: boolean;
	isSuccess: boolean;
	data?: T;
	reset: () => void;
}

// Hook options for customization
export interface UseSessionsOptions {
	refetchInterval?: number;
	enabled?: boolean;
}

export interface UseGitStatusOptions {
	sessionId: string;
	refetchInterval?: number;
	enabled?: boolean;
}

export interface UseGitDiffOptions {
	sessionId: string;
	refetchInterval?: number;
	enabled?: boolean;
}

// Error types for better error handling
export type ApiErrorType =
	| "NETWORK_ERROR"
	| "SERVER_ERROR"
	| "CLIENT_ERROR"
	| "NOT_FOUND"
	| "UNAUTHORIZED"
	| "FORBIDDEN"
	| "TIMEOUT"
	| "UNKNOWN";

export interface DetailedApiError {
	type: ApiErrorType;
	message: string;
	status?: number;
	url?: string;
	details?: unknown;
}

// Utility types
export type Optional<T, K extends keyof T> = Omit<T, K> & Partial<Pick<T, K>>;
export type RequiredFields<T, K extends keyof T> = T & Required<Pick<T, K>>;

// Export common type guards
export const isSession = (obj: unknown): obj is Session => {
	return (
		obj !== null &&
		typeof obj === "object" &&
		"id" in obj &&
		typeof (obj as Session).id === "string" &&
		"agent" in obj &&
		typeof (obj as Session).agent === "string"
	);
};

export const isGitStatus = (obj: unknown): obj is GitStatus => {
	return (
		obj !== null &&
		typeof obj === "object" &&
		"files" in obj &&
		Array.isArray((obj as GitStatus).files) &&
		"clean" in obj &&
		typeof (obj as GitStatus).clean === "boolean"
	);
};

export const isGitDiff = (obj: unknown): obj is GitDiff => {
	return (
		obj !== null &&
		typeof obj === "object" &&
		"files" in obj &&
		Array.isArray((obj as GitDiff).files)
	);
};

export const isProjectInfo = (obj: unknown): obj is ProjectInfo => {
	return (
		obj !== null &&
		typeof obj === "object" &&
		"id" in obj &&
		typeof (obj as ProjectInfo).id === "string" &&
		"name" in obj &&
		typeof (obj as ProjectInfo).name === "string" &&
		"path" in obj &&
		typeof (obj as ProjectInfo).path === "string"
	);
};
