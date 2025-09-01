import "../global.css";
import { PortalHost } from "@rn-primitives/portal";
import { QueryClientProvider } from "@tanstack/react-query";
import { Slot } from "expo-router";
import { StatusBar } from "expo-status-bar";
import { useColorScheme } from "nativewind";
import { queryClient } from "../lib/queryClient";

export default function Layout() {
	const { colorScheme } = useColorScheme();

	return (
		<QueryClientProvider client={queryClient}>
			<StatusBar style={colorScheme === "dark" ? "light" : "dark"} />
			<Slot />
			<PortalHost />
		</QueryClientProvider>
	);
}
