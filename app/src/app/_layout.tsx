import "../global.css";
import { QueryClientProvider } from "@tanstack/react-query";
import { Slot } from "expo-router";
import { queryClient } from "../lib/queryClient";

export default function Layout() {
	return (
		<QueryClientProvider client={queryClient}>
			<Slot />
		</QueryClientProvider>
	);
}
