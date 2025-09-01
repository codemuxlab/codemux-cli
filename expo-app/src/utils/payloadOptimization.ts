import type { GridCell } from "../stores/terminalStore";

// Utility to remove null, false, and undefined values from objects
export function optimizeGridCell(cell: GridCell): Partial<GridCell> {
	const optimized: Partial<GridCell> = {};

	// Only include char if it's not a space
	if (cell.char && cell.char !== " ") {
		optimized.char = cell.char;
	}

	// Only include colors if they exist
	if (cell.fg_color) {
		optimized.fg_color = cell.fg_color;
	}

	if (cell.bg_color) {
		optimized.bg_color = cell.bg_color;
	}

	// Only include style flags if they're true
	if (cell.bold) {
		optimized.bold = true;
	}

	if (cell.italic) {
		optimized.italic = true;
	}

	if (cell.underline) {
		optimized.underline = true;
	}

	if (cell.reverse) {
		optimized.reverse = true;
	}

	return optimized;
}

// Reconstruct a full GridCell from optimized payload, filling in defaults
export function reconstructGridCell(optimized: Partial<GridCell>): GridCell {
	return {
		char: optimized.char ?? " ",
		fg_color: optimized.fg_color ?? null,
		bg_color: optimized.bg_color ?? null,
		bold: optimized.bold ?? false,
		italic: optimized.italic ?? false,
		underline: optimized.underline ?? false,
		reverse: optimized.reverse ?? false,
	};
}

// Generic function to remove falsy values from any object
export function omitFalsyValues<T extends Record<string, unknown>>(
	obj: T,
): Partial<T> {
	const result: Partial<T> = {};

	for (const [key, value] of Object.entries(obj)) {
		if (
			value !== null &&
			value !== false &&
			value !== undefined &&
			value !== ""
		) {
			result[key as keyof T] = value as T[keyof T];
		}
	}

	return result;
}
