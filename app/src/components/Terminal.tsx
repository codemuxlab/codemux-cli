import React, { useEffect, useRef, useState, memo, useCallback } from 'react';
import { View, Text, TextInput, ScrollView, TouchableOpacity } from 'react-native';
import { useTerminalStore, availableThemes } from '../stores/terminalStore';
import { TerminalCell } from './TerminalCell';

interface TerminalProps {
  sessionId: string;
}

// Memoized row component to prevent unnecessary re-renders
const TerminalRow = memo(({ row, cols }: { row: number; cols: number }) => {
  const cells = [];
  
  for (let col = 0; col < cols; col++) {
    cells.push(
      <TerminalCell key={`${row}-${col}`} row={row} col={col} />
    );
  }
  
  return (
    <View className="flex-row">
      {cells}
    </View>
  );
});

TerminalRow.displayName = 'TerminalRow';

// Separate component for the terminal grid to isolate re-renders
const TerminalGrid = memo(() => {
  // Only subscribe to size changes
  const size = useTerminalStore((state) => state.size);
  const cellCount = useTerminalStore((state) => state.cells.size);
  
  console.log(`TerminalGrid rendering: ${size.rows}x${size.cols}, ${cellCount} cells in store`);
  
  const rows = [];
  for (let row = 0; row < size.rows; row++) {
    rows.push(
      <TerminalRow key={row} row={row} cols={size.cols} />
    );
  }
  
  return (
    <View>
      {rows}
    </View>
  );
});

TerminalGrid.displayName = 'TerminalGrid';

// Separate input component to isolate input state changes
const TerminalInput = memo(({ onSubmit }: { onSubmit: (text: string) => void }) => {
  const [inputValue, setInputValue] = useState('');
  
  const handleSubmit = useCallback(() => {
    if (inputValue.trim()) {
      onSubmit(inputValue);
      setInputValue('');
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
});

TerminalInput.displayName = 'TerminalInput';

// Background component with theme support
const TerminalBackground = memo(({ children }: { children: React.ReactNode }) => {
  const theme = useTerminalStore((state) => state.theme);
  
  return (
    <View className="flex-1 w-full" style={{ backgroundColor: theme.background }}>
      {children}
    </View>
  );
});

TerminalBackground.displayName = 'TerminalBackground';

// Dark/Light mode toggle component
const DarkLightToggle = memo(() => {
  const currentTheme = useTerminalStore((state) => state.theme);
  const setTheme = useTerminalStore((state) => state.setTheme);
  
  const isDark = currentTheme.name === 'Default Dark';
  
  const toggleMode = () => {
    if (isDark) {
      setTheme(availableThemes.find(t => t.name === 'Light') || availableThemes[1]);
    } else {
      setTheme(availableThemes.find(t => t.name === 'Default Dark') || availableThemes[0]);
    }
  };

  return (
    <TouchableOpacity 
      onPress={toggleMode}
      className="bg-gray-700 px-3 py-1 rounded mr-2"
    >
      <Text className="text-white text-xs">
        {isDark ? '☀️ Light' : '🌙 Dark'}
      </Text>
    </TouchableOpacity>
  );
});

DarkLightToggle.displayName = 'DarkLightToggle';

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
              <Text className={`text-xs ${currentTheme.name === theme.name ? 'text-blue-400 font-bold' : 'text-white'}`}>
                {theme.name}
              </Text>
            </TouchableOpacity>
          ))}
        </View>
      )}
    </View>
  );
});

ThemeSelector.displayName = 'ThemeSelector';

export default function Terminal({ sessionId }: TerminalProps) {
  const [isConnected, setIsConnected] = useState(false);
  const wsRef = useRef<WebSocket | null>(null);
  const scrollViewRef = useRef<ScrollView>(null);

  useEffect(() => {
    // Connect to WebSocket
    const wsUrl = `ws://localhost:8765/ws/${sessionId}`;
    const ws = new WebSocket(wsUrl);
    wsRef.current = ws;

    ws.onopen = () => {
      setIsConnected(true);
      console.log('WebSocket connected');
      
      // Request initial keyframe to get current terminal state
      ws.send(JSON.stringify({
        type: 'request_keyframe'
      }));
    };

    ws.onmessage = (event) => {
      try {
        const message = JSON.parse(event.data);
        handleWebSocketMessage(message);
      } catch (error) {
        console.error('Failed to parse WebSocket message:', error);
      }
    };

    ws.onclose = () => {
      setIsConnected(false);
      console.log('WebSocket disconnected');
    };

    ws.onerror = (error) => {
      console.error('WebSocket error:', error);
    };

    return () => {
      if (ws.readyState === WebSocket.OPEN) {
        ws.close();
      }
    };
  }, [sessionId]);

  const handleWebSocketMessage = useCallback((message: any) => {
    console.log('WebSocket message received:', message.type, message);
    
    switch (message.type) {
      case 'grid_update':
        console.log('Grid update:', {
          type: message.update_type,
          cellCount: message.cells?.length,
          size: message.size,
          cursor: message.cursor,
          cursor_visible: message.cursor_visible,
        });
        
        // Log first few cells for debugging
        if (message.cells?.length > 0) {
          console.log('Sample cells:', message.cells.slice(0, 5));
        }
        
        // Call the store action directly without subscribing
        useTerminalStore.getState().handleGridUpdate(message);
        break;
      case 'pty_size':
        console.log('PTY size update:', message.rows, 'x', message.cols);
        useTerminalStore.getState().updateSize(message.rows, message.cols);
        break;
      case 'output':
        // Handle legacy output messages - these are raw terminal output
        // We can log them for debugging but grid_update is the primary channel
        console.log('Received raw output:', message.content);
        break;
      default:
        console.log('Unknown message type:', message.type, message);
    }
  }, []);

  const sendInput = useCallback((data: string) => {
    if (wsRef.current && wsRef.current.readyState === WebSocket.OPEN) {
      wsRef.current.send(JSON.stringify({
        type: 'input',
        data: data
      }));
    }
  }, []);

  const handleInputSubmit = useCallback((text: string) => {
    // Send the message text
    sendInput(text);
    // Send carriage return to submit
    sendInput('\r');
  }, [sendInput]);

  return (
    <View className="flex-1 bg-black">
      {/* Connection status and theme controls */}
      <View className={`p-2 flex-row justify-between items-center ${isConnected ? 'bg-green-700' : 'bg-red-700'}`}>
        <Text className="text-white text-xs">
          {isConnected ? `Connected to session ${sessionId.slice(0, 8)}` : 'Disconnected'}
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
            justifyContent: 'center',
            alignItems: 'center',
            padding: 16,
            minHeight: '100%'
          }}
        >
          <TerminalGrid />
        </ScrollView>
      </TerminalBackground>

      {/* Input area */}
      <TerminalInput onSubmit={handleInputSubmit} />
    </View>
  );
}