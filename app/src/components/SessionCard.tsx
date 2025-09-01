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

	const getStatusStyles = () => {
		if (session.attributes?.status === "running") {
			return {
				bg: "bg-green-900",
				text: "text-green-400",
			};
		}
		if (session.attributes?.session_type === "Active") {
			return {
				bg: "bg-blue-900",
				text: "text-blue-400",
			};
		}
		return {
			bg: "bg-gray-600",
			text: "text-gray-400",
		};
	};

	const statusStyles = getStatusStyles();

	return (
		<Card>
			<CardHeader>
				<View className="flex-row justify-between items-center">
					<View className="flex-1">
						<CardTitle className="text-sm">Session: {session.id}</CardTitle>
						<View className="flex-row items-center gap-2 mt-1">
							<CardDescription className="text-xs">
								Agent: {session.attributes?.agent || "Unknown"}
							</CardDescription>
							{session.attributes?.session_type === "Historical" && (
								<View className="px-1.5 py-0.5 rounded bg-amber-900">
									<Text className="text-amber-400 text-xs">HISTORICAL</Text>
								</View>
							)}
						</View>
					</View>
					<View className={`px-2 py-1 rounded ${statusStyles.bg}`}>
						<Text className={`text-xs ${statusStyles.text}`}>
							{session.attributes?.status?.toUpperCase() || "UNKNOWN"}
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
