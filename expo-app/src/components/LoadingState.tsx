import React from "react";
import { Text, View } from "react-native";

export function LoadingState() {
	return (
		<View className="flex-1 justify-center items-center bg-background">
			<Text className="text-foreground text-lg">Loading projects...</Text>
		</View>
	);
}
