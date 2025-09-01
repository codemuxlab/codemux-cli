import React from "react";
import { Text, View } from "react-native";

export function EmptyProjectsState() {
	return (
		<View className="bg-gray-800 rounded-lg p-8 items-center">
			<Text className="text-gray-400 text-lg mb-4">No projects found</Text>
			<Text className="text-gray-500 text-sm text-center">
				Start codemux from a project directory to see it here
			</Text>
		</View>
	);
}
