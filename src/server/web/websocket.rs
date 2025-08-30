use axum::{
    extract::{ws::WebSocketUpgrade, Path, State},
    response::IntoResponse,
};

use super::types::AppState;
use crate::core::{ClientMessage, ServerMessage};

pub async fn websocket_handler(
    Path(session_id): Path<String>,
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, session_id, state))
}

async fn handle_socket(
    mut socket: axum::extract::ws::WebSocket,
    session_id: String,
    state: AppState,
) {
    use axum::extract::ws::Message;

    tracing::info!(
        "WebSocket connection established for session: {}",
        session_id
    );

    // Get PTY channels from session manager or resume the session
    tracing::debug!("WebSocket requesting channels for session: {}", session_id);
    let pty_channels = if let Some(channels) = state
        .session_manager
        .get_session_channels(&session_id)
        .await
    {
        tracing::debug!("WebSocket found active channels for session: {}", session_id);
        channels
    } else {
        tracing::info!("WebSocket: No active session found for {}, attempting to resume...", session_id);
        
        // Try to get session info to see if it exists but is inactive
        if let Some(session_info) = state.session_manager.get_session(&session_id).await {
            tracing::info!("WebSocket: Found inactive session {}, resuming...", session_id);
            
            // Resume the session by creating a new PTY session with the same ID
            match state.session_manager.resume_session(
                session_id.clone(),
                session_info.agent.clone(),
                vec![], // Resume with empty args
                session_info.project.clone(),
            ).await {
                Ok(_resumed_session) => {
                    tracing::info!("WebSocket: Successfully resumed session {}", session_id);
                    // Get the channels for the resumed session
                    if let Some(channels) = state
                        .session_manager
                        .get_session_channels(&session_id)
                        .await
                    {
                        channels
                    } else {
                        tracing::error!("WebSocket: Failed to get channels for resumed session {}", session_id);
                        return;
                    }
                }
                Err(e) => {
                    tracing::error!("WebSocket: Failed to resume session {}: {}", session_id, e);
                    return;
                }
            }
        } else {
            tracing::error!(
                "WebSocket: Session {} not found - may have been deleted or never existed",
                session_id
            );
            return;
        }
    };

    // Send initial connection message
    let session_short = if session_id.len() >= 8 {
        &session_id[..8]
    } else {
        &session_id
    };
    let welcome_msg = ServerMessage::Output {
        data: format!(
            "Connected to session {} - Claude Code TUI starting...\r\n",
            session_short
        )
        .into_bytes(),
        timestamp: std::time::SystemTime::now(),
    };
    if let Ok(welcome_str) = serde_json::to_string(&welcome_msg) {
        tracing::debug!("WebSocket sending welcome message: {}", welcome_str);
        if socket.send(Message::Text(welcome_str)).await.is_err() {
            tracing::error!("Failed to send welcome message via WebSocket");
            return;
        }
    }

    // Subscribe to PTY size updates
    let mut size_rx = pty_channels.size_tx.subscribe();
    if let Ok(current_size) = size_rx.try_recv() {
        let size_msg = serde_json::json!({
            "type": "pty_size",
            "rows": current_size.rows,
            "cols": current_size.cols
        });
        if socket
            .send(Message::Text(size_msg.to_string()))
            .await
            .is_err()
        {
            return;
        }
    }

    // Subscribe to PTY grid updates (our new primary channel)
    let mut grid_rx = pty_channels.grid_tx.subscribe();
    tracing::debug!("Subscribed to grid update channel");

    // Subscribe to PTY output for fallback/debug (raw bytes)
    let mut pty_output_rx = pty_channels.output_tx.subscribe();
    tracing::debug!("Subscribed to PTY output channel");

    // Clone input channel for sending to PTY
    let pty_input_tx = pty_channels.input_tx.clone();

    // Request keyframe for new client (so they get current terminal state immediately)
    match pty_channels.request_keyframe().await {
        Ok(keyframe) => {
            tracing::debug!("Received keyframe for new WebSocket client");
            let keyframe_ws_msg = ServerMessage::GridUpdate { update: keyframe };
            if let Ok(keyframe_str) = serde_json::to_string(&keyframe_ws_msg) {
                // Test that we can deserialize what we're about to send
                match serde_json::from_str::<ServerMessage>(&keyframe_str) {
                    Ok(_) => {
                        tracing::debug!("WebSocket sending initial keyframe: {} chars (verified deserializable)", keyframe_str.len());
                    }
                    Err(e) => {
                        tracing::error!("Initial keyframe cannot be deserialized: {}", e);
                        tracing::error!("Message content: {}", keyframe_str);
                    }
                }
                if socket.send(Message::Text(keyframe_str)).await.is_err() {
                    tracing::error!("Failed to send initial keyframe to new WebSocket client");
                    return;
                }
            } else {
                tracing::error!(
                    "Initial keyframe cannot be deserialized: {:?}",
                    serde_json::to_string(&keyframe_ws_msg)
                );
            }
        }
        Err(e) => {
            tracing::warn!("Failed to request keyframe for new WebSocket client: {}", e);
        }
    }

    // Main WebSocket handling loop
    loop {
        tokio::select! {
            // Forward grid updates to WebSocket (primary channel)
            grid_update = grid_rx.recv() => {
                match grid_update {
                    Ok(update) => {
                        let ws_msg = ServerMessage::GridUpdate { update };
                        if let Ok(grid_msg) = serde_json::to_string(&ws_msg) {
                            // Test that we can deserialize what we're about to send
                            match serde_json::from_str::<ServerMessage>(&grid_msg) {
                                Ok(_) => {
                                    tracing::debug!("WebSocket sending grid update: {} chars (verified deserializable)", grid_msg.len());
                                }
                                Err(e) => {
                                    tracing::error!("Grid update message cannot be deserialized: {}", e);
                                    tracing::error!("Message content: {}", grid_msg);
                                }
                            }
                            if socket.send(Message::Text(grid_msg)).await.is_err() {
                                tracing::error!("Failed to send grid update via WebSocket");
                                break;
                            }
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                        tracing::info!("PTY grid channel closed");
                        break;
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                        tracing::warn!("WebSocket lagged behind grid updates");
                        // Continue processing
                    }
                }
            }
            // Optional: Forward raw PTY output for debugging
            pty_output = pty_output_rx.recv() => {
                match pty_output {
                    Ok(_output_msg) => {
                        // Debug: show raw PTY output
                        tracing::debug!("WebSocket received raw PTY output: {} bytes", _output_msg.data.len());
                        // Skip raw output - we're using grid updates now
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                        tracing::info!("PTY output channel closed");
                        // Don't break - we can continue with just grid updates
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                        tracing::debug!("WebSocket lagged behind raw PTY output");
                    }
                }
            }
            // Forward PTY size updates to WebSocket
            size_update = size_rx.recv() => {
                match size_update {
                    Ok(size) => {
                        let ws_msg = ServerMessage::PtySize { rows: size.rows, cols: size.cols };
                        if let Ok(size_msg_str) = serde_json::to_string(&ws_msg) {
                            if socket.send(Message::Text(size_msg_str)).await.is_err() {
                                break;
                            }
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                        tracing::info!("PTY size channel closed");
                        // Don't break - we can continue without size updates
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                        tracing::warn!("WebSocket lagged behind PTY size updates");
                        // Continue processing
                    }
                }
            }
            // Handle WebSocket messages from client
            ws_msg = socket.recv() => {
                match ws_msg {
                    Some(Ok(Message::Text(text))) => {
                        tracing::debug!("WebSocket received message: {} chars", text.len());
                        if let Ok(client_msg) = serde_json::from_str::<ClientMessage>(&text) {
                            match client_msg {
                                ClientMessage::Key { code, modifiers } => {
                                    tracing::debug!("WebSocket received key event: {:?} with modifiers {:?}", code, modifiers);
                                    // Convert to PtyInputMessage with key event
                                    let key_event = crate::core::pty_session::KeyEvent { code, modifiers };
                                    let input_msg = crate::core::pty_session::PtyInputMessage {
                                        input: crate::core::pty_session::PtyInput::Key {
                                            event: key_event,
                                            client_id: "web".to_string(),
                                        },
                                    };
                                    if pty_input_tx.send(input_msg).is_err() {
                                        tracing::error!("Failed to send key input to PTY");
                                        break;
                                    }
                                }
                                ClientMessage::Scroll { direction, lines } => {
                                    tracing::debug!("WebSocket received scroll: {:?} {} lines", direction, lines);
                                    // Convert to PtyInputMessage with scroll event
                                    let input_msg = crate::core::pty_session::PtyInputMessage {
                                        input: crate::core::pty_session::PtyInput::Scroll {
                                            direction,
                                            lines,
                                            client_id: "web".to_string(),
                                        },
                                    };
                                    if pty_input_tx.send(input_msg).is_err() {
                                        tracing::error!("Failed to send scroll input to PTY");
                                        break;
                                    }
                                }
                                ClientMessage::Resize { rows, cols } => {
                                    tracing::debug!("WebSocket received resize: {}x{}", cols, rows);
                                    // Send resize control message to PTY
                                    let resize_msg = crate::core::pty_session::PtyControlMessage::Resize { rows, cols };
                                    if let Err(e) = pty_channels.control_tx.send(resize_msg) {
                                        tracing::warn!("Failed to send resize to PTY session {}: {}", session_id, e);
                                    } else {
                                        tracing::debug!("Sent resize {}x{} to PTY session {}", cols, rows, session_id);
                                    }
                                }
                            }
                        } else {
                            tracing::warn!("Failed to parse WebSocket message: {}", text);
                        }
                    }
                    Some(Ok(Message::Close(_))) => {
                        break;
                    }
                    Some(Ok(Message::Ping(data))) => {
                        if socket.send(Message::Pong(data)).await.is_err() {
                            break;
                        }
                    }
                    Some(Err(e)) => {
                        tracing::error!("WebSocket error: {}", e);
                        break;
                    }
                    None => break,
                    _ => {}
                }
            }
        }
    }

    tracing::info!("WebSocket connection closed for session: {}", session_id);
}
