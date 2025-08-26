import React, { memo } from 'react';
import { Text } from 'react-native';
import { useTerminalStore, GridCell } from '../stores/terminalStore';

interface TerminalCellProps {
  row: number;
  col: number;
}

// Memoized cell component that only re-renders when its specific data changes
export const TerminalCell = memo(({ row, col }: TerminalCellProps) => {
  const cellKey = `${row}-${col}`;
  
  // Subscribe only to this specific cell (cursor is now part of cell data)
  const cell = useTerminalStore((state) => state.cells.get(cellKey));
  
  // Log first few cells to debug rendering
  if (row === 0 && col < 3 && cell) {
    console.log(`Rendering cell [${row},${col}]:`, cell);
  }
  
  const char = cell?.char || ' ';
  
  const getForegroundColor = (): string => {
    if (cell?.reverse) {
      return cell?.bg_color || '#000000';
    }
    return cell?.fg_color || '#c9d1d9';
  };
  
  const getBackgroundColor = (): string => {
    if (cell?.has_cursor) {
      return '#58a6ff';
    }
    if (cell?.reverse) {
      return cell?.fg_color || '#c9d1d9';
    }
    // Add a subtle background for debugging - empty cells should be barely visible
    return cell?.bg_color || (char === ' ' ? '#111111' : 'transparent');
  };
  
  const style = {
    color: getForegroundColor(),
    backgroundColor: getBackgroundColor(),
    fontWeight: cell?.bold ? 'bold' as const : 'normal' as const,
    fontStyle: cell?.italic ? 'italic' as const : 'normal' as const,
    textDecorationLine: cell?.underline ? 'underline' as const : 'none' as const,
    fontFamily: 'monospace',
    fontSize: 14,
    lineHeight: 20,
    minWidth: 9,
    textAlign: 'center' as const,
  };
  
  return (
    <Text style={style}>
      {char}
    </Text>
  );
});

TerminalCell.displayName = 'TerminalCell';