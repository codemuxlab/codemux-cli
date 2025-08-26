import React, { useState } from "react";
import { ScrollView, Text, TouchableOpacity, View } from "react-native";
import { handleApiError, useGitData, useRefreshGit } from "../hooks/api";
import { DiffLine } from "./DiffLine";
import { FileDiffHeader } from "./FileDiffHeader";

interface GitDiffViewerProps {
	sessionId: string;
}

export default function GitDiffViewer({ sessionId }: GitDiffViewerProps) {
	const [selectedFile, setSelectedFile] = useState<string | null>(null);

	// Use the combined git data hook
	const { status, diff, isLoading, error } = useGitData(sessionId);
	const { refreshAll } = useRefreshGit(sessionId);

	// Extract data from queries
	const gitStatus = status.data;
	const gitDiff = diff.data;

	// Auto-select first file when diff data changes
	React.useEffect(() => {
		if (gitDiff?.files.length && !selectedFile) {
			setSelectedFile(gitDiff.files[0].path);
		}
	}, [gitDiff, selectedFile]);

	// Early return if no session ID
	if (!sessionId || sessionId.trim() === "") {
		return (
			<View className="flex-1 bg-gray-900 p-4">
				<Text className="text-red-500 text-center">No session ID provided</Text>
			</View>
		);
	}

	const getStatusColor = (status: string) => {
		switch (status) {
			case "modified":
				return "text-yellow-500";
			case "added":
				return "text-green-500";
			case "deleted":
				return "text-red-500";
			case "renamed":
				return "text-blue-500";
			case "untracked":
				return "text-gray-500";
			default:
				return "text-white";
		}
	};

	const getStatusIcon = (status: string) => {
		switch (status) {
			case "modified":
				return "✏️";
			case "added":
				return "➕";
			case "deleted":
				return "❌";
			case "renamed":
				return "🔄";
			case "untracked":
				return "❓";
			default:
				return "📄";
		}
	};

	const selectedFileDiff = gitDiff?.files.find((f) => f.path === selectedFile);

	if (isLoading) {
		return (
			<View className="flex-1 bg-gray-900 p-4">
				<Text className="text-white text-center">Loading git status...</Text>
			</View>
		);
	}

	if (error) {
		return (
			<View className="flex-1 bg-gray-900 p-4">
				<Text className="text-red-500 text-center">
					Error: {handleApiError(error)}
				</Text>
				<TouchableOpacity
					onPress={() => {
						refreshAll();
					}}
					className="mt-4 bg-blue-600 p-2 rounded"
				>
					<Text className="text-white text-center">Retry</Text>
				</TouchableOpacity>
			</View>
		);
	}

	if (gitStatus?.clean) {
		return (
			<View className="flex-1 bg-gray-900 p-4">
				<View className="flex-row items-center mb-4">
					<Text className="text-green-500 text-lg">✓ Working tree clean</Text>
					{gitStatus.branch && (
						<Text className="text-gray-400 ml-2">({gitStatus.branch})</Text>
					)}
				</View>
				<Text className="text-gray-400">No changes to display</Text>
			</View>
		);
	}

	return (
		<View className="flex-1 bg-gray-900">
			{/* Header with branch info */}
			<View className="bg-gray-800 p-3 border-b border-gray-700">
				<View className="flex-row items-center justify-between">
					<View className="flex-row items-center">
						{gitStatus?.branch && (
							<Text className="text-white text-sm">📍 {gitStatus.branch}</Text>
						)}
						<Text className="text-gray-400 text-sm ml-4">
							{gitStatus?.files.length || 0} files changed
						</Text>
					</View>
					<TouchableOpacity
						onPress={() => {
							refreshAll();
						}}
						className="bg-gray-700 px-3 py-1 rounded"
					>
						<Text className="text-white text-xs">🔄 Refresh</Text>
					</TouchableOpacity>
				</View>
			</View>

			<View className="flex-1 flex-row">
				{/* File list sidebar */}
				<View className="w-1/3 bg-gray-800 border-r border-gray-700">
					<ScrollView className="flex-1">
						{gitStatus?.files.map((file) => (
							<TouchableOpacity
								key={file.path}
								onPress={() => setSelectedFile(file.path)}
								className={`p-3 border-b border-gray-700 ${
									selectedFile === file.path
										? "bg-blue-900"
										: "hover:bg-gray-700"
								}`}
							>
								<View className="flex-row items-center">
									<Text className="text-lg mr-2">
										{getStatusIcon(file.status)}
									</Text>
									<View className="flex-1">
										<Text className="text-white text-sm font-mono truncate">
											{file.path}
										</Text>
										<View className="flex-row items-center mt-1">
											<Text
												className={`text-xs ${getStatusColor(file.status)}`}
											>
												{file.status}
											</Text>
											{file.additions !== undefined &&
												file.deletions !== undefined && (
													<Text className="text-gray-400 text-xs ml-2">
														+{file.additions} -{file.deletions}
													</Text>
												)}
										</View>
									</View>
								</View>
							</TouchableOpacity>
						))}
					</ScrollView>
				</View>

				{/* Diff content */}
				<View className="flex-1 bg-gray-900">
					{selectedFileDiff ? (
						<View className="flex-1">
							<FileDiffHeader
								file={selectedFileDiff}
								onRefresh={() => refreshAll()}
							/>
							<ScrollView className="flex-1 p-4">
								<View className="bg-black rounded p-4">
									{selectedFileDiff.diff.split("\n").map((line, index) => (
										<DiffLine
											key={`${selectedFileDiff.path}-${index}`}
											line={line}
											lineNumber={index + 1}
										/>
									))}
								</View>
							</ScrollView>
						</View>
					) : (
						<View className="flex-1 justify-center items-center">
							<Text className="text-gray-400 text-lg">
								Select a file to view diff
							</Text>
						</View>
					)}
				</View>
			</View>
		</View>
	);
}
