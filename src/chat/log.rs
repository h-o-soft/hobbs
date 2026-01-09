//! Chat log storage for HOBBS.
//!
//! This module provides functionality to store and retrieve chat message logs
//! from the database.

use chrono::{DateTime, Utc};

use super::room::MessageType;
use crate::db::DbPool;
use crate::{HobbsError, Result};

/// Default number of recent logs to retrieve.
pub const DEFAULT_RECENT_LOG_COUNT: usize = 20;

/// A stored chat log entry.
#[derive(Debug, Clone)]
pub struct ChatLog {
    /// Log entry ID.
    pub id: i64,
    /// Room ID where the message was sent.
    pub room_id: String,
    /// User ID of the sender (None for system messages).
    pub user_id: Option<i64>,
    /// Display name of the sender at the time of sending.
    pub sender_name: String,
    /// Type of the message.
    pub message_type: MessageType,
    /// Message content.
    pub content: String,
    /// Timestamp when the message was created.
    pub created_at: DateTime<Utc>,
}

impl ChatLog {
    /// Format the log entry for display.
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

/// Database row type for ChatLog.
#[derive(sqlx::FromRow)]
struct ChatLogRow {
    id: i64,
    room_id: String,
    user_id: Option<i64>,
    sender_name: String,
    message_type: String,
    content: String,
    created_at: String,
}

impl From<ChatLogRow> for ChatLog {
    fn from(row: ChatLogRow) -> Self {
        let message_type = match row.message_type.as_str() {
            "chat" => MessageType::Chat,
            "action" => MessageType::Action,
            "system" => MessageType::System,
            "join" => MessageType::Join,
            "leave" => MessageType::Leave,
            _ => MessageType::Chat, // Default fallback
        };

        let created_at = DateTime::parse_from_rfc3339(&row.created_at)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        Self {
            id: row.id,
            room_id: row.room_id,
            user_id: row.user_id,
            sender_name: row.sender_name,
            message_type,
            content: row.content,
            created_at,
        }
    }
}

/// New chat log entry for insertion.
#[derive(Debug, Clone)]
pub struct NewChatLog {
    /// Room ID.
    pub room_id: String,
    /// User ID (None for system messages).
    pub user_id: Option<i64>,
    /// Sender display name.
    pub sender_name: String,
    /// Message type.
    pub message_type: MessageType,
    /// Message content.
    pub content: String,
}

impl NewChatLog {
    /// Create a new chat log entry.
    pub fn new(
        room_id: impl Into<String>,
        user_id: Option<i64>,
        sender_name: impl Into<String>,
        message_type: MessageType,
        content: impl Into<String>,
    ) -> Self {
        Self {
            room_id: room_id.into(),
            user_id,
            sender_name: sender_name.into(),
            message_type,
            content: content.into(),
        }
    }

    /// Create a chat message log.
    pub fn chat(
        room_id: impl Into<String>,
        user_id: i64,
        sender_name: impl Into<String>,
        content: impl Into<String>,
    ) -> Self {
        Self::new(
            room_id,
            Some(user_id),
            sender_name,
            MessageType::Chat,
            content,
        )
    }

    /// Create an action message log.
    pub fn action(
        room_id: impl Into<String>,
        user_id: i64,
        sender_name: impl Into<String>,
        content: impl Into<String>,
    ) -> Self {
        Self::new(
            room_id,
            Some(user_id),
            sender_name,
            MessageType::Action,
            content,
        )
    }

    /// Create a system message log.
    pub fn system(room_id: impl Into<String>, content: impl Into<String>) -> Self {
        Self::new(room_id, None, "", MessageType::System, content)
    }

    /// Create a join notification log.
    pub fn join(
        room_id: impl Into<String>,
        user_id: Option<i64>,
        sender_name: impl Into<String>,
    ) -> Self {
        let name = sender_name.into();
        let content = format!("{name} が入室しました");
        Self::new(room_id, user_id, &name, MessageType::Join, content)
    }

    /// Create a leave notification log.
    pub fn leave(
        room_id: impl Into<String>,
        user_id: Option<i64>,
        sender_name: impl Into<String>,
    ) -> Self {
        let name = sender_name.into();
        let content = format!("{name} が退室しました");
        Self::new(room_id, user_id, &name, MessageType::Leave, content)
    }
}

/// Repository for chat log operations.
pub struct ChatLogRepository<'a> {
    pool: &'a DbPool,
}

impl<'a> ChatLogRepository<'a> {
    /// Create a new ChatLogRepository with the given database pool reference.
    pub fn new(pool: &'a DbPool) -> Self {
        Self { pool }
    }

    /// Save a chat log entry.
    pub async fn save(&self, log: &NewChatLog) -> Result<i64> {
        let id: i64 = sqlx::query_scalar(
            r#"
            INSERT INTO chat_logs (room_id, user_id, sender_name, message_type, content)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id
            "#,
        )
        .bind(&log.room_id)
        .bind(log.user_id)
        .bind(&log.sender_name)
        .bind(log.message_type.as_str())
        .bind(&log.content)
        .fetch_one(self.pool)
        .await
        .map_err(|e| HobbsError::Database(e.to_string()))?;

        Ok(id)
    }

    /// Get a log entry by ID.
    pub async fn get_by_id(&self, id: i64) -> Result<Option<ChatLog>> {
        let result = sqlx::query_as::<_, ChatLogRow>(
            r#"
            SELECT id, room_id, user_id, sender_name, message_type, content, created_at
            FROM chat_logs
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(self.pool)
        .await
        .map_err(|e| HobbsError::Database(e.to_string()))?;

        Ok(result.map(ChatLog::from))
    }

    /// Get recent logs for a room.
    ///
    /// Returns logs in chronological order (oldest first).
    pub async fn get_recent(&self, room_id: &str, limit: usize) -> Result<Vec<ChatLog>> {
        let rows = sqlx::query_as::<_, ChatLogRow>(
            r#"
            SELECT id, room_id, user_id, sender_name, message_type, content, created_at
            FROM chat_logs
            WHERE room_id = $1
            ORDER BY created_at DESC, id DESC
            LIMIT $2
            "#,
        )
        .bind(room_id)
        .bind(limit as i64)
        .fetch_all(self.pool)
        .await
        .map_err(|e| HobbsError::Database(e.to_string()))?;

        // Reverse to get chronological order
        let logs: Vec<ChatLog> = rows.into_iter().map(ChatLog::from).rev().collect();
        Ok(logs)
    }

    /// Get logs for a room after a specific timestamp.
    ///
    /// Returns logs in chronological order (oldest first).
    pub async fn get_since(&self, room_id: &str, since: DateTime<Utc>) -> Result<Vec<ChatLog>> {
        let rows = sqlx::query_as::<_, ChatLogRow>(
            r#"
            SELECT id, room_id, user_id, sender_name, message_type, content, created_at
            FROM chat_logs
            WHERE room_id = $1 AND created_at > $2
            ORDER BY created_at ASC, id ASC
            "#,
        )
        .bind(room_id)
        .bind(since.to_rfc3339())
        .fetch_all(self.pool)
        .await
        .map_err(|e| HobbsError::Database(e.to_string()))?;

        Ok(rows.into_iter().map(ChatLog::from).collect())
    }

    /// Count logs for a room.
    pub async fn count(&self, room_id: &str) -> Result<i64> {
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM chat_logs WHERE room_id = $1")
            .bind(room_id)
            .fetch_one(self.pool)
            .await
            .map_err(|e| HobbsError::Database(e.to_string()))?;

        Ok(count.0)
    }

    /// Delete logs older than a specific timestamp.
    pub async fn delete_before(&self, before: DateTime<Utc>) -> Result<usize> {
        let result = sqlx::query("DELETE FROM chat_logs WHERE created_at < $1")
            .bind(before.to_rfc3339())
            .execute(self.pool)
            .await
            .map_err(|e| HobbsError::Database(e.to_string()))?;

        Ok(result.rows_affected() as usize)
    }

    /// Delete all logs for a room.
    pub async fn delete_room(&self, room_id: &str) -> Result<usize> {
        let result = sqlx::query("DELETE FROM chat_logs WHERE room_id = $1")
            .bind(room_id)
            .execute(self.pool)
            .await
            .map_err(|e| HobbsError::Database(e.to_string()))?;

        Ok(result.rows_affected() as usize)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{Database, NewUser, UserRepository};

    async fn setup_db() -> Database {
        Database::open_in_memory().await.unwrap()
    }

    async fn create_test_user(db: &Database) -> i64 {
        let repo = UserRepository::new(db.pool());
        let user = NewUser::new("testuser", "password123", "Test User");
        repo.create(&user).await.unwrap().id
    }

    #[test]
    fn test_new_chat_log() {
        let log = NewChatLog::new("lobby", Some(1), "Alice", MessageType::Chat, "Hello!");
        assert_eq!(log.room_id, "lobby");
        assert_eq!(log.user_id, Some(1));
        assert_eq!(log.sender_name, "Alice");
        assert_eq!(log.message_type, MessageType::Chat);
        assert_eq!(log.content, "Hello!");
    }

    #[test]
    fn test_new_chat_log_chat() {
        let log = NewChatLog::chat("lobby", 1, "Alice", "Hello!");
        assert_eq!(log.message_type, MessageType::Chat);
        assert_eq!(log.user_id, Some(1));
    }

    #[test]
    fn test_new_chat_log_action() {
        let log = NewChatLog::action("lobby", 1, "Alice", "waves");
        assert_eq!(log.message_type, MessageType::Action);
    }

    #[test]
    fn test_new_chat_log_system() {
        let log = NewChatLog::system("lobby", "Server maintenance");
        assert_eq!(log.message_type, MessageType::System);
        assert!(log.user_id.is_none());
        assert_eq!(log.sender_name, "");
    }

    #[test]
    fn test_new_chat_log_join() {
        let log = NewChatLog::join("lobby", Some(1), "Alice");
        assert_eq!(log.message_type, MessageType::Join);
        assert!(log.content.contains("Alice"));
        assert!(log.content.contains("入室"));
    }

    #[test]
    fn test_new_chat_log_leave() {
        let log = NewChatLog::leave("lobby", Some(1), "Alice");
        assert_eq!(log.message_type, MessageType::Leave);
        assert!(log.content.contains("Alice"));
        assert!(log.content.contains("退室"));
    }

    #[tokio::test]
    async fn test_save_and_get_by_id() {
        let db = setup_db().await;
        let user_id = create_test_user(&db).await;
        let repo = ChatLogRepository::new(db.pool());
        let log = NewChatLog::chat("lobby", user_id, "Alice", "Hello!");

        let id = repo.save(&log).await.unwrap();
        assert!(id > 0);

        let retrieved = repo.get_by_id(id).await.unwrap().unwrap();

        assert_eq!(retrieved.id, id);
        assert_eq!(retrieved.room_id, "lobby");
        assert_eq!(retrieved.user_id, Some(user_id));
        assert_eq!(retrieved.sender_name, "Alice");
        assert_eq!(retrieved.message_type, MessageType::Chat);
        assert_eq!(retrieved.content, "Hello!");
    }

    #[tokio::test]
    async fn test_get_by_id_not_found() {
        let db = setup_db().await;
        let repo = ChatLogRepository::new(db.pool());
        let result = repo.get_by_id(999).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_get_recent() {
        let db = setup_db().await;
        let user_id = create_test_user(&db).await;
        let repo = ChatLogRepository::new(db.pool());

        // Create multiple logs
        for i in 1..=5 {
            let log = NewChatLog::chat("lobby", user_id, "Alice", format!("Message {i}"));
            repo.save(&log).await.unwrap();
        }

        // Get recent 3
        let logs = repo.get_recent("lobby", 3).await.unwrap();

        assert_eq!(logs.len(), 3);
        // Should be in chronological order (oldest first)
        assert!(logs[0].content.contains("3"));
        assert!(logs[1].content.contains("4"));
        assert!(logs[2].content.contains("5"));
    }

    #[tokio::test]
    async fn test_get_recent_empty() {
        let db = setup_db().await;
        let repo = ChatLogRepository::new(db.pool());
        let logs = repo.get_recent("lobby", 10).await.unwrap();
        assert!(logs.is_empty());
    }

    #[tokio::test]
    async fn test_get_recent_different_rooms() {
        let db = setup_db().await;
        let user_id = create_test_user(&db).await;
        let repo = ChatLogRepository::new(db.pool());

        repo.save(&NewChatLog::chat("lobby", user_id, "Alice", "Lobby msg"))
            .await
            .unwrap();
        repo.save(&NewChatLog::chat("room2", user_id, "Alice", "Room2 msg"))
            .await
            .unwrap();

        let lobby_logs = repo.get_recent("lobby", 10).await.unwrap();
        let room2_logs = repo.get_recent("room2", 10).await.unwrap();

        assert_eq!(lobby_logs.len(), 1);
        assert_eq!(room2_logs.len(), 1);
        assert!(lobby_logs[0].content.contains("Lobby"));
        assert!(room2_logs[0].content.contains("Room2"));
    }

    #[tokio::test]
    async fn test_count() {
        let db = setup_db().await;
        let user_id = create_test_user(&db).await;
        let repo = ChatLogRepository::new(db.pool());

        assert_eq!(repo.count("lobby").await.unwrap(), 0);

        for i in 1..=3 {
            let log = NewChatLog::chat("lobby", user_id, "Alice", format!("Message {i}"));
            repo.save(&log).await.unwrap();
        }

        assert_eq!(repo.count("lobby").await.unwrap(), 3);
    }

    #[tokio::test]
    async fn test_delete_room() {
        let db = setup_db().await;
        let user_id = create_test_user(&db).await;
        let repo = ChatLogRepository::new(db.pool());

        repo.save(&NewChatLog::chat("lobby", user_id, "Alice", "Msg 1"))
            .await
            .unwrap();
        repo.save(&NewChatLog::chat("lobby", user_id, "Alice", "Msg 2"))
            .await
            .unwrap();
        repo.save(&NewChatLog::chat("room2", user_id, "Alice", "Msg 3"))
            .await
            .unwrap();

        let deleted = repo.delete_room("lobby").await.unwrap();
        assert_eq!(deleted, 2);

        assert_eq!(repo.count("lobby").await.unwrap(), 0);
        assert_eq!(repo.count("room2").await.unwrap(), 1);
    }

    #[test]
    fn test_chat_log_format() {
        let log = ChatLog {
            id: 1,
            room_id: "lobby".to_string(),
            user_id: Some(1),
            sender_name: "Alice".to_string(),
            message_type: MessageType::Chat,
            content: "Hello!".to_string(),
            created_at: Utc::now(),
        };
        assert_eq!(log.format(), "<Alice> Hello!");

        let action_log = ChatLog {
            id: 2,
            room_id: "lobby".to_string(),
            user_id: Some(1),
            sender_name: "Alice".to_string(),
            message_type: MessageType::Action,
            content: "waves".to_string(),
            created_at: Utc::now(),
        };
        assert_eq!(action_log.format(), "* Alice waves");

        let system_log = ChatLog {
            id: 3,
            room_id: "lobby".to_string(),
            user_id: None,
            sender_name: "".to_string(),
            message_type: MessageType::System,
            content: "Server notice".to_string(),
            created_at: Utc::now(),
        };
        assert_eq!(system_log.format(), "*** Server notice");
    }

    #[tokio::test]
    async fn test_save_all_message_types() {
        let db = setup_db().await;
        let user_id = create_test_user(&db).await;
        let repo = ChatLogRepository::new(db.pool());

        let logs = vec![
            NewChatLog::chat("lobby", user_id, "Alice", "Hello"),
            NewChatLog::action("lobby", user_id, "Alice", "waves"),
            NewChatLog::system("lobby", "Notice"),
            NewChatLog::join("lobby", Some(user_id), "Alice"),
            NewChatLog::leave("lobby", Some(user_id), "Alice"),
        ];

        for log in &logs {
            repo.save(log).await.unwrap();
        }

        let retrieved = repo.get_recent("lobby", 10).await.unwrap();

        assert_eq!(retrieved.len(), 5);
        assert_eq!(retrieved[0].message_type, MessageType::Chat);
        assert_eq!(retrieved[1].message_type, MessageType::Action);
        assert_eq!(retrieved[2].message_type, MessageType::System);
        assert_eq!(retrieved[3].message_type, MessageType::Join);
        assert_eq!(retrieved[4].message_type, MessageType::Leave);
    }

    #[tokio::test]
    async fn test_save_system_message_null_user() {
        let db = setup_db().await;
        let repo = ChatLogRepository::new(db.pool());
        let log = NewChatLog::system("lobby", "Server maintenance");

        let id = repo.save(&log).await.unwrap();

        let retrieved = repo.get_by_id(id).await.unwrap().unwrap();

        assert!(retrieved.user_id.is_none());
    }
}
