import { Redirect, useLocalSearchParams } from "expo-router";

export default function SessionIndex() {
	const { sessionId } = useLocalSearchParams<{ sessionId: string }>();

	// Redirect to the terminal tab by default
	return <Redirect href={`/session/${sessionId}/terminal`} />;
}
