import { Tabs } from "expo-router";
import React from "react";

export default function SessionTabsLayout() {
	return (
		<Tabs
			screenOptions={{
				headerShown: false,
				tabBarStyle: {
					backgroundColor: "hsl(0 0% 3.9%)", // --card (dark theme)
					borderTopColor: "hsl(0 0% 14.9%)", // --border (dark theme)
					height: 60,
				},
				tabBarActiveTintColor: "hsl(0 0% 98%)", // --primary (dark theme)
				tabBarInactiveTintColor: "hsl(0 0% 63.9%)", // --muted-foreground (dark theme)
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
