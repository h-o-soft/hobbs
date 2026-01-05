//! WebSocket message types for chat communication.

use serde::{Deserialize, Serialize};

/// Messages sent from client to server.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    /// Join a chat room.
    Join {
        /// Room ID to join.
        room_id: String,
    },
    /// Leave the current chat room.
    Leave,
    /// Send a chat message.
    Message {
        /// Message content.
        content: String,
    },
    /// Send an action (/me command).
    Action {
        /// Action content.
        content: String,
    },
    /// Heartbeat ping.
    Ping,
}

/// Messages sent from server to client.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    /// Chat message.
    Chat {
        /// Sender's user ID (None for system messages).
        user_id: Option<i64>,
        /// Sender's display name.
        username: String,
        /// Message content.
        content: String,
        /// ISO 8601 timestamp.
        timestamp: String,
    },
    /// Action message (/me command).
    Action {
        /// Sender's user ID.
        user_id: i64,
        /// Sender's display name.
        username: String,
        /// Action content.
        content: String,
        /// ISO 8601 timestamp.
        timestamp: String,
    },
    /// User joined the room.
    UserJoined {
        /// User ID.
        user_id: i64,
        /// Display name.
        username: String,
        /// ISO 8601 timestamp.
        timestamp: String,
    },
    /// User left the room.
    UserLeft {
        /// User ID.
        user_id: i64,
        /// Display name.
        username: String,
        /// ISO 8601 timestamp.
        timestamp: String,
    },
    /// System message.
    System {
        /// Message content.
        content: String,
        /// ISO 8601 timestamp.
        timestamp: String,
    },
    /// Error message.
    Error {
        /// Error code.
        code: String,
        /// Error message.
        message: String,
    },
    /// Heartbeat pong response.
    Pong,
    /// Successfully joined a room.
    Joined {
        /// Room ID.
        room_id: String,
        /// Room name.
        room_name: String,
        /// List of current participants.
        participants: Vec<ParticipantInfo>,
    },
    /// Successfully left a room.
    Left {
        /// Room ID.
        room_id: String,
    },
    /// Room list.
    RoomList {
        /// Available rooms.
        rooms: Vec<RoomInfo>,
    },
}

/// Information about a chat participant.
#[derive(Debug, Clone, Serialize)]
pub struct ParticipantInfo {
    /// User ID.
    pub user_id: Option<i64>,
    /// Display name.
    pub username: String,
}

/// Information about a chat room.
#[derive(Debug, Clone, Serialize)]
pub struct RoomInfo {
    /// Room ID.
    pub id: String,
    /// Room name.
    pub name: String,
    /// Number of participants.
    pub participant_count: usize,
}

impl ServerMessage {
    /// Create an error message.
    pub fn error(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Error {
            code: code.into(),
            message: message.into(),
        }
    }

    /// Create a system message.
    pub fn system(content: impl Into<String>) -> Self {
        Self::System {
            content: content.into(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_message_join_deserialize() {
        let json = r#"{"type": "join", "room_id": "lobby"}"#;
        let msg: ClientMessage = serde_json::from_str(json).unwrap();
        match msg {
            ClientMessage::Join { room_id } => assert_eq!(room_id, "lobby"),
            _ => panic!("Expected Join message"),
        }
    }

    #[test]
    fn test_client_message_message_deserialize() {
        let json = r#"{"type": "message", "content": "Hello!"}"#;
        let msg: ClientMessage = serde_json::from_str(json).unwrap();
        match msg {
            ClientMessage::Message { content } => assert_eq!(content, "Hello!"),
            _ => panic!("Expected Message message"),
        }
    }

    #[test]
    fn test_client_message_action_deserialize() {
        let json = r#"{"type": "action", "content": "waves"}"#;
        let msg: ClientMessage = serde_json::from_str(json).unwrap();
        match msg {
            ClientMessage::Action { content } => assert_eq!(content, "waves"),
            _ => panic!("Expected Action message"),
        }
    }

    #[test]
    fn test_client_message_leave_deserialize() {
        let json = r#"{"type": "leave"}"#;
        let msg: ClientMessage = serde_json::from_str(json).unwrap();
        assert!(matches!(msg, ClientMessage::Leave));
    }

    #[test]
    fn test_client_message_ping_deserialize() {
        let json = r#"{"type": "ping"}"#;
        let msg: ClientMessage = serde_json::from_str(json).unwrap();
        assert!(matches!(msg, ClientMessage::Ping));
    }

    #[test]
    fn test_server_message_chat_serialize() {
        let msg = ServerMessage::Chat {
            user_id: Some(1),
            username: "Alice".to_string(),
            content: "Hello!".to_string(),
            timestamp: "2024-01-01T00:00:00Z".to_string(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"type\":\"chat\""));
        assert!(json.contains("\"username\":\"Alice\""));
    }

    #[test]
    fn test_server_message_error_serialize() {
        let msg = ServerMessage::error("not_in_room", "You are not in a room");
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"type\":\"error\""));
        assert!(json.contains("\"code\":\"not_in_room\""));
    }

    #[test]
    fn test_server_message_pong_serialize() {
        let msg = ServerMessage::Pong;
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"type\":\"pong\""));
    }
}
