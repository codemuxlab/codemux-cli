import { Tabs } from "expo-router";
import React from "react";

export default function SessionLayout() {
	return (
		<Tabs
			screenOptions={{
				headerShown: false,
				tabBarStyle: {
					backgroundColor: "hsl(0 0% 3.9%)", // --card (dark theme)
					borderTopColor: "hsl(0 0% 14.9%)", // --border (dark theme)
				},
				tabBarActiveTintColor: "hsl(0 0% 98%)", // --primary (dark theme)
				tabBarInactiveTintColor: "hsl(0 0% 63.9%)", // --muted-foreground (dark theme)
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
