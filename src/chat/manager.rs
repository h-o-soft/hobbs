//! Chat room manager for HOBBS.
//!
//! This module provides centralized management of chat rooms,
//! allowing multiple sessions to share the same room instances.

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::RwLock;

use super::room::{ChatParticipant, ChatRoom, JoinResult};

/// Default chat rooms to create on startup.
const DEFAULT_ROOMS: &[(&str, &str)] =
    &[("lobby", "Lobby"), ("tech", "Tech"), ("random", "Random")];

/// Manager for chat rooms.
///
/// This is shared across all sessions and provides thread-safe
/// access to chat rooms.
pub struct ChatRoomManager {
    /// Chat rooms indexed by ID.
    rooms: RwLock<HashMap<String, Arc<ChatRoom>>>,
}

impl ChatRoomManager {
    /// Create a new chat room manager.
    pub fn new() -> Self {
        Self {
            rooms: RwLock::new(HashMap::new()),
        }
    }

    /// Create a new manager with default rooms.
    pub async fn with_defaults() -> Self {
        let manager = Self::new();
        for (id, name) in DEFAULT_ROOMS {
            manager.create_room(*id, *name).await;
        }
        manager
    }

    /// Create a new room.
    ///
    /// Returns the room if created, or None if a room with that ID already exists.
    pub async fn create_room(
        &self,
        id: impl Into<String>,
        name: impl Into<String>,
    ) -> Option<Arc<ChatRoom>> {
        let id = id.into();
        let mut rooms = self.rooms.write().await;

        if rooms.contains_key(&id) {
            return None;
        }

        let room = Arc::new(ChatRoom::new(&id, name));
        rooms.insert(id, Arc::clone(&room));
        Some(room)
    }

    /// Get a room by ID.
    pub async fn get_room(&self, id: &str) -> Option<Arc<ChatRoom>> {
        self.rooms.read().await.get(id).cloned()
    }

    /// List all rooms.
    pub async fn list_rooms(&self) -> Vec<RoomInfo> {
        let rooms = self.rooms.read().await;
        let mut result = Vec::new();

        for (id, room) in rooms.iter() {
            result.push(RoomInfo {
                id: id.clone(),
                name: room.name().to_string(),
                participant_count: room.participant_count().await,
            });
        }

        // Sort by ID for consistent ordering
        result.sort_by(|a, b| a.id.cmp(&b.id));
        result
    }

    /// Get the number of rooms.
    pub async fn room_count(&self) -> usize {
        self.rooms.read().await.len()
    }

    /// Remove a session from all rooms.
    ///
    /// This should be called when a session disconnects.
    pub async fn leave_all_rooms(&self, session_id: &str) {
        let rooms = self.rooms.read().await;
        for room in rooms.values() {
            room.leave(session_id).await;
        }
    }

    /// Get total number of participants across all rooms.
    pub async fn total_participants(&self) -> usize {
        let rooms = self.rooms.read().await;
        let mut total = 0;
        for room in rooms.values() {
            total += room.participant_count().await;
        }
        total
    }

    /// Join a room.
    ///
    /// Returns Ok(room) if joined successfully, or Err(JoinResult) if failed.
    pub async fn join_room(
        &self,
        room_id: &str,
        participant: ChatParticipant,
    ) -> Result<Arc<ChatRoom>, JoinResult> {
        let room = self.get_room(room_id).await.ok_or(JoinResult::RoomFull)?;
        let result = room.join(participant).await;
        match result {
            JoinResult::Joined => Ok(room),
            other => Err(other),
        }
    }

    /// Leave a room.
    pub async fn leave_room(&self, room_id: &str, session_id: &str) -> bool {
        if let Some(room) = self.get_room(room_id).await {
            room.leave(session_id).await
        } else {
            false
        }
    }

    /// Delete a room.
    ///
    /// Returns Ok(room_name) if deleted successfully, or an error if:
    /// - The room doesn't exist
    /// - The room has active participants
    pub async fn delete_room(&self, room_id: &str) -> Result<String, DeleteRoomError> {
        let mut rooms = self.rooms.write().await;

        // Check if room exists
        let room = rooms.get(room_id).ok_or(DeleteRoomError::NotFound)?;

        // Check if room has participants
        if room.participant_count().await > 0 {
            return Err(DeleteRoomError::HasParticipants);
        }

        // Remove the room
        let room = rooms.remove(room_id).unwrap();
        Ok(room.name().to_string())
    }
}

impl Default for ChatRoomManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Information about a chat room.
#[derive(Debug, Clone)]
pub struct RoomInfo {
    /// Room ID.
    pub id: String,
    /// Room name.
    pub name: String,
    /// Number of participants.
    pub participant_count: usize,
}

/// Error when deleting a room.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeleteRoomError {
    /// Room not found.
    NotFound,
    /// Room has active participants.
    HasParticipants,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_manager_new() {
        let manager = ChatRoomManager::new();
        assert_eq!(manager.room_count().await, 0);
    }

    #[tokio::test]
    async fn test_manager_with_defaults() {
        let manager = ChatRoomManager::with_defaults().await;
        assert_eq!(manager.room_count().await, 3);

        assert!(manager.get_room("lobby").await.is_some());
        assert!(manager.get_room("tech").await.is_some());
        assert!(manager.get_room("random").await.is_some());
    }

    #[tokio::test]
    async fn test_create_room() {
        let manager = ChatRoomManager::new();

        let room = manager.create_room("test", "Test Room").await;
        assert!(room.is_some());
        assert_eq!(manager.room_count().await, 1);

        // Duplicate should fail
        let room2 = manager.create_room("test", "Another Name").await;
        assert!(room2.is_none());
        assert_eq!(manager.room_count().await, 1);
    }

    #[tokio::test]
    async fn test_get_room() {
        let manager = ChatRoomManager::new();
        manager.create_room("lobby", "Lobby").await;

        let room = manager.get_room("lobby").await;
        assert!(room.is_some());
        assert_eq!(room.unwrap().name(), "Lobby");

        let missing = manager.get_room("nonexistent").await;
        assert!(missing.is_none());
    }

    #[tokio::test]
    async fn test_list_rooms() {
        let manager = ChatRoomManager::new();
        manager.create_room("room-b", "Room B").await;
        manager.create_room("room-a", "Room A").await;

        let rooms = manager.list_rooms().await;
        assert_eq!(rooms.len(), 2);
        // Should be sorted by ID
        assert_eq!(rooms[0].id, "room-a");
        assert_eq!(rooms[1].id, "room-b");
    }

    #[tokio::test]
    async fn test_join_room() {
        let manager = ChatRoomManager::new();
        manager.create_room("lobby", "Lobby").await;

        let participant = ChatParticipant::new("session1", Some(1), "Alice");
        let room = manager.join_room("lobby", participant).await;

        assert!(room.is_ok());
        assert_eq!(room.unwrap().participant_count().await, 1);
    }

    #[tokio::test]
    async fn test_join_room_not_found() {
        let manager = ChatRoomManager::new();

        let participant = ChatParticipant::new("session1", Some(1), "Alice");
        let result = manager.join_room("nonexistent", participant).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_leave_room() {
        let manager = ChatRoomManager::new();
        manager.create_room("lobby", "Lobby").await;

        let participant = ChatParticipant::new("session1", Some(1), "Alice");
        let _ = manager.join_room("lobby", participant).await;

        let room = manager.get_room("lobby").await.unwrap();
        assert_eq!(room.participant_count().await, 1);

        let left = manager.leave_room("lobby", "session1").await;
        assert!(left);
        assert_eq!(room.participant_count().await, 0);
    }

    #[tokio::test]
    async fn test_leave_all_rooms() {
        let manager = ChatRoomManager::new();
        manager.create_room("room1", "Room 1").await;
        manager.create_room("room2", "Room 2").await;

        let p1 = ChatParticipant::new("session1", Some(1), "Alice");
        let p2 = ChatParticipant::new("session1", Some(1), "Alice");
        let _ = manager.join_room("room1", p1).await;
        let _ = manager.join_room("room2", p2).await;

        let room1 = manager.get_room("room1").await.unwrap();
        let room2 = manager.get_room("room2").await.unwrap();
        assert_eq!(room1.participant_count().await, 1);
        assert_eq!(room2.participant_count().await, 1);

        manager.leave_all_rooms("session1").await;

        assert_eq!(room1.participant_count().await, 0);
        assert_eq!(room2.participant_count().await, 0);
    }

    #[tokio::test]
    async fn test_total_participants() {
        let manager = ChatRoomManager::new();
        manager.create_room("room1", "Room 1").await;
        manager.create_room("room2", "Room 2").await;

        assert_eq!(manager.total_participants().await, 0);

        let _ = manager
            .join_room("room1", ChatParticipant::new("s1", Some(1), "Alice"))
            .await;
        let _ = manager
            .join_room("room1", ChatParticipant::new("s2", Some(2), "Bob"))
            .await;
        let _ = manager
            .join_room("room2", ChatParticipant::new("s3", Some(3), "Charlie"))
            .await;

        assert_eq!(manager.total_participants().await, 3);
    }

    #[tokio::test]
    async fn test_delete_room_success() {
        let manager = ChatRoomManager::new();
        manager.create_room("test", "Test Room").await;
        assert_eq!(manager.room_count().await, 1);

        let result = manager.delete_room("test").await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Test Room");
        assert_eq!(manager.room_count().await, 0);
    }

    #[tokio::test]
    async fn test_delete_room_not_found() {
        let manager = ChatRoomManager::new();

        let result = manager.delete_room("nonexistent").await;
        assert_eq!(result, Err(DeleteRoomError::NotFound));
    }

    #[tokio::test]
    async fn test_delete_room_has_participants() {
        let manager = ChatRoomManager::new();
        manager.create_room("test", "Test Room").await;

        let participant = ChatParticipant::new("session1", Some(1), "Alice");
        let _ = manager.join_room("test", participant).await;

        let result = manager.delete_room("test").await;
        assert_eq!(result, Err(DeleteRoomError::HasParticipants));
        assert_eq!(manager.room_count().await, 1);
    }

    #[tokio::test]
    async fn test_delete_room_after_leave() {
        let manager = ChatRoomManager::new();
        manager.create_room("test", "Test Room").await;

        let participant = ChatParticipant::new("session1", Some(1), "Alice");
        let _ = manager.join_room("test", participant).await;

        // Cannot delete while participant is in room
        assert_eq!(
            manager.delete_room("test").await,
            Err(DeleteRoomError::HasParticipants)
        );

        // Leave the room
        manager.leave_room("test", "session1").await;

        // Now can delete
        let result = manager.delete_room("test").await;
        assert!(result.is_ok());
        assert_eq!(manager.room_count().await, 0);
    }
}
