// Re-export all generated TypeScript bindings from Rust
// This centralizes imports and provides a single source of truth for types

export type { ClientMessage } from "../../../bindings/ClientMessage";
// Re-export under legacy names for compatibility during transition
export type {
	GridCell,
	GridCell as ApiGridCell,
} from "../../../bindings/GridCell";
export type {
	GridUpdateMessage,
	GridUpdateMessage as ApiGridUpdateMessage,
} from "../../../bindings/GridUpdateMessage";
export type { KeyCode, KeyCode as WebKeyCode } from "../../../bindings/KeyCode";
export type {
	KeyEvent,
	KeyEvent as WebKeyEvent,
} from "../../../bindings/KeyEvent";
export type {
	KeyModifiers,
	KeyModifiers as WebKeyModifiers,
} from "../../../bindings/KeyModifiers";
export type { SerializablePtySize } from "../../../bindings/SerializablePtySize";
export type { ServerMessage } from "../../../bindings/ServerMessage";
export type {
	TerminalColor,
	TerminalColor as StoreTerminalColor,
} from "../../../bindings/TerminalColor";
