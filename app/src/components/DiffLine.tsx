import React from "react";
import { Text, View } from "react-native";

interface DiffLineProps {
	line: string;
	lineNumber: number;
}

export function DiffLine({ line, lineNumber }: DiffLineProps) {
	const getLineStyle = (line: string) => {
		if (line.startsWith("+") && !line.startsWith("+++")) {
			return {
				backgroundColor: "#064e3b", // dark green background
				borderLeft: "3px solid #10b981", // green border
				textColor: "text-green-200",
			};
		} else if (line.startsWith("-") && !line.startsWith("---")) {
			return {
				backgroundColor: "#7f1d1d", // dark red background
				borderLeft: "3px solid #ef4444", // red border
				textColor: "text-red-200",
			};
		} else if (line.startsWith("@@")) {
			return {
				backgroundColor: "#374151", // gray background for context lines
				borderLeft: "3px solid #6b7280",
				textColor: "text-blue-300",
			};
		} else if (line.startsWith("+++") || line.startsWith("---")) {
			return {
				backgroundColor: "#1f2937",
				borderLeft: "3px solid #4b5563",
				textColor: "text-gray-400",
			};
		} else {
			return {
				backgroundColor: "transparent",
				borderLeft: "3px solid transparent",
				textColor: "text-gray-300",
			};
		}
	};

	const getLinePrefix = (line: string) => {
		if (line.startsWith("@@")) {
			// Extract line numbers from context header like "@@ -1,4 +1,6 @@"
			return line;
		} else if (line.startsWith("+") && !line.startsWith("+++")) {
			return "+";
		} else if (line.startsWith("-") && !line.startsWith("---")) {
			return "-";
		} else {
			return " ";
		}
	};

	const getLineContent = (line: string) => {
		if (line.startsWith("@@")) {
			return line;
		} else if (
			(line.startsWith("+") || line.startsWith("-")) &&
			!line.startsWith("+++") &&
			!line.startsWith("---")
		) {
			return line.slice(1); // Remove the +/- prefix for display
		} else {
			return line;
		}
	};

	const lineStyle = getLineStyle(line);
	const prefix = getLinePrefix(line);
	const content = getLineContent(line);

	// Skip empty lines but preserve spacing
	if (line.trim() === "") {
		return (
			<View className="flex-row min-h-[1.25rem]">
				<Text className="text-xs text-gray-500 w-12 text-right pr-2 font-mono">
					{lineNumber}
				</Text>
				<View className="flex-1" />
			</View>
		);
	}

	// Special layout for diff headers (@@) - span full width
	if (line.startsWith("@@")) {
		return (
			<View
				className="flex-row min-h-[1.25rem] py-0.5"
				style={{
					backgroundColor: lineStyle.backgroundColor,
					borderLeftWidth: 3,
					borderLeftColor: lineStyle.borderLeft.split(" ")[2],
				}}
			>
				{/* Line number */}
				<Text className="text-xs text-gray-500 w-12 text-right pr-2 font-mono">
					{lineNumber}
				</Text>

				{/* Full width header content - no prefix column */}
				<Text
					className={`text-sm font-mono flex-1 pl-2 ${lineStyle.textColor}`}
				>
					{content}
				</Text>
			</View>
		);
	}

	// Regular diff lines with three-column layout
	return (
		<View
			className="flex-row min-h-[1.25rem] py-0.5"
			style={{
				backgroundColor: lineStyle.backgroundColor,
				borderLeftWidth: 3,
				borderLeftColor: lineStyle.borderLeft.split(" ")[2],
			}}
		>
			{/* Line number */}
			<Text className="text-xs text-gray-500 w-12 text-right pr-2 font-mono">
				{lineNumber}
			</Text>

			{/* Diff prefix (+/-/space) */}
			<Text
				className={`text-sm w-6 text-center font-mono ${lineStyle.textColor}`}
			>
				{prefix}
			</Text>

			{/* Line content */}
			<Text className={`text-sm font-mono flex-1 ${lineStyle.textColor}`}>
				{content}
			</Text>
		</View>
	);
}
