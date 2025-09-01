import React from "react";
import { Text, View } from "react-native";

export function LoadingState() {
	return (
		<View className="flex-1 justify-center items-center bg-gray-900">
			<Text className="text-white text-lg">Loading projects...</Text>
		</View>
	);
}
