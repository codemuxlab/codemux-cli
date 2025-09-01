import React from "react";
import { Text, TouchableOpacity, View } from "react-native";

interface GitFileDiff {
	path: string;
	old_path?: string;
	status: string;
	additions: number;
	deletions: number;
	diff: string;
}

interface FileDiffHeaderProps {
	file: GitFileDiff;
	onRefresh: () => void;
}

export function FileDiffHeader({ file, onRefresh }: FileDiffHeaderProps) {
	const getStatusColor = (status: string) => {
		switch (status) {
			case "modified":
				return "text-yellow-500";
			case "added":
				return "text-green-500";
			case "deleted":
				return "text-red-500";
			case "renamed":
				return "text-primary";
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
			default:
				return "📄";
		}
	};

	return (
		<View className="bg-card border-b border-border p-4">
			<View className="flex-row items-center justify-between">
				<View className="flex-1">
					{/* File path and status */}
					<View className="flex-row items-center mb-2">
						<Text className="text-lg mr-2">{getStatusIcon(file.status)}</Text>
						<Text className="text-white text-lg font-mono font-semibold flex-1">
							{file.path}
						</Text>
						<Text
							className={`text-sm font-semibold ${getStatusColor(file.status)}`}
						>
							{file.status.toUpperCase()}
						</Text>
					</View>

					{/* Renamed file info */}
					{file.old_path && file.old_path !== file.path && (
						<View className="mb-2">
							<Text className="text-gray-400 text-sm font-mono">
								Renamed from: {file.old_path}
							</Text>
						</View>
					)}

					{/* Stats */}
					<View className="flex-row items-center">
						<View className="flex-row items-center mr-4">
							<Text className="text-green-500 text-sm font-semibold">
								+{file.additions}
							</Text>
							<Text className="text-gray-400 text-sm mx-1">/</Text>
							<Text className="text-red-500 text-sm font-semibold">
								-{file.deletions}
							</Text>
						</View>

						{/* Visual diff bar */}
						<View className="flex-1 h-2 bg-muted rounded-full overflow-hidden mr-4">
							{(file.additions > 0 || file.deletions > 0) && (
								<View className="flex-row h-full">
									<View
										className="bg-green-500"
										style={{
											width: `${(file.additions / (file.additions + file.deletions)) * 100}%`,
										}}
									/>
									<View
										className="bg-red-500"
										style={{
											width: `${(file.deletions / (file.additions + file.deletions)) * 100}%`,
										}}
									/>
								</View>
							)}
						</View>

						<Text className="text-gray-400 text-sm">
							{file.additions + file.deletions} lines
						</Text>
					</View>
				</View>

				{/* Actions */}
				<View className="ml-4">
					<TouchableOpacity
						onPress={onRefresh}
						className="bg-muted px-3 py-1 rounded mr-2"
					>
						<Text className="text-white text-xs">🔄</Text>
					</TouchableOpacity>
				</View>
			</View>
		</View>
	);
}
