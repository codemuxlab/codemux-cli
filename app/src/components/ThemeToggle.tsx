import { MoonIcon, SunIcon } from "lucide-react-native";
import { useColorScheme } from "nativewind";
import React from "react";
import { Pressable } from "react-native";
import { Icon } from "@/components/ui/icon";

export function ThemeToggle() {
	const { colorScheme, toggleColorScheme } = useColorScheme();

	return (
		<Pressable
			onPress={toggleColorScheme}
			className="mr-4 p-2 rounded-full bg-muted hover:bg-accent"
		>
			<Icon size={20} className="text-foreground">
				{colorScheme === "dark" ? SunIcon : MoonIcon}
			</Icon>
		</Pressable>
	);
}
