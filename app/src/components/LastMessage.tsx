import React from "react";
import { Text, View } from "react-native";
import {
	type ClaudeJSONLEntry,
	getMessageIcon,
	getMessageType,
	isClaudeJSONLEntry,
} from "../types/claude-jsonl";

interface LastMessageProps {
	message: string | null;
	agent: string;
}

export const LastMessage: React.FC<LastMessageProps> = ({ message, agent }) => {
	if (!message) {
		return null;
	}

	// Parse the JSON message (now expecting an array of messages)
	let parsedMessages: unknown[];
	try {
		const parsed = JSON.parse(message);
		parsedMessages = Array.isArray(parsed) ? parsed : [parsed];
	} catch {
		// If parsing fails, show the raw message
		return (
			<View className="bg-gray-600 rounded-md p-2 mb-2">
				<Text className="text-gray-300 text-xs font-semibold mb-1">
					💬 Recent Messages:
				</Text>
				<Text className="text-gray-200 text-xs leading-4" numberOfLines={2}>
					{message}
				</Text>
			</View>
		);
	}

	// Limit to 3 most recent messages
	const recentMessages = parsedMessages.slice(-3);

	return (
		<View className="bg-gray-600 rounded-md p-2 mb-2">
			<Text className="text-gray-300 text-xs font-semibold mb-1">
				💬 Recent Messages ({agent}):
			</Text>
			{recentMessages.map((parsedEntry, index) => {
				// Create a unique key from the message content
				const getMessageKey = () => {
					if (
						parsedEntry &&
						typeof parsedEntry === "object" &&
						"uuid" in parsedEntry
					) {
						return String(parsedEntry.uuid);
					}
					if (
						parsedEntry &&
						typeof parsedEntry === "object" &&
						"timestamp" in parsedEntry
					) {
						return `${String(parsedEntry.timestamp)}-${index}`;
					}
					return `message-${index}`;
				};

				// Render based on agent type
				const renderContent = () => {
					// Debug: check if the entry is valid
					if (!isClaudeJSONLEntry(parsedEntry)) {
						return (
							<Text
								className="text-gray-400 text-xs leading-4"
								numberOfLines={2}
							>
								Invalid message format:{" "}
								{JSON.stringify(parsedEntry).substring(0, 100)}...
							</Text>
						);
					}

					switch (agent.toLowerCase()) {
						case "claude":
							return renderClaudeMessage(
								parsedEntry,
								index === recentMessages.length - 1,
							);
						case "gemini":
							return renderGeminiMessage(
								parsedEntry,
								index === recentMessages.length - 1,
							);
						case "aider":
							return renderAiderMessage(
								parsedEntry,
								index === recentMessages.length - 1,
							);
						default:
							return renderGenericMessage(
								parsedEntry,
								index === recentMessages.length - 1,
							);
					}
				};

				return (
					<View
						key={getMessageKey()}
						className={
							index < recentMessages.length - 1
								? "mb-2 pb-2 border-b border-gray-500"
								: ""
						}
					>
						{renderContent()}
					</View>
				);
			})}
		</View>
	);
};

// Claude JSONL message format
function renderClaudeMessage(
	parsedMessage: unknown,
	isLatest = true,
): React.ReactElement {
	if (!isClaudeJSONLEntry(parsedMessage)) {
		return (
			<Text className="text-gray-200 text-xs leading-4" numberOfLines={2}>
				{JSON.stringify(parsedMessage)}
			</Text>
		);
	}

	const entry = parsedMessage as ClaudeJSONLEntry;
	const summaryLength = isLatest ? 120 : 80; // More text for latest message
	const messageType = getMessageType(entry);
	const icon = getMessageIcon(entry);

	// Handle different message types with specific logic
	let displayText: string | null = null;

	if (entry.type === "summary" && entry.summary) {
		displayText = entry.summary;
	} else if (entry.type === "system" && entry.content) {
		// Clean ANSI escape codes from system messages
		const esc = String.fromCharCode(0x1b);
		displayText = entry.content.replace(
			new RegExp(`${esc}\\[[0-9;]*m`, "g"),
			"",
		);
	} else if (entry.message?.content) {
		// Handle assistant messages with tools
		if (Array.isArray(entry.message.content)) {
			const textParts = entry.message.content
				.map((content) => {
					if (content && typeof content === "object") {
						if (content.type === "text") {
							return content.text;
						} else if (content.type === "tool_use") {
							return `🛠️ ${content.name}`;
						} else if (content.type === "tool_result") {
							const resultContent =
								typeof content.content === "string"
									? content.content.substring(0, 50)
									: String(content.content || "").substring(0, 50);
							return `📋 ${resultContent}...`;
						}
					}
					return null;
				})
				.filter(Boolean);

			if (textParts.length > 0) {
				displayText = textParts.join(" | ");
			}
		} else if (typeof entry.message.content === "string") {
			displayText = entry.message.content;
		}
	}

	// Handle tool use results
	if (entry.toolUseResult?.content) {
		displayText = entry.toolUseResult.content;
	}

	// Truncate if needed
	if (displayText && displayText.length > summaryLength) {
		displayText = `${displayText.substring(0, summaryLength)}...`;
	}

	if (displayText) {
		return (
			<View>
				<Text className="text-gray-400 text-xs mb-1">
					{icon} {messageType}
				</Text>
				<Text
					className={
						isLatest
							? "text-gray-200 text-xs leading-4"
							: "text-gray-300 text-xs leading-4"
					}
					numberOfLines={isLatest ? 3 : 2}
				>
					{displayText}
				</Text>
			</View>
		);
	}

	// Fallback for entries without extractable content
	return (
		<Text className="text-gray-400 text-xs italic" numberOfLines={1}>
			{messageType} -{" "}
			{entry.timestamp
				? new Date(entry.timestamp).toLocaleTimeString()
				: "No timestamp"}
		</Text>
	);
}

// Gemini message format (placeholder - adjust based on actual format when available)
function renderGeminiMessage(
	parsedMessage: unknown,
	isLatest = true,
): React.ReactElement {
	return renderGenericMessage(parsedMessage, isLatest);
}

// Aider message format (placeholder - adjust based on actual format when available)
function renderAiderMessage(
	parsedMessage: unknown,
	isLatest = true,
): React.ReactElement {
	return renderGenericMessage(parsedMessage, isLatest);
}

// Generic message format for unknown agents
function renderGenericMessage(
	parsedMessage: unknown,
	isLatest = true,
): React.ReactElement {
	if (!parsedMessage || typeof parsedMessage !== "object") {
		return (
			<Text className="text-gray-200 text-xs leading-4" numberOfLines={2}>
				{String(parsedMessage)}
			</Text>
		);
	}

	const message = parsedMessage as Record<string, unknown>;
	const maxLength = isLatest ? 120 : 80;

	// Handle tool use results first
	if (message.toolUseResult && typeof message.toolUseResult === "object") {
		const toolResult = message.toolUseResult as Record<string, unknown>;
		if (toolResult.content && typeof toolResult.content === "string") {
			const summary =
				toolResult.content.length > maxLength
					? `${toolResult.content.substring(0, maxLength)}...`
					: toolResult.content;

			return (
				<Text
					className={
						isLatest
							? "text-gray-200 text-xs leading-4"
							: "text-gray-300 text-xs leading-4"
					}
					numberOfLines={isLatest ? 3 : 2}
				>
					{summary}
				</Text>
			);
		}
	}

	// Try common message field names
	const textFields = ["text", "content", "message", "data", "summary"] as const;

	for (const field of textFields) {
		const value = message[field];
		if (value && typeof value === "string") {
			// Clean and truncate the text
			const cleanText = value.trim();
			const summary =
				cleanText.length > maxLength
					? `${cleanText.substring(0, maxLength)}...`
					: cleanText;

			return (
				<Text
					className={
						isLatest
							? "text-gray-200 text-xs leading-4"
							: "text-gray-300 text-xs leading-4"
					}
					numberOfLines={isLatest ? 3 : 2}
				>
					{summary}
				</Text>
			);
		}
	}

	// Show timestamp if available
	if (message.timestamp && typeof message.timestamp === "string") {
		return (
			<Text className="text-gray-400 text-xs italic" numberOfLines={1}>
				Message at {new Date(message.timestamp).toLocaleTimeString()}
			</Text>
		);
	}

	// Show object type as fallback
	const type = message.type || message.role || "Unknown";
	return (
		<Text className="text-gray-400 text-xs italic" numberOfLines={1}>
			{String(type)} message
		</Text>
	);
}
