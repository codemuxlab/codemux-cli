import React from "react";
import { Text, View } from "react-native";
import { ThemeToggle } from "./ThemeToggle";

export function ProjectsHeader() {
	return (
		<View className="bg-card p-6 border-b border-border">
			<View className="flex-row justify-between items-start mb-2">
				<View className="flex-1">
					<Text className="text-card-foreground text-2xl font-bold">
						Codemux Projects
					</Text>
				</View>
				<ThemeToggle />
			</View>
			<Text className="text-muted-foreground">
				Manage your AI coding projects
			</Text>
		</View>
	);
}
