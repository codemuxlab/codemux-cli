use serde::{Deserialize, Serialize};
use super::{PtyInputMessage, PtyOutputMessage, GridUpdateMessage};

/// Unified WebSocket message format for client-server communication
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WebSocketMessage {
    // Client-to-Server messages
    #[serde(rename = "input")]
    Input { data: PtyInputMessage },
    #[serde(rename = "resize")]
    Resize { rows: u16, cols: u16 },
    #[serde(rename = "request_keyframe")]
    RequestKeyframe,
    
    // Server-to-Client messages
    #[serde(rename = "output")]
    Output { data: PtyOutputMessage },
    #[serde(rename = "grid")]
    Grid { data: GridUpdateMessage },
    #[serde(rename = "pty_size")]
    PtySize { rows: u16, cols: u16 },
    #[serde(rename = "error")]
    Error { message: String },
}