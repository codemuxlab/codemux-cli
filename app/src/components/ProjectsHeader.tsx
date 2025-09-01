import React from "react";
import { Text, View } from "react-native";

export function ProjectsHeader() {
	return (
		<View className="bg-gray-800 p-6 border-b border-gray-700">
			<Text className="text-white text-2xl font-bold mb-2">
				Codemux Projects
			</Text>
			<Text className="text-gray-400">Manage your AI coding projects</Text>
		</View>
	);
}
