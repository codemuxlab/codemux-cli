// JSON API v1.0 specification types - re-export generated types from Rust
export type {
	JsonApiDocument,
	JsonApiError,
	JsonApiErrorDocument,
	JsonApiRelationship,
	JsonApiResource,
	JsonApiResourceIdentifier,
} from "./bindings";

// Additional JSON API interfaces not generated from Rust
export interface JsonApiLinks {
	self?: string;
	related?: string;
	first?: string;
	last?: string;
	prev?: string;
	next?: string;
}

// Import the types for use in function signatures
import type {
	JsonApiDocument as ImportedJsonApiDocument,
	JsonApiError as ImportedJsonApiError,
	JsonApiErrorDocument as ImportedJsonApiErrorDocument,
	JsonApiResource as ImportedJsonApiResource,
} from "./bindings";

// Type guards
export function isJsonApiDocument(
	obj: unknown,
): obj is ImportedJsonApiDocument<unknown> {
	return obj !== null && typeof obj === "object" && "data" in obj;
}

export function isJsonApiResource<T = unknown>(
	obj: unknown,
): obj is ImportedJsonApiResource<T> {
	return (
		obj !== null &&
		typeof obj === "object" &&
		"type" in obj &&
		"id" in obj &&
		typeof (obj as ImportedJsonApiResource<T>).type === "string" &&
		typeof (obj as ImportedJsonApiResource<T>).id === "string"
	);
}

export function isJsonApiError(obj: unknown): obj is ImportedJsonApiError {
	return (
		obj !== null &&
		typeof obj === "object" &&
		("status" in obj || "title" in obj || "detail" in obj)
	);
}

// Simple extraction that just returns the document data as-is
export function extractFromDocument<T>(
	document: ImportedJsonApiDocument<T>,
): T {
	return document.data;
}

// Helper to build JSON API error response
export function buildJsonApiError(
	status: string,
	title: string,
	detail?: string,
): ImportedJsonApiErrorDocument {
	return {
		errors: [
			{
				status,
				title,
				detail,
			},
		],
	};
}
