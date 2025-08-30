import type React from "react";
import { memo, useCallback, useEffect, useRef, useState } from "react";
import {
	ScrollView,
	Text,
	TextInput,
	TouchableOpacity,
	View,
} from "react-native";
import {
	availableThemes,
	useTerminalStore,
	type WebKeyEvent,
} from "../stores/terminalStore";
import type { ClientMessage, ServerMessage } from "../types/bindings";
import { TerminalCell } from "./TerminalCell";

interface TerminalProps {
	sessionId: string;
}

// Memoized row component to prevent unnecessary re-renders
const TerminalRow = memo(({ row, cols }: { row: number; cols: number }) => {
	const cells = [];

	for (let col = 0; col < cols; col++) {
		cells.push(<TerminalCell key={`${row}-${col}`} row={row} col={col} />);
	}

	return <View className="flex-row">{cells}</View>;
});

TerminalRow.displayName = "TerminalRow";

// Separate component for the terminal grid to isolate re-renders
const TerminalGrid = memo(() => {
	// Only subscribe to size changes
	const size = useTerminalStore((state) => state.size);
	const cellCount = useTerminalStore((state) => state.cells.size);

	console.log(
		`TerminalGrid rendering: ${size.rows}x${size.cols}, ${cellCount} cells in store`,
	);

	const rows = [];
	for (let row = 0; row < size.rows; row++) {
		rows.push(<TerminalRow key={row} row={row} cols={size.cols} />);
	}

	return <View>{rows}</View>;
});

TerminalGrid.displayName = "TerminalGrid";

// Separate input component to isolate input state changes
const TerminalInput = memo(
	({ onSubmit }: { onSubmit: (text: string) => void }) => {
		const [inputValue, setInputValue] = useState("");

		const handleSubmit = useCallback(() => {
			if (inputValue.trim()) {
				onSubmit(inputValue);
				setInputValue("");
			}
		}, [inputValue, onSubmit]);

		return (
			<View className="flex-row p-2 bg-gray-800 items-center">
				<TextInput
					className="flex-1 bg-white text-black p-2 font-mono text-sm mr-2 rounded"
					value={inputValue}
					onChangeText={setInputValue}
					onSubmitEditing={handleSubmit}
					placeholder="Type your input here..."
					placeholderTextColor="#666666"
					multiline={false}
					returnKeyType="send"
					autoCorrect={false}
					autoCapitalize="none"
				/>
				<Text className="text-white text-xs bg-gray-600 p-1 rounded">
					Enter to send
				</Text>
			</View>
		);
	},
);

TerminalInput.displayName = "TerminalInput";

// Background component with theme support
const TerminalBackground = memo(
	({ children }: { children: React.ReactNode }) => {
		const theme = useTerminalStore((state) => state.theme);

		return (
			<View
				className="flex-1 w-full"
				style={{ backgroundColor: theme.background }}
			>
				{children}
			</View>
		);
	},
);

TerminalBackground.displayName = "TerminalBackground";

// Dark/Light mode toggle component
const DarkLightToggle = memo(() => {
	const currentTheme = useTerminalStore((state) => state.theme);
	const setTheme = useTerminalStore((state) => state.setTheme);

	const isDark = currentTheme.name === "Default Dark";

	const toggleMode = () => {
		if (isDark) {
			setTheme(
				availableThemes.find((t) => t.name === "Light") || availableThemes[1],
			);
		} else {
			setTheme(
				availableThemes.find((t) => t.name === "Default Dark") ||
					availableThemes[0],
			);
		}
	};

	return (
		<TouchableOpacity
			onPress={toggleMode}
			className="bg-gray-700 px-3 py-1 rounded mr-2"
		>
			<Text className="text-white text-xs">
				{isDark ? "☀️ Light" : "🌙 Dark"}
			</Text>
		</TouchableOpacity>
	);
});

DarkLightToggle.displayName = "DarkLightToggle";

// Theme selector component
const ThemeSelector = memo(() => {
	const [showThemes, setShowThemes] = useState(false);
	const currentTheme = useTerminalStore((state) => state.theme);
	const setTheme = useTerminalStore((state) => state.setTheme);

	return (
		<View className="relative">
			<TouchableOpacity
				onPress={() => setShowThemes(!showThemes)}
				className="bg-gray-700 px-3 py-1 rounded"
			>
				<Text className="text-white text-xs">{currentTheme.name}</Text>
			</TouchableOpacity>

			{showThemes && (
				<View className="absolute top-8 right-0 bg-gray-800 rounded shadow-lg z-10 min-w-32">
					{availableThemes.map((theme) => (
						<TouchableOpacity
							key={theme.name}
							onPress={() => {
								setTheme(theme);
								setShowThemes(false);
							}}
							className="p-2 border-b border-gray-600 last:border-b-0"
						>
							<Text
								className={`text-xs ${currentTheme.name === theme.name ? "text-blue-400 font-bold" : "text-white"}`}
							>
								{theme.name}
							</Text>
						</TouchableOpacity>
					))}
				</View>
			)}
		</View>
	);
});

ThemeSelector.displayName = "ThemeSelector";

export default function Terminal({ sessionId }: TerminalProps) {
	const [isConnected, setIsConnected] = useState(false);
	const wsRef = useRef<WebSocket | null>(null);
	const scrollViewRef = useRef<ScrollView>(null);
	const terminalRef = useRef<View>(null);

	const handleWebSocketMessage = useCallback((message: ServerMessage) => {
		console.log("WebSocket message received:", message.type, message);

		switch (message.type) {
			case "grid_update":
				if ("Keyframe" in message) {
					console.log("Grid update keyframe:", {
						size: message.Keyframe.size,
						cellCount: message.Keyframe.cells.length,
						cursor: message.Keyframe.cursor,
						cursor_visible: message.Keyframe.cursor_visible,
					});

					// Transform keyframe data to match store expectations
					const transformedMessage = {
						type: "grid_update",
						size: message.Keyframe.size,
						cells: message.Keyframe.cells,
						cursor: {
							row: message.Keyframe.cursor[0],
							col: message.Keyframe.cursor[1],
						},
						cursor_visible: message.Keyframe.cursor_visible,
						timestamp: message.Keyframe.timestamp,
					};

					useTerminalStore.getState().handleGridUpdate(transformedMessage);
				} else if ("Diff" in message) {
					console.log("Grid update diff:", {
						changeCount: message.Diff.changes.length,
						cursor: message.Diff.cursor,
						cursor_visible: message.Diff.cursor_visible,
					});

					// Transform diff data to match store expectations
					const transformedMessage = {
						type: "grid_update",
						cells: message.Diff.changes,
						cursor: message.Diff.cursor
							? {
									row: message.Diff.cursor[0],
									col: message.Diff.cursor[1],
								}
							: undefined,
						cursor_visible: message.Diff.cursor_visible,
						timestamp: message.Diff.timestamp,
					};

					useTerminalStore.getState().handleGridUpdate(transformedMessage);
				}
				break;
			case "pty_size":
				console.log("PTY size update:", message.rows, "x", message.cols);
				useTerminalStore.getState().updateSize(message.rows, message.cols);
				break;
			case "output":
				// Handle legacy output messages - these are raw terminal output
				console.log(
					"Received raw output:",
					message.data,
					"at",
					message.timestamp,
				);
				break;
			case "error":
				console.error("Server error:", message.message);
				break;
			default:
				console.log("Unknown message type:", message);
		}
	}, []);

	useEffect(() => {
		// Connect to WebSocket
		const wsUrl = `ws://localhost:8765/ws/${sessionId}`;
		const ws = new WebSocket(wsUrl);
		wsRef.current = ws;

		ws.onopen = () => {
			setIsConnected(true);
			console.log("WebSocket connected");

			// Request initial keyframe to get current terminal state
			// TODO: This message type is not in the generated ClientMessage union
			// Consider adding it to the Rust backend or removing this functionality
			ws.send(
				JSON.stringify({
					type: "request_keyframe",
				}),
			);
		};

		ws.onmessage = (event) => {
			try {
				const message = JSON.parse(event.data) as ServerMessage;
				handleWebSocketMessage(message);
			} catch (error) {
				console.error("Failed to parse WebSocket message:", error);
			}
		};

		ws.onclose = () => {
			setIsConnected(false);
			console.log("WebSocket disconnected");
		};

		ws.onerror = (error) => {
			console.error("WebSocket error:", error);
		};

		return () => {
			if (ws.readyState === WebSocket.OPEN) {
				ws.close();
			}
		};
	}, [sessionId, handleWebSocketMessage]);

	const sendScrollEvent = useCallback(
		(direction: "Up" | "Down", lines: number = 1) => {
			if (wsRef.current && wsRef.current.readyState === WebSocket.OPEN) {
				const message: ClientMessage = {
					type: "scroll",
					direction,
					lines,
				};
				wsRef.current.send(JSON.stringify(message));
			}
		},
		[],
	);

	// Add wheel event listener for web platforms
	useEffect(() => {
		const handleWheel = (event: WheelEvent) => {
			// Prevent default scroll behavior
			event.preventDefault();

			// Determine scroll direction from wheel delta
			const direction = event.deltaY > 0 ? "Down" : "Up";

			sendScrollEvent(direction, 1);
		};

		// Add to document for web platforms
		if (typeof window !== "undefined") {
			document.addEventListener("wheel", handleWheel, { passive: false });
			return () => {
				document.removeEventListener("wheel", handleWheel);
			};
		}
	}, [sendScrollEvent]);

	const _sendInput = useCallback((data: string) => {
		if (wsRef.current && wsRef.current.readyState === WebSocket.OPEN) {
			wsRef.current.send(
				JSON.stringify({
					type: "input",
					data: data,
				}),
			);
		}
	}, []);

	const sendKeyEvent = useCallback((keyEvent: WebKeyEvent) => {
		if (wsRef.current && wsRef.current.readyState === WebSocket.OPEN) {
			const message: ClientMessage = {
				type: "key",
				code: keyEvent.code,
				modifiers: keyEvent.modifiers,
			};
			wsRef.current.send(JSON.stringify(message));
		}
	}, []);

	const handleInputSubmit = useCallback(
		(text: string) => {
			// Send each character as a key event for better terminal compatibility
			for (const char of text) {
				sendKeyEvent({
					code: { Char: char },
					modifiers: { shift: false, ctrl: false, alt: false, meta: false },
				});
			}
			// Send Enter key
			sendKeyEvent({
				code: "Enter",
				modifiers: { shift: false, ctrl: false, alt: false, meta: false },
			});
		},
		[sendKeyEvent],
	);

	// Handle keyboard events for direct key input
	const handleKeyDown = useCallback(
		(event: KeyboardEvent) => {
			// Prevent default browser behavior for most keys
			if (!["F5", "F12"].includes(event.key)) {
				event.preventDefault();
			}

			const modifiers = {
				shift: event.shiftKey || false,
				ctrl: event.ctrlKey || false,
				alt: event.altKey || false,
				meta: event.metaKey || false,
			};

			let keyCode: import("../types/bindings").KeyCode;

			// Map common keys
			switch (event.key) {
				case "Enter":
					keyCode = "Enter";
					break;
				case "Backspace":
					keyCode = "Backspace";
					break;
				case "Tab":
					keyCode = "Tab";
					break;
				case "Escape":
					keyCode = "Esc";
					break;
				case "ArrowLeft":
					keyCode = "Left";
					break;
				case "ArrowRight":
					keyCode = "Right";
					break;
				case "ArrowUp":
					keyCode = "Up";
					break;
				case "ArrowDown":
					keyCode = "Down";
					break;
				case "Home":
					keyCode = "Home";
					break;
				case "End":
					keyCode = "End";
					break;
				case "PageUp":
					keyCode = "PageUp";
					break;
				case "PageDown":
					keyCode = "PageDown";
					break;
				case "Delete":
					keyCode = "Delete";
					break;
				case "Insert":
					keyCode = "Insert";
					break;
				default:
					// Handle function keys
					if (event.key.startsWith("F") && event.key.length > 1) {
						const fNum = parseInt(event.key.slice(1), 10);
						if (!Number.isNaN(fNum)) {
							keyCode = { F: fNum };
						}
					} else if (event.key.length === 1) {
						// Regular character
						keyCode = { Char: event.key };
					} else {
						return;
					}
					break;
			}

			sendKeyEvent({
				code: keyCode,
				modifiers: modifiers,
			});
		},
		[sendKeyEvent],
	);

	// Set up keyboard event listener
	useEffect(() => {
		const handleKeyDownEvent = (event: KeyboardEvent) => {
			handleKeyDown(event);
		};

		document.addEventListener("keydown", handleKeyDownEvent);
		return () => {
			document.removeEventListener("keydown", handleKeyDownEvent);
		};
	}, [handleKeyDown]);

	return (
		<View className="flex-1 bg-black" ref={terminalRef}>
			{/* Connection status and theme controls */}
			<View
				className={`p-2 flex-row justify-between items-center ${isConnected ? "bg-green-700" : "bg-red-700"}`}
			>
				<Text className="text-white text-xs">
					{isConnected
						? `Connected to session ${sessionId.slice(0, 8)}`
						: "Disconnected"}
				</Text>
				<View className="flex-row items-center">
					<DarkLightToggle />
					<ThemeSelector />
				</View>
			</View>

			{/* Terminal grid container - constrain ScrollView size */}
			<TerminalBackground>
				<ScrollView
					ref={scrollViewRef}
					className="w-full"
					showsVerticalScrollIndicator={true}
					showsHorizontalScrollIndicator={true}
					contentContainerStyle={{
						justifyContent: "center",
						alignItems: "center",
						padding: 16,
						minHeight: "100%",
					}}
				>
					<View ref={terminalRef}>
						<TerminalGrid />
					</View>
				</ScrollView>
			</TerminalBackground>

			{/* Input area */}
			<TerminalInput onSubmit={handleInputSubmit} />
		</View>
	);
}
