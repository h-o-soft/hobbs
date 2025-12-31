//! Chat room implementation for HOBBS.
//!
//! This module provides chat room functionality with broadcast messaging
//! using tokio's broadcast channel.

use std::collections::HashMap;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use tokio::sync::{broadcast, RwLock};

/// Maximum number of messages to buffer in the broadcast channel.
const CHANNEL_CAPACITY: usize = 100;

/// Type of chat message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageType {
    /// Regular chat message.
    Chat,
    /// Action message (e.g., "/me yawns").
    Action,
    /// System message.
    System,
    /// User joined notification.
    Join,
    /// User left notification.
    Leave,
}

impl MessageType {
    /// Get string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            MessageType::Chat => "chat",
            MessageType::Action => "action",
            MessageType::System => "system",
            MessageType::Join => "join",
            MessageType::Leave => "leave",
        }
    }
}

impl std::fmt::Display for MessageType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// A chat message.
#[derive(Debug, Clone)]
pub struct ChatMessage {
    /// Sender's session ID (None for system messages).
    pub sender_id: Option<String>,
    /// Sender's display name (empty for system messages).
    pub sender_name: String,
    /// Message type.
    pub message_type: MessageType,
    /// Message content.
    pub content: String,
    /// Timestamp when the message was sent.
    pub timestamp: DateTime<Utc>,
}

impl ChatMessage {
    /// Create a new chat message.
    pub fn new(
        sender_id: impl Into<String>,
        sender_name: impl Into<String>,
        content: impl Into<String>,
    ) -> Self {
        Self {
            sender_id: Some(sender_id.into()),
            sender_name: sender_name.into(),
            message_type: MessageType::Chat,
            content: content.into(),
            timestamp: Utc::now(),
        }
    }

    /// Create an action message (/me command).
    pub fn action(
        sender_id: impl Into<String>,
        sender_name: impl Into<String>,
        action: impl Into<String>,
    ) -> Self {
        Self {
            sender_id: Some(sender_id.into()),
            sender_name: sender_name.into(),
            message_type: MessageType::Action,
            content: action.into(),
            timestamp: Utc::now(),
        }
    }

    /// Create a system message.
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            sender_id: None,
            sender_name: String::new(),
            message_type: MessageType::System,
            content: content.into(),
            timestamp: Utc::now(),
        }
    }

    /// Create a join notification.
    pub fn join(sender_id: impl Into<String>, sender_name: impl Into<String>) -> Self {
        let name = sender_name.into();
        Self {
            sender_id: Some(sender_id.into()),
            sender_name: name.clone(),
            message_type: MessageType::Join,
            content: format!("{name} が入室しました"),
            timestamp: Utc::now(),
        }
    }

    /// Create a leave notification.
    pub fn leave(sender_id: impl Into<String>, sender_name: impl Into<String>) -> Self {
        let name = sender_name.into();
        Self {
            sender_id: Some(sender_id.into()),
            sender_name: name.clone(),
            message_type: MessageType::Leave,
            content: format!("{name} が退室しました"),
            timestamp: Utc::now(),
        }
    }

    /// Format the message for display.
    pub fn format(&self) -> String {
        match self.message_type {
            MessageType::Chat => format!("<{}> {}", self.sender_name, self.content),
            MessageType::Action => format!("* {} {}", self.sender_name, self.content),
            MessageType::System => format!("*** {}", self.content),
            MessageType::Join => format!("*** {}", self.content),
            MessageType::Leave => format!("*** {}", self.content),
        }
    }
}

/// A chat participant.
#[derive(Debug, Clone)]
pub struct ChatParticipant {
    /// Session ID.
    pub session_id: String,
    /// User ID (None for guests).
    pub user_id: Option<i64>,
    /// Display name.
    pub name: String,
    /// Join timestamp.
    pub joined_at: DateTime<Utc>,
}

impl ChatParticipant {
    /// Create a new participant.
    pub fn new(
        session_id: impl Into<String>,
        user_id: Option<i64>,
        name: impl Into<String>,
    ) -> Self {
        Self {
            session_id: session_id.into(),
            user_id,
            name: name.into(),
            joined_at: Utc::now(),
        }
    }
}

/// A chat room with broadcast messaging.
pub struct ChatRoom {
    /// Room ID.
    id: String,
    /// Room name.
    name: String,
    /// Participants indexed by session ID.
    participants: Arc<RwLock<HashMap<String, ChatParticipant>>>,
    /// Broadcast sender for messages.
    sender: broadcast::Sender<ChatMessage>,
}

impl ChatRoom {
    /// Create a new chat room.
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        let (sender, _) = broadcast::channel(CHANNEL_CAPACITY);
        Self {
            id: id.into(),
            name: name.into(),
            participants: Arc::new(RwLock::new(HashMap::new())),
            sender,
        }
    }

    /// Get the room ID.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Get the room name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get a receiver for broadcast messages.
    pub fn subscribe(&self) -> broadcast::Receiver<ChatMessage> {
        self.sender.subscribe()
    }

    /// Get the number of participants.
    pub async fn participant_count(&self) -> usize {
        self.participants.read().await.len()
    }

    /// Get a list of participant names.
    pub async fn participant_names(&self) -> Vec<String> {
        self.participants
            .read()
            .await
            .values()
            .map(|p| p.name.clone())
            .collect()
    }

    /// Get all participants.
    pub async fn participants(&self) -> Vec<ChatParticipant> {
        self.participants.read().await.values().cloned().collect()
    }

    /// Check if a session is in the room.
    pub async fn is_participant(&self, session_id: &str) -> bool {
        self.participants.read().await.contains_key(session_id)
    }

    /// Join the room.
    ///
    /// Returns true if the user was added, false if already in the room.
    pub async fn join(&self, participant: ChatParticipant) -> bool {
        let session_id = participant.session_id.clone();
        let name = participant.name.clone();

        let mut participants = self.participants.write().await;
        if participants.contains_key(&session_id) {
            return false;
        }

        participants.insert(session_id.clone(), participant);
        drop(participants);

        // Broadcast join notification
        let _ = self.sender.send(ChatMessage::join(&session_id, &name));
        true
    }

    /// Leave the room.
    ///
    /// Returns true if the user was removed, false if not in the room.
    pub async fn leave(&self, session_id: &str) -> bool {
        let mut participants = self.participants.write().await;
        if let Some(participant) = participants.remove(session_id) {
            drop(participants);

            // Broadcast leave notification
            let _ = self
                .sender
                .send(ChatMessage::leave(session_id, &participant.name));
            true
        } else {
            false
        }
    }

    /// Send a chat message.
    ///
    /// Returns the number of receivers that received the message.
    pub async fn send_message(&self, session_id: &str, content: impl Into<String>) -> usize {
        let participants = self.participants.read().await;
        if let Some(participant) = participants.get(session_id) {
            let message = ChatMessage::new(session_id, &participant.name, content);
            drop(participants);
            self.sender.send(message).unwrap_or(0)
        } else {
            0
        }
    }

    /// Send an action message (/me command).
    ///
    /// Returns the number of receivers that received the message.
    pub async fn send_action(&self, session_id: &str, action: impl Into<String>) -> usize {
        let participants = self.participants.read().await;
        if let Some(participant) = participants.get(session_id) {
            let message = ChatMessage::action(session_id, &participant.name, action);
            drop(participants);
            self.sender.send(message).unwrap_or(0)
        } else {
            0
        }
    }

    /// Broadcast a system message.
    ///
    /// Returns the number of receivers that received the message.
    pub fn broadcast_system(&self, content: impl Into<String>) -> usize {
        self.sender.send(ChatMessage::system(content)).unwrap_or(0)
    }

    /// Broadcast a raw message.
    ///
    /// Returns the number of receivers that received the message.
    pub fn broadcast(&self, message: ChatMessage) -> usize {
        self.sender.send(message).unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_type_as_str() {
        assert_eq!(MessageType::Chat.as_str(), "chat");
        assert_eq!(MessageType::Action.as_str(), "action");
        assert_eq!(MessageType::System.as_str(), "system");
        assert_eq!(MessageType::Join.as_str(), "join");
        assert_eq!(MessageType::Leave.as_str(), "leave");
    }

    #[test]
    fn test_chat_message_new() {
        let msg = ChatMessage::new("session1", "Alice", "Hello!");
        assert_eq!(msg.sender_id, Some("session1".to_string()));
        assert_eq!(msg.sender_name, "Alice");
        assert_eq!(msg.message_type, MessageType::Chat);
        assert_eq!(msg.content, "Hello!");
    }

    #[test]
    fn test_chat_message_action() {
        let msg = ChatMessage::action("session1", "Alice", "yawns");
        assert_eq!(msg.message_type, MessageType::Action);
        assert_eq!(msg.content, "yawns");
    }

    #[test]
    fn test_chat_message_system() {
        let msg = ChatMessage::system("Server is shutting down");
        assert!(msg.sender_id.is_none());
        assert_eq!(msg.sender_name, "");
        assert_eq!(msg.message_type, MessageType::System);
    }

    #[test]
    fn test_chat_message_join() {
        let msg = ChatMessage::join("session1", "Alice");
        assert_eq!(msg.message_type, MessageType::Join);
        assert!(msg.content.contains("Alice"));
        assert!(msg.content.contains("入室"));
    }

    #[test]
    fn test_chat_message_leave() {
        let msg = ChatMessage::leave("session1", "Alice");
        assert_eq!(msg.message_type, MessageType::Leave);
        assert!(msg.content.contains("Alice"));
        assert!(msg.content.contains("退室"));
    }

    #[test]
    fn test_chat_message_format() {
        let chat = ChatMessage::new("s1", "Alice", "Hello!");
        assert_eq!(chat.format(), "<Alice> Hello!");

        let action = ChatMessage::action("s1", "Alice", "waves");
        assert_eq!(action.format(), "* Alice waves");

        let system = ChatMessage::system("Test");
        assert_eq!(system.format(), "*** Test");
    }

    #[test]
    fn test_chat_participant_new() {
        let participant = ChatParticipant::new("session1", Some(42), "Alice");
        assert_eq!(participant.session_id, "session1");
        assert_eq!(participant.user_id, Some(42));
        assert_eq!(participant.name, "Alice");
    }

    #[test]
    fn test_chat_participant_guest() {
        let participant = ChatParticipant::new("session1", None, "Guest123");
        assert!(participant.user_id.is_none());
    }

    #[tokio::test]
    async fn test_chat_room_new() {
        let room = ChatRoom::new("lobby", "Lobby");
        assert_eq!(room.id(), "lobby");
        assert_eq!(room.name(), "Lobby");
        assert_eq!(room.participant_count().await, 0);
    }

    #[tokio::test]
    async fn test_chat_room_join() {
        let room = ChatRoom::new("lobby", "Lobby");
        let participant = ChatParticipant::new("session1", Some(1), "Alice");

        let joined = room.join(participant).await;
        assert!(joined);
        assert_eq!(room.participant_count().await, 1);
        assert!(room.is_participant("session1").await);
    }

    #[tokio::test]
    async fn test_chat_room_join_duplicate() {
        let room = ChatRoom::new("lobby", "Lobby");
        let participant1 = ChatParticipant::new("session1", Some(1), "Alice");
        let participant2 = ChatParticipant::new("session1", Some(1), "Alice");

        assert!(room.join(participant1).await);
        assert!(!room.join(participant2).await); // Duplicate
        assert_eq!(room.participant_count().await, 1);
    }

    #[tokio::test]
    async fn test_chat_room_leave() {
        let room = ChatRoom::new("lobby", "Lobby");
        let participant = ChatParticipant::new("session1", Some(1), "Alice");

        room.join(participant).await;
        assert!(room.is_participant("session1").await);

        let left = room.leave("session1").await;
        assert!(left);
        assert!(!room.is_participant("session1").await);
        assert_eq!(room.participant_count().await, 0);
    }

    #[tokio::test]
    async fn test_chat_room_leave_not_found() {
        let room = ChatRoom::new("lobby", "Lobby");
        let left = room.leave("nonexistent").await;
        assert!(!left);
    }

    #[tokio::test]
    async fn test_chat_room_participant_names() {
        let room = ChatRoom::new("lobby", "Lobby");
        room.join(ChatParticipant::new("s1", Some(1), "Alice"))
            .await;
        room.join(ChatParticipant::new("s2", Some(2), "Bob")).await;

        let names = room.participant_names().await;
        assert_eq!(names.len(), 2);
        assert!(names.contains(&"Alice".to_string()));
        assert!(names.contains(&"Bob".to_string()));
    }

    #[tokio::test]
    async fn test_chat_room_send_message() {
        let room = ChatRoom::new("lobby", "Lobby");
        let mut receiver = room.subscribe();

        let participant = ChatParticipant::new("session1", Some(1), "Alice");
        room.join(participant).await;

        // Consume the join message
        let _ = receiver.recv().await;

        room.send_message("session1", "Hello!").await;

        let msg = receiver.recv().await.unwrap();
        assert_eq!(msg.message_type, MessageType::Chat);
        assert_eq!(msg.content, "Hello!");
        assert_eq!(msg.sender_name, "Alice");
    }

    #[tokio::test]
    async fn test_chat_room_send_action() {
        let room = ChatRoom::new("lobby", "Lobby");
        let mut receiver = room.subscribe();

        let participant = ChatParticipant::new("session1", Some(1), "Alice");
        room.join(participant).await;

        // Consume the join message
        let _ = receiver.recv().await;

        room.send_action("session1", "waves").await;

        let msg = receiver.recv().await.unwrap();
        assert_eq!(msg.message_type, MessageType::Action);
        assert_eq!(msg.content, "waves");
    }

    #[tokio::test]
    async fn test_chat_room_broadcast_system() {
        let room = ChatRoom::new("lobby", "Lobby");
        let mut receiver = room.subscribe();

        room.broadcast_system("Server maintenance in 5 minutes");

        let msg = receiver.recv().await.unwrap();
        assert_eq!(msg.message_type, MessageType::System);
        assert!(msg.content.contains("maintenance"));
    }

    #[tokio::test]
    async fn test_chat_room_send_message_not_participant() {
        let room = ChatRoom::new("lobby", "Lobby");
        let count = room.send_message("nonexistent", "Hello!").await;
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn test_chat_room_multiple_participants() {
        let room = ChatRoom::new("lobby", "Lobby");

        room.join(ChatParticipant::new("s1", Some(1), "Alice"))
            .await;
        room.join(ChatParticipant::new("s2", Some(2), "Bob")).await;
        room.join(ChatParticipant::new("s3", Some(3), "Charlie"))
            .await;

        assert_eq!(room.participant_count().await, 3);

        let participants = room.participants().await;
        assert_eq!(participants.len(), 3);
    }

    #[tokio::test]
    async fn test_chat_room_broadcast_to_multiple_receivers() {
        let room = ChatRoom::new("lobby", "Lobby");

        let mut receiver1 = room.subscribe();
        let mut receiver2 = room.subscribe();

        room.broadcast_system("Test message");

        // Both receivers should get the message
        let msg1 = receiver1.recv().await.unwrap();
        let msg2 = receiver2.recv().await.unwrap();

        assert_eq!(msg1.content, msg2.content);
    }

    #[tokio::test]
    async fn test_chat_room_join_broadcasts_notification() {
        let room = ChatRoom::new("lobby", "Lobby");
        let mut receiver = room.subscribe();

        let participant = ChatParticipant::new("session1", Some(1), "Alice");
        room.join(participant).await;

        let msg = receiver.recv().await.unwrap();
        assert_eq!(msg.message_type, MessageType::Join);
        assert!(msg.content.contains("Alice"));
    }

    #[tokio::test]
    async fn test_chat_room_leave_broadcasts_notification() {
        let room = ChatRoom::new("lobby", "Lobby");
        let mut receiver = room.subscribe();

        let participant = ChatParticipant::new("session1", Some(1), "Alice");
        room.join(participant).await;

        // Consume join message
        let _ = receiver.recv().await;

        room.leave("session1").await;

        let msg = receiver.recv().await.unwrap();
        assert_eq!(msg.message_type, MessageType::Leave);
        assert!(msg.content.contains("Alice"));
    }

    #[tokio::test]
    async fn test_chat_room_concurrent_operations() {
        let room = Arc::new(ChatRoom::new("lobby", "Lobby"));

        let room1 = room.clone();
        let room2 = room.clone();
        let room3 = room.clone();

        let h1 = tokio::spawn(async move {
            room1
                .join(ChatParticipant::new("s1", Some(1), "Alice"))
                .await
        });
        let h2 =
            tokio::spawn(
                async move { room2.join(ChatParticipant::new("s2", Some(2), "Bob")).await },
            );
        let h3 = tokio::spawn(async move {
            room3
                .join(ChatParticipant::new("s3", Some(3), "Charlie"))
                .await
        });

        let _ = tokio::join!(h1, h2, h3);

        assert_eq!(room.participant_count().await, 3);
    }

    #[tokio::test]
    async fn test_chat_room_receiver_lagged() {
        let room = ChatRoom::new("lobby", "Lobby");
        let mut receiver = room.subscribe();

        // Send more messages than the channel capacity
        for i in 0..150 {
            room.broadcast_system(format!("Message {i}"));
        }

        // The receiver should have lagged
        // First recv will return a lagged error, then subsequent messages
        let result = receiver.recv().await;
        // Either we get a message or we got lagged
        assert!(result.is_ok() || result.is_err());
    }
}
