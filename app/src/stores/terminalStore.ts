import { create } from 'zustand';
import { subscribeWithSelector } from 'zustand/middleware';

export type TerminalColor = 
  | 'Default'
  | { Indexed: number }
  | { Palette: number }
  | { Rgb: { r: number; g: number; b: number } };

export interface GridCell {
  char: string;
  fg_color: TerminalColor | null;
  bg_color: TerminalColor | null;
  bold: boolean;
  italic: boolean;
  underline: boolean;
  reverse: boolean;
  has_cursor?: boolean;
}

export interface TerminalTheme {
  name: string;
  background: string;
  foreground: string;
  cursor: string;
  selection: string;
  colors: string[]; // 16 standard colors (0-15)
}

// Default dark theme based on VS Code
export const defaultTheme: TerminalTheme = {
  name: 'Default Dark',
  background: '#0d1117',
  foreground: '#c9d1d9',
  cursor: '#58a6ff',
  selection: '#264f78',
  colors: [
    '#000000', '#cd3131', '#0dbc79', '#e5e510',
    '#2472c8', '#bc3fbc', '#11a8cd', '#e5e5e5',
    '#666666', '#f14c4c', '#23d18b', '#f5f543',
    '#3b8eea', '#d670d6', '#29b8db', '#ffffff',
  ]
};

// Additional themes
export const lightTheme: TerminalTheme = {
  name: 'Light',
  background: '#ffffff',
  foreground: '#1f2328',
  cursor: '#0969da',
  selection: '#b6d7ff',
  colors: [
    '#24292f', '#cf222e', '#116329', '#4d2d00',
    '#0969da', '#8250df', '#1b7c83', '#656d76',
    '#8c959f', '#ff6b6b', '#2da44e', '#fb8500',
    '#0969da', '#8250df', '#3192aa', '#1f2328',
  ]
};

export const monochromeTheme: TerminalTheme = {
  name: 'Monochrome',
  background: '#1a1a1a',
  foreground: '#c0c0c0',
  cursor: '#ffffff',
  selection: '#404040',
  colors: [
    '#000000', '#808080', '#a0a0a0', '#c0c0c0',
    '#606060', '#909090', '#b0b0b0', '#d0d0d0',
    '#404040', '#888888', '#a8a8a8', '#c8c8c8',
    '#707070', '#989898', '#b8b8b8', '#ffffff',
  ]
};

export const availableThemes = [defaultTheme, lightTheme, monochromeTheme];

// Web key event types (matching backend)
export interface WebKeyModifiers {
  shift: boolean;
  ctrl: boolean;
  alt: boolean;
  meta: boolean;
}

export type WebKeyCode = 
  | { Char: string }
  | 'Backspace'
  | 'Enter'
  | 'Left'
  | 'Right'
  | 'Up'
  | 'Down'
  | 'Home'
  | 'End'
  | 'PageUp'
  | 'PageDown'
  | 'Tab'
  | 'Delete'
  | 'Insert'
  | { F: number }
  | 'Esc';

export interface WebKeyEvent {
  code: WebKeyCode;
  modifiers: WebKeyModifiers;
}

interface TerminalState {
  size: { rows: number; cols: number };
  cells: Map<string, GridCell>;
  cursor: { row: number; col: number };
  cursor_visible: boolean;
  theme: TerminalTheme;
  
  // Actions
  updateSize: (rows: number, cols: number) => void;
  updateCell: (row: number, col: number, cell: GridCell) => void;
  updateCells: (updates: Array<[number, number, GridCell]>) => void;
  updateCursor: (row: number, col: number) => void;
  setCursorVisible: (visible: boolean) => void;
  clearCells: () => void;
  handleGridUpdate: (message: any) => void;
  setTheme: (theme: TerminalTheme) => void;
  resolveColor: (color: TerminalColor | null, isBackground?: boolean) => string;
}

export const useTerminalStore = create<TerminalState>()(
  subscribeWithSelector((set, get) => ({
  size: { rows: 24, cols: 80 },
  cells: new Map(),
  cursor: { row: 0, col: 0 },
  cursor_visible: true,
  theme: defaultTheme,
  
  updateSize: (rows, cols) => set(() => ({
    size: { rows, cols }
  })),
  
  updateCell: (row, col, cell) => set((state) => {
    const newCells = new Map(state.cells);
    newCells.set(`${row}-${col}`, cell);
    return { cells: newCells };
  }),
  
  updateCells: (updates) => set((state) => {
    const newCells = new Map(state.cells);
    updates.forEach(([row, col, cell]) => {
      newCells.set(`${row}-${col}`, cell);
    });
    return { cells: newCells };
  }),
  
  updateCursor: (row, col) => set((state) => {
    const newCells = new Map(state.cells);
    
    // Clear old cursor position
    const oldCursorKey = `${state.cursor.row}-${state.cursor.col}`;
    const oldCell = newCells.get(oldCursorKey);
    if (oldCell && oldCell.has_cursor) {
      newCells.set(oldCursorKey, { ...oldCell, has_cursor: false });
    }
    
    // Set new cursor position
    const newCursorKey = `${row}-${col}`;
    const newCell = newCells.get(newCursorKey) || {
      char: ' ',
      fg_color: null,
      bg_color: null,
      bold: false,
      italic: false,
      underline: false,
      reverse: false,
    };
    
    if (state.cursor_visible) {
      newCells.set(newCursorKey, { ...newCell, has_cursor: true });
    }
    
    return {
      cursor: { row, col },
      cells: newCells
    };
  }),
  
  setCursorVisible: (visible) => set((state) => {
    const newCells = new Map(state.cells);
    const cursorKey = `${state.cursor.row}-${state.cursor.col}`;
    const cursorCell = newCells.get(cursorKey);
    
    if (cursorCell) {
      newCells.set(cursorKey, { ...cursorCell, has_cursor: visible });
    }
    
    return {
      cursor_visible: visible,
      cells: newCells
    };
  }),
  
  clearCells: () => set(() => ({
    cells: new Map()
  })),
  
  handleGridUpdate: (message) => set((state) => {
    console.log('Store handleGridUpdate called:', message);
    const updates: Partial<TerminalState> = {};
    let hasChanges = false;
    
    // Update size if provided and different
    if (message.size && (message.size.rows !== state.size.rows || message.size.cols !== state.size.cols)) {
      console.log('Updating size from', state.size, 'to', message.size);
      updates.size = message.size;
      hasChanges = true;
    }
    
    // Apply cell changes
    if (message.cells && message.cells.length > 0) {
      const newCells = new Map(state.cells);
      console.log('Processing', message.cells.length, 'cell updates');
      
      message.cells.forEach(([row, col, cell]: [number, number, any]) => {
        // Reconstruct full GridCell with defaults for omitted values
        const fullCell: GridCell = {
          char: cell.char ?? ' ',
          fg_color: cell.fg_color ?? null,
          bg_color: cell.bg_color ?? null,
          bold: cell.bold ?? false,
          italic: cell.italic ?? false,
          underline: cell.underline ?? false,
          reverse: cell.reverse ?? false,
        };
        
        
        newCells.set(`${row}-${col}`, fullCell);
      });
      
      console.log('Total cells in map after update:', newCells.size);
      updates.cells = newCells;
      hasChanges = true;
    }
    
    // Update cursor position if different
    if (message.cursor && (message.cursor.row !== state.cursor.row || message.cursor.col !== state.cursor.col)) {
      // Ensure we have a newCells map to work with
      const cellsToUpdate = updates.cells || new Map(state.cells);
      
      // Clear old cursor from cells
      const oldCursorKey = `${state.cursor.row}-${state.cursor.col}`;
      const oldCell = cellsToUpdate.get(oldCursorKey);
      if (oldCell && oldCell.has_cursor) {
        cellsToUpdate.set(oldCursorKey, { ...oldCell, has_cursor: false });
      }
      
      // Set new cursor in cells
      const newCursorKey = `${message.cursor.row}-${message.cursor.col}`;
      const newCursorCell = cellsToUpdate.get(newCursorKey) || {
        char: ' ',
        fg_color: null,
        bg_color: null,
        bold: false,
        italic: false,
        underline: false,
        reverse: false,
      };
      
      // Only add cursor if it's visible (check both current state and message)
      const cursorVisible = message.cursor_visible ?? state.cursor_visible;
      if (cursorVisible) {
        cellsToUpdate.set(newCursorKey, { ...newCursorCell, has_cursor: true });
      }
      
      updates.cursor = { row: message.cursor.row, col: message.cursor.col };
      updates.cells = cellsToUpdate;
      hasChanges = true;
    }
    
    // Update cursor visibility if different
    if (typeof message.cursor_visible === 'boolean' && message.cursor_visible !== state.cursor_visible) {
      // Ensure we have a cells map to work with
      const cellsToUpdate = updates.cells || new Map(state.cells);
      
      // Update cursor visibility on current cursor cell
      const cursorKey = `${state.cursor.row}-${state.cursor.col}`;
      const cursorCell = cellsToUpdate.get(cursorKey);
      
      if (cursorCell) {
        if (message.cursor_visible) {
          // Show cursor
          cellsToUpdate.set(cursorKey, { ...cursorCell, has_cursor: true });
        } else {
          // Hide cursor
          cellsToUpdate.set(cursorKey, { ...cursorCell, has_cursor: false });
        }
      }
      
      updates.cursor_visible = message.cursor_visible;
      updates.cells = cellsToUpdate;
      hasChanges = true;
    }
    
    // Only return updates if there are actual changes
    return hasChanges ? updates : {};
  }),

  setTheme: (theme) => set(() => ({
    theme
  })),

  resolveColor: (color, isBackground = false) => {
    const state = get();
    
    if (!color) {
      return isBackground ? state.theme.background : state.theme.foreground;
    }

    // Handle different color variant structures
    if (typeof color === 'string' && color === 'Default') {
      return isBackground ? state.theme.background : state.theme.foreground;
    }

    if (typeof color === 'object') {
      // Handle Indexed variant
      if ('Indexed' in color && typeof color.Indexed === 'number') {
        const index = color.Indexed;
        if (index >= 0 && index < state.theme.colors.length) {
          return state.theme.colors[index];
        }
        return isBackground ? state.theme.background : state.theme.foreground;
      }

      // Handle Palette variant
      if ('Palette' in color && typeof color.Palette === 'number') {
        const index = color.Palette;
        // For 8-bit palette colors (16-255), use a simplified mapping
        if (index < 16) {
          return state.theme.colors[index] || (isBackground ? state.theme.background : state.theme.foreground);
        } else if (index < 232) {
          // 216 color cube (6x6x6)
          const n = index - 16;
          const r = Math.floor(n / 36) * 51;
          const g = Math.floor((n % 36) / 6) * 51;
          const b = (n % 6) * 51;
          return `rgb(${r}, ${g}, ${b})`;
        } else {
          // Grayscale ramp (24 levels)
          const level = Math.floor((index - 232) * 255 / 23);
          return `rgb(${level}, ${level}, ${level})`;
        }
      }

      // Handle Rgb variant
      if ('Rgb' in color && typeof color.Rgb === 'object') {
        const { r, g, b } = color.Rgb;
        return `rgb(${r}, ${g}, ${b})`;
      }
    }

    return isBackground ? state.theme.background : state.theme.foreground;
  }
})))