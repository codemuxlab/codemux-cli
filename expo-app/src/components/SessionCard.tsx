import { useRouter } from "expo-router";
import React from "react";
import { Text, View } from "react-native";
import { Button } from "@/components/ui/button";
import {
	Card,
	CardContent,
	CardDescription,
	CardHeader,
	CardTitle,
} from "@/components/ui/card";
import { Text as UIText } from "@/components/ui/text";
import { LastMessage } from "./LastMessage";

interface SessionAttributes {
	agent?: string;
	status?: string;
	session_type?: string;
	last_message?: string;
}

interface Session {
	id: string;
	attributes?: SessionAttributes;
}

interface SessionCardProps {
	session: Session;
}

export function SessionCard({ session }: SessionCardProps) {
	const router = useRouter();

	const getSessionStatus = () => {
		const status = session.attributes?.status;
		const sessionType = session.attributes?.session_type;

		// Priority: running status > session type > default
		if (status === "running") {
			return {
				label: "RUNNING",
				bg: "bg-primary",
				text: "text-primary-foreground",
			};
		}
		if (status === "completed") {
			return {
				label: "COMPLETED",
				bg: "bg-secondary",
				text: "text-secondary-foreground",
			};
		}
		if (sessionType === "Historical") {
			return {
				label: "HISTORICAL",
				bg: "bg-muted",
				text: "text-muted-foreground",
			};
		}
		if (sessionType === "Active") {
			return {
				label: "ACTIVE",
				bg: "bg-secondary",
				text: "text-secondary-foreground",
			};
		}
		return {
			label: status?.toUpperCase() || "UNKNOWN",
			bg: "bg-muted",
			text: "text-muted-foreground",
		};
	};

	const sessionStatus = getSessionStatus();

	return (
		<Card>
			<CardHeader>
				<View className="flex-row justify-between items-center">
					<View className="flex-1">
						<CardTitle className="text-sm">Session: {session.id}</CardTitle>
						<CardDescription className="text-xs mt-1">
							Agent: {session.attributes?.agent || "Unknown"}
						</CardDescription>
					</View>
					<View className={`px-2 py-1 rounded ${sessionStatus.bg}`}>
						<Text className={`text-xs ${sessionStatus.text}`}>
							{sessionStatus.label}
						</Text>
					</View>
				</View>
			</CardHeader>

			<CardContent>
				<LastMessage
					message={session.attributes?.last_message}
					agent={session.attributes?.agent}
				/>

				<View className="flex-row gap-2 mt-2">
					<Button
						variant="outline"
						size="sm"
						onPress={() => router.push(`/session/${session.id}/terminal`)}
					>
						<UIText className="text-xs">🖥️ Terminal</UIText>
					</Button>
					<Button
						variant="outline"
						size="sm"
						onPress={() => router.push(`/session/${session.id}/diff`)}
					>
						<UIText className="text-xs">📋 Changes</UIText>
					</Button>
					<Button
						variant="outline"
						size="sm"
						onPress={() => router.push(`/session/${session.id}/logs`)}
					>
						<UIText className="text-xs">📜 Logs</UIText>
					</Button>
				</View>
			</CardContent>
		</Card>
	);
}
