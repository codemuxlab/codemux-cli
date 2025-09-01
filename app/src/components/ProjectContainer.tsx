import React from "react";
import { Text, View } from "react-native";
import { SessionCard } from "./SessionCard";

interface ProjectAttributes {
	name?: string;
	path?: string;
}

interface Session {
	id: string;
	attributes?: {
		agent?: string;
		status?: string;
		session_type?: string;
		last_message?: string;
	};
}

interface ProjectRelationships {
	recent_sessions?: Session[];
}

interface Project {
	id: string;
	attributes?: ProjectAttributes;
	relationships?: ProjectRelationships;
}

interface ProjectContainerProps {
	project: Project;
}

export function ProjectContainer({ project }: ProjectContainerProps) {
	const projectSessions = project.relationships?.recent_sessions || [];

	return (
		<View className="bg-gray-800 rounded-lg p-4 mb-4 border border-gray-700">
			{/* Project Header */}
			<View className="flex-row justify-between items-start mb-3">
				<View className="flex-1">
					<Text className="text-white text-xl font-bold">
						{project.attributes?.name || "Unknown Project"}
					</Text>
					<Text className="text-gray-400 text-sm mt-1">
						{project.attributes?.path || "Unknown Path"}
					</Text>
				</View>
				<View className="px-2 py-1 rounded bg-blue-900">
					<Text className="text-blue-400 text-xs">
						{projectSessions.length} SESSION
						{projectSessions.length !== 1 ? "S" : ""}
					</Text>
				</View>
			</View>

			{/* Sessions for this project */}
			{projectSessions.length > 0 ? (
				<View className="space-y-2">
					{projectSessions.map((session) => (
						<SessionCard key={session.id} session={session} />
					))}
				</View>
			) : (
				<View className="bg-gray-700 rounded-lg p-3 text-center mt-2">
					<Text className="text-gray-400 text-sm">
						No active sessions for this project
					</Text>
				</View>
			)}
		</View>
	);
}
