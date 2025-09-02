import React from "react";
import { ScrollView, View } from "react-native";
import { EmptyProjectsState } from "../components/EmptyProjectsState";
import { ErrorState } from "../components/ErrorState";
import { LoadingState } from "../components/LoadingState";
import { ProjectContainer } from "../components/ProjectContainer";
import { ProjectsHeader } from "../components/ProjectsHeader";
import { useProjects } from "../hooks/api";

export default function Page() {
	const { data: projects = [], isLoading: loading, error } = useProjects();

	if (loading) {
		return <LoadingState />;
	}

	if (error) {
		return <ErrorState />;
	}

	return (
		<View className="flex-1 bg-background">
			<ProjectsHeader />

			<ScrollView className="flex-1 p-4">
				{projects.length === 0 ? (
					<EmptyProjectsState />
				) : (
					projects.map((project) => (
						<ProjectContainer key={project.id} project={project} />
					))
				)}
			</ScrollView>
		</View>
	);
}
