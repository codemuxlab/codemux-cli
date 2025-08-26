const { getDefaultConfig } = require("expo/metro-config");
const { withNativeWind } = require("nativewind/metro");

const config = getDefaultConfig(__dirname);

// Fix for Zustand import.meta issue
config.resolver = {
  ...config.resolver,
  // Prioritize CommonJS over ESM to avoid import.meta issues
  unstable_conditionNames: ['browser', 'require', 'react-native'],
};

module.exports = withNativeWind(config, { input: "./src/global.css" });
