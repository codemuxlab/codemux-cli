use serde::{Deserialize, Serialize};
use super::{PtyInputMessage, PtyOutputMessage, GridUpdateMessage};

/// Messages sent from client to server
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ClientMessage {
    #[serde(rename = "input")]
    Input { data: PtyInputMessage },
    #[serde(rename = "resize")]
    Resize { rows: u16, cols: u16 },
    #[serde(rename = "request_keyframe")]
    RequestKeyframe,
}

/// Messages sent from server to client
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ServerMessage {
    #[serde(rename = "output")]
    Output { data: PtyOutputMessage },
    #[serde(rename = "grid")]
    Grid { data: GridUpdateMessage },
    #[serde(rename = "pty_size")]
    PtySize { rows: u16, cols: u16 },
    #[serde(rename = "error")]
    Error { message: String },
}