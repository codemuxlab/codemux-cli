import { Terminal } from "lucide-react-native";
import React from "react";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";

export function EmptyProjectsState() {
	return (
		<Alert icon={Terminal} className="max-w-md">
			<AlertTitle>No projects found</AlertTitle>
			<AlertDescription>
				Start codemux from a project directory to see it here. Once you run
				codemux in a project, it will appear in this list.
			</AlertDescription>
		</Alert>
	);
}
