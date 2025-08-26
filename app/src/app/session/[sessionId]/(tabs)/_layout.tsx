import { Tabs } from "expo-router";
import React from "react";

export default function SessionTabsLayout() {
	return (
		<Tabs
			screenOptions={{
				headerShown: false,
				tabBarStyle: {
					backgroundColor: "#1f2937", // gray-800
					borderTopColor: "#374151", // gray-700
					height: 60,
				},
				tabBarActiveTintColor: "#3b82f6", // blue-500
				tabBarInactiveTintColor: "#9ca3af", // gray-400
				tabBarLabelStyle: {
					fontSize: 12,
					fontWeight: "500",
				},
			}}
		>
			<Tabs.Screen
				name="terminal"
				options={{
					title: "Terminal",
					tabBarIcon: ({ color }) => (
						<span style={{ color, fontSize: 18 }}>🖥️</span>
					),
				}}
			/>
			<Tabs.Screen
				name="diff"
				options={{
					title: "Changes",
					tabBarIcon: ({ color }) => (
						<span style={{ color, fontSize: 18 }}>📋</span>
					),
				}}
			/>
			<Tabs.Screen
				name="logs"
				options={{
					title: "Logs",
					tabBarIcon: ({ color }) => (
						<span style={{ color, fontSize: 18 }}>📜</span>
					),
				}}
			/>
		</Tabs>
	);
}
