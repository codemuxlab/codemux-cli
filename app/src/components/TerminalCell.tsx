import React, { memo } from 'react';
import { Text } from 'react-native';
import { useTerminalStore, GridCell, TerminalColor } from '../stores/terminalStore';

interface TerminalCellProps {
  row: number;
  col: number;
}

// Memoized cell component that only re-renders when its specific data changes
export const TerminalCell = memo(({ row, col }: TerminalCellProps) => {
  const cellKey = `${row}-${col}`;
  
  // Subscribe to both the cell data and the theme resolver
  const cell = useTerminalStore((state) => state.cells.get(cellKey));
  const resolveColor = useTerminalStore((state) => state.resolveColor);
  const theme = useTerminalStore((state) => state.theme);
  
  
  const char = cell?.char || ' ';
  
  const getForegroundColor = (): string => {
    if (cell?.reverse) {
      return resolveColor(cell?.bg_color, true);
    }
    return resolveColor(cell?.fg_color, false);
  };
  
  const getBackgroundColor = (): string => {
    if (cell?.has_cursor) {
      return theme.cursor;
    }
    if (cell?.reverse) {
      return resolveColor(cell?.fg_color, false);
    }
    return resolveColor(cell?.bg_color, true);
  };
  
  // Build dynamic classes for styles that change
  const dynamicClasses = [
    cell?.bold && 'font-bold',
    cell?.italic && 'italic',
    cell?.underline && 'underline'
  ].filter(Boolean).join(' ');

  const dynamicStyle = {
    color: getForegroundColor(),
    backgroundColor: getBackgroundColor(),
  };
  
  return (
    <Text 
      className={`font-mono text-sm leading-5 text-center min-w-[9px] ${dynamicClasses}`}
      style={dynamicStyle}
    >
      {char}
    </Text>
  );
});

TerminalCell.displayName = 'TerminalCell';