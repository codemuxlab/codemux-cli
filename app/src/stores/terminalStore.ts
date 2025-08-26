import { create } from 'zustand';
import { subscribeWithSelector } from 'zustand/middleware';

export interface GridCell {
  char: string;
  fg_color: string | null;
  bg_color: string | null;
  bold: boolean;
  italic: boolean;
  underline: boolean;
  reverse: boolean;
  has_cursor?: boolean;  // Add cursor as part of cell data
}

interface TerminalState {
  size: { rows: number; cols: number };
  cells: Map<string, GridCell>;
  cursor: { row: number; col: number };
  cursor_visible: boolean;
  
  // Actions
  updateSize: (rows: number, cols: number) => void;
  updateCell: (row: number, col: number, cell: GridCell) => void;
  updateCells: (updates: Array<[number, number, GridCell]>) => void;
  updateCursor: (row: number, col: number) => void;
  setCursorVisible: (visible: boolean) => void;
  clearCells: () => void;
  handleGridUpdate: (message: any) => void;
}

export const useTerminalStore = create<TerminalState>()(
  subscribeWithSelector((set, get) => ({
  size: { rows: 24, cols: 80 },
  cells: new Map(),
  cursor: { row: 0, col: 0 },
  cursor_visible: true,
  
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
        
        // Log a sample cell for debugging
        if (row === 0 && col < 5) {
          console.log(`Cell [${row},${col}]:`, fullCell);
        }
        
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
  })
})))