// TypeScript interfaces for Claude JSONL format
// Based on analysis of actual Claude session files

export interface ClaudeJSONLEntry {
	parentUuid?: string;
	isSidechain?: boolean;
	userType?: "external" | "internal";
	cwd?: string;
	sessionId?: string;
	version?: string;
	gitBranch?: string;
	type: "user" | "assistant" | "system" | "summary";
	message?: ClaudeMessage;
	content?: string; // For system messages
	summary?: string; // For summary messages
	leafUuid?: string; // For summary messages
	isCompactSummary?: boolean; // For summary messages
	uuid?: string;
	timestamp?: string; // ISO 8601
	requestId?: string;
	toolUseResult?: ToolUseResult;
	toolUseID?: string;
	level?: "info" | "warn" | "error";
	isMeta?: boolean;
}

export interface ClaudeMessage {
	id?: string;
	type?: "message";
	role: "user" | "assistant";
	model?: string;
	content: ClaudeContent[] | ClaudeContent | string;
	stop_reason?: string | null;
	stop_sequence?: string | null;
	usage?: ClaudeUsage;
}

export type ClaudeContent =
	| ClaudeTextContent
	| ClaudeToolUseContent
	| ClaudeToolResultContent;

export interface ClaudeTextContent {
	type: "text";
	text: string;
}

export interface ClaudeToolUseContent {
	type: "tool_use";
	id: string;
	name: string;
	input: Record<string, unknown>;
}

export interface ClaudeToolResultContent {
	type: "tool_result";
	tool_use_id: string;
	content: string;
}

export interface ClaudeUsage {
	input_tokens: number;
	cache_creation_input_tokens?: number;
	cache_read_input_tokens?: number;
	output_tokens: number;
	service_tier: string;
	cache_creation?: {
		ephemeral_5m_input_tokens?: number;
		ephemeral_1h_input_tokens?: number;
	};
}

export interface ToolUseResult {
	type: "text" | "file";
	content?: string;
	file?: {
		filePath: string;
		content: string;
		numLines: number;
		startLine: number;
		totalLines: number;
	};
}

// Helper functions for extracting text from different message types

function cleanAnsiCodes(text: string): string {
	// Remove ANSI escape codes - construct the escape character to avoid linter warning
	const esc = String.fromCharCode(0x1b);
	return text.replace(new RegExp(`${esc}\\[[0-9;]*m`, "g"), "");
}

export function extractText(entry: ClaudeJSONLEntry): string | null {
	// Summary messages
	if (entry.type === "summary" && entry.summary) {
		return entry.summary;
	}

	// System messages
	if (entry.type === "system" && entry.content) {
		// Clean up ANSI escape codes from system messages
		return cleanAnsiCodes(entry.content);
	}

	// User/Assistant messages
	if (entry.message?.content) {
		// Handle string content directly
		if (typeof entry.message.content === "string") {
			return entry.message.content;
		}

		// Handle array of content items
		if (Array.isArray(entry.message.content)) {
			const textContents = entry.message.content
				.filter(
					(content): content is ClaudeTextContent =>
						content && typeof content === "object" && content.type === "text",
				)
				.map((content) => content.text);

			if (textContents.length > 0) {
				return textContents.join("\n");
			}
		}

		// Handle single content item
		if (
			typeof entry.message.content === "object" &&
			!Array.isArray(entry.message.content) &&
			"type" in entry.message.content &&
			entry.message.content.type === "text"
		) {
			const textContent = entry.message.content as ClaudeTextContent;
			return textContent.text;
		}
	}

	return null;
}

export function extractSummary(
	entry: ClaudeJSONLEntry,
	maxLength = 100,
): string | null {
	const text = extractText(entry);
	if (!text) return null;

	// Truncate and add ellipsis if too long
	if (text.length <= maxLength) {
		return text;
	}

	return `${text.substring(0, maxLength).trim()}...`;
}

export function getMessageType(entry: ClaudeJSONLEntry): string {
	if (entry.type === "summary") {
		return entry.isCompactSummary ? "Summary (Compact)" : "Summary";
	}

	if (entry.type === "system") {
		return "System";
	}

	if (entry.message?.role === "user") {
		// Check if it's a tool result
		const hasToolResult =
			Array.isArray(entry.message.content) &&
			entry.message.content.some(
				(content) =>
					content &&
					typeof content === "object" &&
					content.type === "tool_result",
			);
		return hasToolResult ? "Tool Result" : "User";
	}

	if (entry.message?.role === "assistant") {
		// Check if it contains tool use
		const hasToolUse =
			Array.isArray(entry.message.content) &&
			entry.message.content.some(
				(content) =>
					content && typeof content === "object" && content.type === "tool_use",
			);
		return hasToolUse ? "Assistant + Tools" : "Assistant";
	}

	return "Unknown";
}

export function getMessageIcon(entry: ClaudeJSONLEntry): string {
	const type = getMessageType(entry);

	switch (type) {
		case "Summary":
		case "Summary (Compact)":
			return "📝";
		case "User":
			return "👤";
		case "Assistant":
			return "🤖";
		case "Assistant + Tools":
			return "🛠️";
		case "Tool Result":
			return "📋";
		case "System":
			return "⚙️";
		default:
			return "❓";
	}
}

// Type guard functions
export const isClaudeJSONLEntry = (obj: unknown): obj is ClaudeJSONLEntry => {
	if (typeof obj !== "object" || obj === null || !("type" in obj)) {
		return false;
	}

	const entry = obj as ClaudeJSONLEntry;
	const validTypes = ["user", "assistant", "system", "summary"];
	return validTypes.includes(entry.type);
};

export const isAssistantMessage = (entry: ClaudeJSONLEntry): boolean => {
	return entry.type === "assistant" || entry.message?.role === "assistant";
};

export const isUserMessage = (entry: ClaudeJSONLEntry): boolean => {
	return entry.type === "user" || entry.message?.role === "user";
};

export const isSystemMessage = (entry: ClaudeJSONLEntry): boolean => {
	return entry.type === "system";
};

export const isSummaryMessage = (entry: ClaudeJSONLEntry): boolean => {
	return entry.type === "summary";
};
