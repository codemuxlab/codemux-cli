use super::GridUpdateMessage;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Messages sent from client to server
#[derive(Debug, Serialize, Deserialize, TS)]
#[serde(tag = "type")]
#[ts(export)]
pub enum ClientMessage {
    #[serde(rename = "key")]
    Key {
        code: crate::core::pty_session::KeyCode,
        modifiers: crate::core::pty_session::KeyModifiers,
    },
    #[serde(rename = "resize")]
    Resize { rows: u16, cols: u16 },
    #[serde(rename = "scroll")]
    Scroll {
        direction: crate::core::pty_session::ScrollDirection,
        lines: u16,
    },
}

/// Messages sent from server to client - flattened to match frontend expectations
#[derive(Debug, Serialize, Deserialize, TS)]
#[serde(tag = "type")]
#[ts(export)]
pub enum ServerMessage {
    #[serde(rename = "output")]
    Output {
        data: Vec<u8>,
        #[ts(type = "string")]
        timestamp: std::time::SystemTime,
    },
    #[serde(rename = "grid_update")]
    GridUpdate {
        #[serde(flatten)]
        update: GridUpdateMessage,
    },
    #[serde(rename = "pty_size")]
    PtySize { rows: u16, cols: u16 },
    #[serde(rename = "error")]
    Error { message: String },
}
