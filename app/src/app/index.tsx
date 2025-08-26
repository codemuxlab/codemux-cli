import React, { useEffect, useState } from "react";
import { View } from "react-native";
import Terminal from "../components/Terminal";

export default function Page() {
  const [sessionId, setSessionId] = useState<string>('');

  useEffect(() => {
    // Extract session ID from URL params or generate one
    if (typeof window !== 'undefined') {
      const urlParams = new URLSearchParams(window.location.search);
      const sessionFromUrl = urlParams.get('session');
      
      if (sessionFromUrl) {
        setSessionId(sessionFromUrl);
      } else {
        // Generate a default session ID for testing
        setSessionId('test-session-' + Math.random().toString(36).substr(2, 9));
      }
    }
  }, []);

  if (!sessionId) {
    return (
      <View style={{ flex: 1, justifyContent: 'center', alignItems: 'center', backgroundColor: '#000' }}>
        {/* Loading state */}
      </View>
    );
  }

  return (
    <View className="flex-1 w-full">
      <Terminal sessionId={sessionId} />
    </View>
  );
}
