import React from "react";
import { Text, View } from "react-native";

export function ErrorState() {
	return (
		<View className="flex-1 justify-center items-center bg-gray-900">
			<Text className="text-red-400 text-lg mb-4">Failed to load projects</Text>
			<Text className="text-gray-400 text-sm text-center px-4">
				Check that the backend is running on port 8765
			</Text>
		</View>
	);
}
