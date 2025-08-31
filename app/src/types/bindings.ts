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
// JSON API types
export type { JsonApiDocument } from "../../../bindings/JsonApiDocument";
export type { JsonApiError } from "../../../bindings/JsonApiError";
export type { JsonApiErrorDocument } from "../../../bindings/JsonApiErrorDocument";
export type { JsonApiRelationship } from "../../../bindings/JsonApiRelationship";
export type { JsonApiResource } from "../../../bindings/JsonApiResource";
export type { JsonApiResourceIdentifier } from "../../../bindings/JsonApiResourceIdentifier";
export type { KeyCode, KeyCode as WebKeyCode } from "../../../bindings/KeyCode";
export type {
	KeyEvent,
	KeyEvent as WebKeyEvent,
} from "../../../bindings/KeyEvent";
export type {
	KeyModifiers,
	KeyModifiers as WebKeyModifiers,
} from "../../../bindings/KeyModifiers";
export type { ProjectAttributes } from "../../../bindings/ProjectAttributes";
export type { ProjectInfo } from "../../../bindings/ProjectInfo";
export type { ProjectListResponse } from "../../../bindings/ProjectListResponse";
export type { ProjectRelationships } from "../../../bindings/ProjectRelationships";
export type { ProjectResourceTS } from "../../../bindings/ProjectResourceTS";
export type { ProjectWithSessions } from "../../../bindings/ProjectWithSessions";
export type { ScrollDirection } from "../../../bindings/ScrollDirection";
export type { SerializablePtySize } from "../../../bindings/SerializablePtySize";
export type { ServerMessage } from "../../../bindings/ServerMessage";
export type { SessionAttributes } from "../../../bindings/SessionAttributes";
export type { SessionInfo } from "../../../bindings/SessionInfo";
export type { SessionResourceTS } from "../../../bindings/SessionResourceTS";
export type { SessionResponse } from "../../../bindings/SessionResponse";
export type { SessionType } from "../../../bindings/SessionType";
export type {
	TerminalColor,
	TerminalColor as StoreTerminalColor,
} from "../../../bindings/TerminalColor";
