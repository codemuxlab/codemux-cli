import { useLocalSearchParams } from "expo-router";
import React from "react";
import { View } from "react-native";
import Terminal from "../../../../components/Terminal";

export default function TerminalTab() {
	const { sessionId } = useLocalSearchParams<{ sessionId: string }>();

	return (
		<View className="flex-1 w-full">
			<Terminal sessionId={sessionId || ""} />
		</View>
	);
}
