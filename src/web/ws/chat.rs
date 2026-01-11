//! Chat WebSocket handler.
//!
//! This module provides WebSocket handling for real-time chat communication.

use axum::{
    extract::{
        ws::{Message, WebSocket},
        Query, State, WebSocketUpgrade,
    },
    response::Response,
};
use futures::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio::sync::broadcast;

use crate::chat::{ChatMessage, ChatParticipant, ChatRoom, ChatRoomManager, MessageType};
use crate::db::{DbPool, OneTimeTokenRepository, TokenPurpose, UserRepository};

use super::messages::{ClientMessage, ParticipantInfo, RoomInfo, ServerMessage};

/// Query parameters for WebSocket connection.
#[derive(Debug, serde::Deserialize)]
pub struct WsQuery {
    /// One-time token for authentication.
    pub token: String,
}

/// State for WebSocket chat handler.
#[derive(Clone)]
pub struct ChatWsState {
    /// Database pool for token validation.
    pub db_pool: DbPool,
    /// Chat room manager.
    pub chat_manager: Arc<ChatRoomManager>,
}

impl ChatWsState {
    /// Create a new chat WebSocket state.
    pub fn new(db_pool: DbPool, chat_manager: Arc<ChatRoomManager>) -> Self {
        Self {
            db_pool,
            chat_manager,
        }
    }
}

/// User info extracted from one-time token validation.
struct TokenUserInfo {
    user_id: i64,
    username: String,
}

/// WebSocket chat handler.
///
/// GET /api/chat/ws?token={one_time_token}
///
/// The token must be obtained from POST /api/auth/one-time-token with purpose "websocket".
pub async fn chat_ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<ChatWsState>>,
    Query(query): Query<WsQuery>,
) -> Response {
    // Validate one-time token
    let user_info = match validate_one_time_token(&state.db_pool, &query.token).await {
        Ok(info) => info,
        Err(e) => {
            tracing::debug!("WebSocket connection rejected: {}", e);
            return Response::builder()
                .status(401)
                .body("Unauthorized".into())
                .unwrap();
        }
    };

    tracing::info!(
        "WebSocket connection from user {} ({})",
        user_info.username,
        user_info.user_id
    );

    // Upgrade to WebSocket
    ws.on_upgrade(move |socket| handle_socket(socket, state, user_info))
}

/// Validate a one-time token and return user info.
async fn validate_one_time_token(
    db_pool: &DbPool,
    token: &str,
) -> Result<TokenUserInfo, String> {
    let repo = OneTimeTokenRepository::new(db_pool);

    // Consume the token (marks it as used atomically)
    let token_data = repo
        .consume_token(token, TokenPurpose::WebSocket, None)
        .await
        .map_err(|e| format!("Database error: {}", e))?
        .ok_or_else(|| "Invalid or expired token".to_string())?;

    // Get user info
    let user_repo = UserRepository::new(db_pool);
    let user = user_repo
        .get_by_id(token_data.user_id)
        .await
        .map_err(|e| format!("Database error: {}", e))?
        .ok_or_else(|| "User not found".to_string())?;

    Ok(TokenUserInfo {
        user_id: user.id,
        username: user.username,
    })
}

/// Handle a WebSocket connection.
async fn handle_socket(
    socket: WebSocket,
    state: Arc<ChatWsState>,
    user_info: TokenUserInfo,
) {
    let session_id = format!("web-{}-{}", user_info.user_id, uuid::Uuid::new_v4());
    let user_id = user_info.user_id;
    let username = user_info.username.clone();

    tracing::debug!(
        "WebSocket session started: {} for user {}",
        session_id,
        username
    );

    // Split the socket into sender and receiver
    let (mut ws_sender, mut ws_receiver) = socket.split();

    // Track current room
    let mut current_room: Option<Arc<ChatRoom>> = None;
    let mut room_receiver: Option<broadcast::Receiver<ChatMessage>> = None;

    // Send room list on connect
    let rooms = state.chat_manager.list_rooms().await;
    let room_list = ServerMessage::RoomList {
        rooms: rooms
            .into_iter()
            .map(|r| RoomInfo {
                id: r.id,
                name: r.name,
                participant_count: r.participant_count,
            })
            .collect(),
    };
    if let Ok(json) = serde_json::to_string(&room_list) {
        let _ = ws_sender.send(Message::Text(json.into())).await;
    }

    loop {
        tokio::select! {
            // Handle incoming WebSocket messages
            Some(msg_result) = ws_receiver.next() => {
                match msg_result {
                    Ok(Message::Text(text)) => {
                        match serde_json::from_str::<ClientMessage>(&text) {
                            Ok(client_msg) => {
                                handle_client_message(
                                    &mut ws_sender,
                                    &state,
                                    &session_id,
                                    user_id,
                                    &username,
                                    client_msg,
                                    &mut current_room,
                                    &mut room_receiver,
                                ).await;
                            }
                            Err(e) => {
                                tracing::debug!("Failed to parse client message: {}", e);
                                let error = ServerMessage::error("invalid_message", "Invalid message format");
                                if let Ok(json) = serde_json::to_string(&error) {
                                    let _ = ws_sender.send(Message::Text(json.into())).await;
                                }
                            }
                        }
                    }
                    Ok(Message::Close(_)) => {
                        tracing::debug!("WebSocket closed by client: {}", session_id);
                        break;
                    }
                    Ok(Message::Ping(data)) => {
                        let _ = ws_sender.send(Message::Pong(data)).await;
                    }
                    Ok(_) => {}
                    Err(e) => {
                        tracing::debug!("WebSocket error: {}", e);
                        break;
                    }
                }
            }

            // Handle chat room messages
            msg = async {
                if let Some(ref mut receiver) = room_receiver {
                    receiver.recv().await.ok()
                } else {
                    // If no room receiver, wait forever (will be interrupted by other branch)
                    std::future::pending::<Option<ChatMessage>>().await
                }
            } => {
                if let Some(chat_msg) = msg {
                    let server_msg = chat_message_to_server_message(&chat_msg);
                    if let Ok(json) = serde_json::to_string(&server_msg) {
                        if ws_sender.send(Message::Text(json.into())).await.is_err() {
                            break;
                        }
                    }
                }
            }
        }
    }

    // Cleanup: leave all rooms
    state.chat_manager.leave_all_rooms(&session_id).await;
    tracing::debug!("WebSocket session ended: {}", session_id);
}

/// Handle a client message.
#[allow(clippy::too_many_arguments)]
async fn handle_client_message(
    ws_sender: &mut futures::stream::SplitSink<WebSocket, Message>,
    state: &ChatWsState,
    session_id: &str,
    user_id: i64,
    username: &str,
    msg: ClientMessage,
    current_room: &mut Option<Arc<ChatRoom>>,
    room_receiver: &mut Option<broadcast::Receiver<ChatMessage>>,
) {
    match msg {
        ClientMessage::Join { room_id } => {
            // Leave current room if any
            if let Some(ref room) = current_room {
                room.leave(session_id).await;
            }

            // Join new room
            let participant = ChatParticipant::new(session_id, Some(user_id), username);
            match state.chat_manager.join_room(&room_id, participant).await {
                Ok(room) => {
                    // Subscribe to room messages
                    *room_receiver = Some(room.subscribe());

                    // Get participant list
                    let participants = room
                        .participants()
                        .await
                        .into_iter()
                        .map(|p| ParticipantInfo {
                            user_id: p.user_id,
                            username: p.name,
                        })
                        .collect();

                    let response = ServerMessage::Joined {
                        room_id: room.id().to_string(),
                        room_name: room.name().to_string(),
                        participants,
                    };

                    *current_room = Some(room);

                    if let Ok(json) = serde_json::to_string(&response) {
                        let _ = ws_sender.send(Message::Text(json.into())).await;
                    }
                }
                Err(_) => {
                    let error = ServerMessage::error("join_failed", "Failed to join room");
                    if let Ok(json) = serde_json::to_string(&error) {
                        let _ = ws_sender.send(Message::Text(json.into())).await;
                    }
                }
            }
        }

        ClientMessage::Leave => {
            if let Some(ref room) = current_room {
                let room_id = room.id().to_string();
                room.leave(session_id).await;
                *current_room = None;
                *room_receiver = None;

                let response = ServerMessage::Left { room_id };
                if let Ok(json) = serde_json::to_string(&response) {
                    let _ = ws_sender.send(Message::Text(json.into())).await;
                }
            } else {
                let error = ServerMessage::error("not_in_room", "You are not in a room");
                if let Ok(json) = serde_json::to_string(&error) {
                    let _ = ws_sender.send(Message::Text(json.into())).await;
                }
            }
        }

        ClientMessage::Message { content } => {
            if let Some(ref room) = current_room {
                // Send message to room
                room.send_message(session_id, &content).await;
            } else {
                let error = ServerMessage::error("not_in_room", "You are not in a room");
                if let Ok(json) = serde_json::to_string(&error) {
                    let _ = ws_sender.send(Message::Text(json.into())).await;
                }
            }
        }

        ClientMessage::Action { content } => {
            if let Some(ref room) = current_room {
                // Send action to room
                room.send_action(session_id, &content).await;
            } else {
                let error = ServerMessage::error("not_in_room", "You are not in a room");
                if let Ok(json) = serde_json::to_string(&error) {
                    let _ = ws_sender.send(Message::Text(json.into())).await;
                }
            }
        }

        ClientMessage::Ping => {
            let response = ServerMessage::Pong;
            if let Ok(json) = serde_json::to_string(&response) {
                let _ = ws_sender.send(Message::Text(json.into())).await;
            }
        }
    }
}

/// Convert a ChatMessage to a ServerMessage.
fn chat_message_to_server_message(msg: &ChatMessage) -> ServerMessage {
    let timestamp = msg.timestamp.to_rfc3339();

    match msg.message_type {
        MessageType::Chat => ServerMessage::Chat {
            user_id: msg.sender_id.as_ref().map(|_| 0), // We don't have user_id in ChatMessage sender_id
            username: msg.sender_name.clone(),
            content: msg.content.clone(),
            timestamp,
        },
        MessageType::Action => {
            // For action, we need to extract user_id somehow
            // Since ChatMessage doesn't store user_id, we'll use 0
            ServerMessage::Action {
                user_id: 0,
                username: msg.sender_name.clone(),
                content: msg.content.clone(),
                timestamp,
            }
        }
        MessageType::System => ServerMessage::System {
            content: msg.content.clone(),
            timestamp,
        },
        MessageType::Join => ServerMessage::UserJoined {
            user_id: 0, // ChatMessage doesn't have user_id
            username: msg.sender_name.clone(),
            timestamp,
        },
        MessageType::Leave => ServerMessage::UserLeft {
            user_id: 0, // ChatMessage doesn't have user_id
            username: msg.sender_name.clone(),
            timestamp,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Database;

    #[tokio::test]
    async fn test_chat_ws_state_new() {
        let db = Database::open_in_memory().await.unwrap();
        let db_pool = db.pool().clone();
        let chat_manager = Arc::new(ChatRoomManager::new());
        let _state = ChatWsState::new(db_pool, chat_manager);
    }

    #[tokio::test]
    async fn test_validate_one_time_token_invalid() {
        let db = Database::open_in_memory().await.unwrap();
        let result = validate_one_time_token(db.pool(), "invalid-token").await;
        assert!(result.is_err());
    }

    #[test]
    fn test_chat_message_to_server_message_chat() {
        let msg = ChatMessage::new("session1", "Alice", "Hello!");
        let server_msg = chat_message_to_server_message(&msg);
        match server_msg {
            ServerMessage::Chat {
                username, content, ..
            } => {
                assert_eq!(username, "Alice");
                assert_eq!(content, "Hello!");
            }
            _ => panic!("Expected Chat message"),
        }
    }

    #[test]
    fn test_chat_message_to_server_message_action() {
        let msg = ChatMessage::action("session1", "Alice", "waves");
        let server_msg = chat_message_to_server_message(&msg);
        match server_msg {
            ServerMessage::Action {
                username, content, ..
            } => {
                assert_eq!(username, "Alice");
                assert_eq!(content, "waves");
            }
            _ => panic!("Expected Action message"),
        }
    }

    #[test]
    fn test_chat_message_to_server_message_system() {
        let msg = ChatMessage::system("Server maintenance");
        let server_msg = chat_message_to_server_message(&msg);
        match server_msg {
            ServerMessage::System { content, .. } => {
                assert_eq!(content, "Server maintenance");
            }
            _ => panic!("Expected System message"),
        }
    }

    #[test]
    fn test_chat_message_to_server_message_join() {
        let msg = ChatMessage::join("session1", "Alice");
        let server_msg = chat_message_to_server_message(&msg);
        match server_msg {
            ServerMessage::UserJoined { username, .. } => {
                assert_eq!(username, "Alice");
            }
            _ => panic!("Expected UserJoined message"),
        }
    }

    #[test]
    fn test_chat_message_to_server_message_leave() {
        let msg = ChatMessage::leave("session1", "Alice");
        let server_msg = chat_message_to_server_message(&msg);
        match server_msg {
            ServerMessage::UserLeft { username, .. } => {
                assert_eq!(username, "Alice");
            }
            _ => panic!("Expected UserLeft message"),
        }
    }
}
