import { useLocalSearchParams } from "expo-router";
import React from "react";
import { View } from "react-native";
import GitDiffViewer from "../../../../components/GitDiffViewer";

export default function DiffTab() {
	const { sessionId } = useLocalSearchParams<{ sessionId: string }>();

	return (
		<View className="flex-1 w-full">
			<GitDiffViewer sessionId={sessionId || ""} />
		</View>
	);
}
