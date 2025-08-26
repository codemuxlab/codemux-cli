import { useLocalSearchParams } from "expo-router";
import React from "react";
import { ScrollView, Text, View } from "react-native";

export default function LogsTab() {
	const { sessionId } = useLocalSearchParams<{ sessionId: string }>();

	// Placeholder for logs functionality
	return (
		<View className="flex-1 bg-gray-900 p-4">
			<Text className="text-white text-lg mb-4">Session Logs</Text>
			<Text className="text-gray-400 mb-2">Session ID: {sessionId}</Text>

			<ScrollView className="flex-1">
				<View className="bg-black rounded p-4">
					<Text className="text-green-400 font-mono text-sm mb-2">
						[INFO] Session started
					</Text>
					<Text className="text-blue-400 font-mono text-sm mb-2">
						[DEBUG] WebSocket connection established
					</Text>
					<Text className="text-yellow-500 font-mono text-sm mb-2">
						[WARN] This is a placeholder logs view
					</Text>
					<Text className="text-gray-400 font-mono text-sm">
						[INFO] Logs functionality will be implemented in future updates
					</Text>
				</View>
			</ScrollView>
		</View>
	);
}
