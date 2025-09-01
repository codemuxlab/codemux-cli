import {
	AlertCircleIcon,
	CheckCircle2Icon,
	Terminal,
} from "lucide-react-native";
import React from "react";
import { View } from "react-native";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Text } from "@/components/ui/text";

export function AlertDemo() {
	return (
		<View className="w-full max-w-xl gap-4 p-4">
			{/* Success Alert */}
			<Alert icon={CheckCircle2Icon}>
				<AlertTitle>Session Connected Successfully</AlertTitle>
				<AlertDescription>
					Your codemux session is ready. You can now start coding with AI.
				</AlertDescription>
			</Alert>

			{/* Info Alert */}
			<Alert icon={Terminal}>
				<AlertTitle>Terminal Ready</AlertTitle>
				<AlertDescription>
					Your terminal is connected and ready for commands.
				</AlertDescription>
			</Alert>

			{/* Error Alert */}
			<Alert variant="destructive" icon={AlertCircleIcon}>
				<AlertTitle>Connection Failed</AlertTitle>
				<AlertDescription>
					Unable to connect to the backend server. Please check your connection
					and try again.
				</AlertDescription>
				<View role="list" className="ml-0.5 pb-2 pl-6">
					<Text role="listitem" className="text-sm">
						<Text className="web:pr-2">•</Text> Check that the server is running
						on port 8765
					</Text>
					<Text role="listitem" className="text-sm">
						<Text className="web:pr-2">•</Text> Verify your network connection
					</Text>
					<Text role="listitem" className="text-sm">
						<Text className="web:pr-2">•</Text> Try refreshing the page
					</Text>
				</View>
			</Alert>

			{/* Alert with no description */}
			<Alert icon={Terminal}>
				<AlertTitle>Git repository detected in project directory</AlertTitle>
			</Alert>
		</View>
	);
}
