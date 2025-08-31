import { useRouter } from "expo-router";
import React from "react";
import { ScrollView, Text, TouchableOpacity, View } from "react-native";
import { LastMessage } from "../components/LastMessage";
import { useProjects } from "../hooks/api";

export default function Page() {
  const router = useRouter();
  const { data: projects = [], isLoading: loading, error } = useProjects();
  console.log(projects)

  if (loading) {
    return (
      <View className="flex-1 justify-center items-center bg-gray-900">
        <Text className="text-white text-lg">Loading projects...</Text>
      </View>
    );
  }

  if (error) {
    return (
      <View className="flex-1 justify-center items-center bg-gray-900">
        <Text className="text-red-400 text-lg mb-4">
          Failed to load projects
        </Text>
        <Text className="text-gray-400 text-sm text-center px-4">
          Check that the backend is running on port 8765
        </Text>
      </View>
    );
  }

  return (
    <View className="flex-1 bg-gray-900">
      {/* Header */}
      <View className="bg-gray-800 p-6 border-b border-gray-700">
        <Text className="text-white text-2xl font-bold mb-2">
          Codemux Projects
        </Text>
        <Text className="text-gray-400">Manage your AI coding projects</Text>
      </View>

      {/* Projects List */}
      <ScrollView className="flex-1 p-4">
        {projects.length === 0 ? (
          <View className="bg-gray-800 rounded-lg p-8 items-center">
            <Text className="text-gray-400 text-lg mb-4">
              No projects found
            </Text>
            <Text className="text-gray-500 text-sm text-center">
              Start codemux from a project directory to see it here
            </Text>
          </View>
        ) : (
          projects.map((project) => {
            console.log(project);
            const projectSessions =
              project.relationships?.recent_sessions || [];

            return (
              <View
                key={project.id}
                className="bg-gray-800 rounded-lg p-4 mb-4 border border-gray-700"
              >
                {/* Project Header */}
                <View className="flex-row justify-between items-start mb-3">
                  <View className="flex-1">
                    <Text className="text-white text-xl font-bold">
                      {project.attributes?.name || "Unknown Project"}
                    </Text>
                    <Text className="text-gray-400 text-sm mt-1">
                      {project.attributes?.path || "Unknown Path"}
                    </Text>
                  </View>
                  <View className="px-2 py-1 rounded bg-blue-900">
                    <Text className="text-blue-400 text-xs">
                      {projectSessions.length} SESSION
                      {projectSessions.length !== 1 ? "S" : ""}
                    </Text>
                  </View>
                </View>

                {/* Sessions for this project */}
                {projectSessions.length > 0 ? (
                  <View className="space-y-2">
                    {projectSessions.map((session) => (
                      <View
                        key={session.id}
                        className="bg-gray-700 rounded-lg p-3"
                      >
                        <View className="flex-row justify-between items-center mb-2">
                          <View className="flex-1">
                            <Text className="text-white text-sm font-semibold">
                              Session: {session.id}
                            </Text>
                            <View className="flex-row items-center gap-2 mt-1">
                              <Text className="text-gray-400 text-xs">
                                Agent: {session.attributes?.agent || "Unknown"}
                              </Text>
                              {session.attributes?.session_type ===
                                "Historical" && (
                                  <View className="px-1.5 py-0.5 rounded bg-amber-900">
                                    <Text className="text-amber-400 text-xs">
                                      HISTORICAL
                                    </Text>
                                  </View>
                                )}
                            </View>
                          </View>
                          <View
                            className={`px-2 py-1 rounded ${session.attributes?.status === "running"
                                ? "bg-green-900"
                                : session.attributes?.session_type === "Active"
                                  ? "bg-blue-900"
                                  : "bg-gray-600"
                              }`}
                          >
                            <Text
                              className={`text-xs ${session.attributes?.status === "running"
                                  ? "text-green-400"
                                  : session.attributes?.session_type ===
                                    "Active"
                                    ? "text-blue-400"
                                    : "text-gray-400"
                                }`}
                            >
                              {session.attributes?.status?.toUpperCase() ||
                                "UNKNOWN"}
                            </Text>
                          </View>
                        </View>

                        {/* Last Message */}
                        <LastMessage
                          message={session.attributes?.last_message}
                          agent={session.attributes?.agent}
                        />

                        <View className="flex-row gap-4">
                          <TouchableOpacity
                            key={`${session.id}-terminal`}
                            onPress={() =>
                              router.push(`/session/${session.id}/terminal`)
                            }
                          >
                            <Text className="text-blue-400 text-xs">
                              🖥️ Terminal
                            </Text>
                          </TouchableOpacity>
                          <TouchableOpacity
                            key={`${session.id}-diff`}
                            onPress={() =>
                              router.push(`/session/${session.id}/diff`)
                            }
                          >
                            <Text className="text-blue-400 text-xs">
                              📋 Changes
                            </Text>
                          </TouchableOpacity>
                          <TouchableOpacity
                            key={`${session.id}-logs`}
                            onPress={() =>
                              router.push(`/session/${session.id}/logs`)
                            }
                          >
                            <Text className="text-blue-400 text-xs">
                              📜 Logs
                            </Text>
                          </TouchableOpacity>
                        </View>
                      </View>
                    ))}
                  </View>
                ) : (
                  <View className="bg-gray-700 rounded-lg p-3 text-center">
                    <Text className="text-gray-400 text-sm">
                      No active sessions for this project
                    </Text>
                  </View>
                )}
              </View>
            );
          })
        )}
      </ScrollView>
    </View>
  );
}
