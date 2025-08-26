import { Tabs } from "expo-router";
import React from "react";

export default function SessionLayout() {
	return (
		<Tabs
			screenOptions={{
				headerShown: false,
				tabBarStyle: {
					backgroundColor: "#1f2937", // gray-800
					borderTopColor: "#374151", // gray-700
				},
				tabBarActiveTintColor: "#3b82f6", // blue-500
				tabBarInactiveTintColor: "#9ca3af", // gray-400
			}}
		>
			<Tabs.Screen
				name="(tabs)"
				options={{
					title: "Session",
					href: null, // Hide this tab from the tab bar
				}}
			/>
		</Tabs>
	);
}
