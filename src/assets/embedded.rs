use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "app/dist"]
pub struct ReactAssets;
