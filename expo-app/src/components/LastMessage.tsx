import {
	Bot,
	FileText,
	MessageCircle,
	Settings,
	User,
	Wrench,
} from "lucide-react-native";
import React from "react";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import {
	type ClaudeJSONLEntry,
	extractSummary,
	getMessageType,
	isClaudeJSONLEntry,
} from "../types/claude-jsonl";

interface LastMessageProps {
	message: string | null;
	agent: string;
}

const getMessageIcon = (messageType: string) => {
	switch (messageType) {
		case "Summary":
		case "Summary (Compact)":
			return FileText;
		case "User":
			return User;
		case "Assistant":
			return Bot;
		case "Assistant + Tools":
			return Wrench;
		case "Tool Result":
			return FileText;
		case "System":
			return Settings;
		default:
			return MessageCircle;
	}
};

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
			<Alert icon={MessageCircle} className="mb-2">
				<AlertTitle>Recent Messages</AlertTitle>
				<AlertDescription numberOfLines={3}>{message}</AlertDescription>
			</Alert>
		);
	}

	// Get the most recent valid message
	const recentMessage = parsedMessages
		.slice(-1)
		.find((entry) => isClaudeJSONLEntry(entry)) as ClaudeJSONLEntry | undefined;

	if (!recentMessage) {
		return null;
	}

	const messageType = getMessageType(recentMessage);
	const IconComponent = getMessageIcon(messageType);
	const summary = extractSummary(recentMessage, 150);

	if (!summary) {
		return (
			<Alert icon={IconComponent} className="mb-2">
				<AlertTitle>{messageType}</AlertTitle>
				<AlertDescription>
					{recentMessage.timestamp
						? new Date(recentMessage.timestamp).toLocaleTimeString()
						: "No content available"}
				</AlertDescription>
			</Alert>
		);
	}

	return (
		<Alert icon={IconComponent} className="mb-2">
			<AlertTitle>
				{messageType} ({agent})
			</AlertTitle>
			<AlertDescription numberOfLines={4}>{summary}</AlertDescription>
		</Alert>
	);
};
