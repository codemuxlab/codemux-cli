import { AlertCircleIcon } from "lucide-react-native";
import React from "react";
import { View } from "react-native";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";

export function ErrorState() {
	return (
		<View className="flex-1 justify-center items-center bg-background p-4">
			<Alert variant="destructive" icon={AlertCircleIcon} className="max-w-md">
				<AlertTitle>Failed to load projects</AlertTitle>
				<AlertDescription>
					Check that the backend is running on port 8765 and try refreshing the
					page.
				</AlertDescription>
			</Alert>
		</View>
	);
}
