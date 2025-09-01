use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "expo-app/dist"]
pub struct ReactAssets;
