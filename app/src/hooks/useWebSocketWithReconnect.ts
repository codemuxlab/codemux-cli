import { useCallback, useEffect, useRef, useState } from "react";

export interface WebSocketConfig {
	url: string;
	protocols?: string | string[];
	maxReconnectAttempts?: number;
	baseDelay?: number; // Base delay in milliseconds
	maxDelay?: number; // Maximum delay in milliseconds
	backoffFactor?: number; // Exponential backoff multiplier
	onOpen?: () => void;
	onMessage?: (event: MessageEvent) => void;
	onClose?: (event: CloseEvent) => void;
	onError?: (event: Event) => void;
	onReconnectAttempt?: (attempt: number, delay: number) => void;
}

export interface WebSocketState {
	socket: WebSocket | null;
	isConnected: boolean;
	isReconnecting: boolean;
	reconnectAttempt: number;
	nextReconnectIn: number;
	error: Event | null;
}

export function useWebSocketWithReconnect(
	config: WebSocketConfig,
): WebSocketState & {
	send: (data: string) => void;
	close: () => void;
	reconnect: () => void;
} {
	const {
		url,
		protocols,
		maxReconnectAttempts = 5,
		baseDelay = 5000,
		maxDelay = 30000,
		backoffFactor = 2,
		onOpen,
		onMessage,
		onClose,
		onError,
		onReconnectAttempt,
	} = config;

	const [state, setState] = useState<WebSocketState>({
		socket: null,
		isConnected: false,
		isReconnecting: false,
		reconnectAttempt: 0,
		nextReconnectIn: 0,
		error: null,
	});

	const socketRef = useRef<WebSocket | null>(null);
	const reconnectTimeoutRef = useRef<NodeJS.Timeout | null>(null);
	const countdownIntervalRef = useRef<NodeJS.Timeout | null>(null);
	const shouldReconnectRef = useRef(true);
	const reconnectAttemptRef = useRef(0);

	// Use refs to avoid recreating callbacks causing reconnections
	const callbacksRef = useRef({
		onOpen,
		onMessage,
		onClose,
		onError,
		onReconnectAttempt,
	});

	// Update callbacks ref when they change
	callbacksRef.current = {
		onOpen,
		onMessage,
		onClose,
		onError,
		onReconnectAttempt,
	};

	// Calculate exponential backoff delay
	const calculateDelay = useCallback(
		(attempt: number): number => {
			const delay = Math.min(baseDelay * backoffFactor ** attempt, maxDelay);
			// Add some jitter to prevent thundering herd
			return delay + Math.random() * 1000;
		},
		[baseDelay, backoffFactor, maxDelay],
	);

	// Clear reconnect timeout and countdown
	const clearReconnectTimer = useCallback(() => {
		if (reconnectTimeoutRef.current) {
			clearTimeout(reconnectTimeoutRef.current);
			reconnectTimeoutRef.current = null;
		}
		if (countdownIntervalRef.current) {
			clearInterval(countdownIntervalRef.current);
			countdownIntervalRef.current = null;
		}
	}, []);

	// Create WebSocket connection
	const createConnection = useCallback(() => {
		console.log(`[WebSocket] Connecting to ${url}...`);

		// Close existing socket if it exists
		if (socketRef.current) {
			socketRef.current.close();
			socketRef.current = null;
		}

		try {
			const socket = new WebSocket(url, protocols);
			socketRef.current = socket;

			socket.onopen = (_event) => {
				console.log("[WebSocket] Connected");
				reconnectAttemptRef.current = 0;
				clearReconnectTimer();

				setState((prev) => ({
					...prev,
					socket,
					isConnected: true,
					isReconnecting: false,
					reconnectAttempt: 0,
					nextReconnectIn: 0,
					error: null,
				}));

				callbacksRef.current.onOpen?.();
			};

			socket.onmessage = (event) => {
				callbacksRef.current.onMessage?.(event);
			};

			socket.onclose = (event) => {
				console.log("[WebSocket] Disconnected", event.code, event.reason);

				setState((prev) => ({
					...prev,
					socket: null,
					isConnected: false,
				}));

				callbacksRef.current.onClose?.(event);

				// Only attempt reconnection if it wasn't a clean close and we should reconnect
				if (
					shouldReconnectRef.current &&
					event.code !== 1000 &&
					reconnectAttemptRef.current < maxReconnectAttempts
				) {
					// Schedule reconnection with exponential backoff
					const attempt = reconnectAttemptRef.current;
					const delay = calculateDelay(attempt);

					console.log(
						`[WebSocket] Scheduling reconnect attempt ${attempt + 1}/${maxReconnectAttempts} in ${Math.round(delay / 1000)}s`,
					);

					reconnectAttemptRef.current++;

					setState((prev) => ({
						...prev,
						isReconnecting: true,
						reconnectAttempt: attempt + 1,
						nextReconnectIn: Math.round(delay / 1000),
					}));

					callbacksRef.current.onReconnectAttempt?.(attempt + 1, delay);

					// Start countdown
					let remainingTime = Math.round(delay / 1000);
					countdownIntervalRef.current = setInterval(() => {
						remainingTime--;
						setState((prev) => ({
							...prev,
							nextReconnectIn: remainingTime,
						}));

						if (remainingTime <= 0) {
							if (countdownIntervalRef.current) {
								clearInterval(countdownIntervalRef.current);
								countdownIntervalRef.current = null;
							}
						}
					}, 1000);

					// Schedule actual reconnection
					reconnectTimeoutRef.current = setTimeout(() => {
						// Clear the countdown interval when reconnecting
						if (countdownIntervalRef.current) {
							clearInterval(countdownIntervalRef.current);
							countdownIntervalRef.current = null;
						}

						if (shouldReconnectRef.current) {
							createConnection();
						}
					}, delay);
				} else if (reconnectAttemptRef.current >= maxReconnectAttempts) {
					console.log("[WebSocket] Max reconnect attempts reached");
					setState((prev) => ({
						...prev,
						isReconnecting: false,
						nextReconnectIn: 0,
					}));
				}
			};

			socket.onerror = (event) => {
				console.error("[WebSocket] Error:", event);

				setState((prev) => ({
					...prev,
					error: event,
				}));

				callbacksRef.current.onError?.(event);
			};
		} catch (error) {
			console.error("[WebSocket] Failed to create socket:", error);
		}
	}, [
		url,
		protocols,
		maxReconnectAttempts,
		clearReconnectTimer,
		calculateDelay,
	]);

	// Send message
	const send = useCallback((data: string) => {
		if (socketRef.current && socketRef.current.readyState === WebSocket.OPEN) {
			socketRef.current.send(data);
		} else {
			console.warn("[WebSocket] Cannot send message - socket not connected");
		}
	}, []);

	// Close connection
	const close = useCallback(() => {
		shouldReconnectRef.current = false;
		clearReconnectTimer();

		if (socketRef.current) {
			socketRef.current.close(1000, "Client requested closure");
			socketRef.current = null;
		}

		setState((prev) => ({
			...prev,
			socket: null,
			isConnected: false,
			isReconnecting: false,
			reconnectAttempt: 0,
			nextReconnectIn: 0,
		}));
	}, [clearReconnectTimer]);

	// Manual reconnect
	const reconnect = useCallback(() => {
		if (socketRef.current) {
			socketRef.current.close();
		}
		reconnectAttemptRef.current = 0;
		shouldReconnectRef.current = true;
		clearReconnectTimer();

		// Reset reconnecting state
		setState((prev) => ({
			...prev,
			isReconnecting: false,
			reconnectAttempt: 0,
			nextReconnectIn: 0,
		}));

		createConnection();
	}, [createConnection, clearReconnectTimer]);

	// Initial connection
	useEffect(() => {
		shouldReconnectRef.current = true;
		createConnection();

		return () => {
			shouldReconnectRef.current = false;
			clearReconnectTimer();
			if (socketRef.current) {
				socketRef.current.close();
			}
		};
	}, [createConnection, clearReconnectTimer]);

	return {
		...state,
		send,
		close,
		reconnect,
	};
}
