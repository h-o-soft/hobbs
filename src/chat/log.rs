//! Chat log storage for HOBBS.
//!
//! This module provides functionality to store and retrieve chat message logs
//! from the database.

use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};

use super::room::MessageType;

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
pub struct ChatLogRepository;

impl ChatLogRepository {
    /// Save a chat log entry.
    pub fn save(conn: &Connection, log: &NewChatLog) -> rusqlite::Result<i64> {
        conn.execute(
            r#"
            INSERT INTO chat_logs (room_id, user_id, sender_name, message_type, content)
            VALUES (?1, ?2, ?3, ?4, ?5)
            "#,
            params![
                log.room_id,
                log.user_id,
                log.sender_name,
                log.message_type.as_str(),
                log.content,
            ],
        )?;
        Ok(conn.last_insert_rowid())
    }

    /// Get a log entry by ID.
    pub fn get_by_id(conn: &Connection, id: i64) -> rusqlite::Result<Option<ChatLog>> {
        conn.query_row(
            r#"
            SELECT id, room_id, user_id, sender_name, message_type, content, created_at
            FROM chat_logs
            WHERE id = ?1
            "#,
            [id],
            Self::map_row,
        )
        .optional()
    }

    /// Get recent logs for a room.
    ///
    /// Returns logs in chronological order (oldest first).
    pub fn get_recent(
        conn: &Connection,
        room_id: &str,
        limit: usize,
    ) -> rusqlite::Result<Vec<ChatLog>> {
        let mut stmt = conn.prepare(
            r#"
            SELECT id, room_id, user_id, sender_name, message_type, content, created_at
            FROM chat_logs
            WHERE room_id = ?1
            ORDER BY created_at DESC, id DESC
            LIMIT ?2
            "#,
        )?;

        let logs: Vec<ChatLog> = stmt
            .query_map(params![room_id, limit as i64], Self::map_row)?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        // Reverse to get chronological order
        Ok(logs.into_iter().rev().collect())
    }

    /// Get logs for a room after a specific timestamp.
    ///
    /// Returns logs in chronological order (oldest first).
    pub fn get_since(
        conn: &Connection,
        room_id: &str,
        since: DateTime<Utc>,
    ) -> rusqlite::Result<Vec<ChatLog>> {
        let mut stmt = conn.prepare(
            r#"
            SELECT id, room_id, user_id, sender_name, message_type, content, created_at
            FROM chat_logs
            WHERE room_id = ?1 AND created_at > ?2
            ORDER BY created_at ASC, id ASC
            "#,
        )?;

        let logs: Vec<ChatLog> = stmt
            .query_map(params![room_id, since.to_rfc3339()], Self::map_row)?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        Ok(logs)
    }

    /// Count logs for a room.
    pub fn count(conn: &Connection, room_id: &str) -> rusqlite::Result<i64> {
        conn.query_row(
            "SELECT COUNT(*) FROM chat_logs WHERE room_id = ?1",
            [room_id],
            |row| row.get(0),
        )
    }

    /// Delete logs older than a specific timestamp.
    pub fn delete_before(conn: &Connection, before: DateTime<Utc>) -> rusqlite::Result<usize> {
        conn.execute(
            "DELETE FROM chat_logs WHERE created_at < ?1",
            [before.to_rfc3339()],
        )
    }

    /// Delete all logs for a room.
    pub fn delete_room(conn: &Connection, room_id: &str) -> rusqlite::Result<usize> {
        conn.execute("DELETE FROM chat_logs WHERE room_id = ?1", [room_id])
    }

    /// Map a database row to a ChatLog.
    fn map_row(row: &rusqlite::Row) -> rusqlite::Result<ChatLog> {
        let message_type_str: String = row.get(4)?;
        let message_type = match message_type_str.as_str() {
            "chat" => MessageType::Chat,
            "action" => MessageType::Action,
            "system" => MessageType::System,
            "join" => MessageType::Join,
            "leave" => MessageType::Leave,
            _ => MessageType::Chat, // Default fallback
        };

        let created_at_str: String = row.get(6)?;
        let created_at = DateTime::parse_from_rfc3339(&created_at_str)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        Ok(ChatLog {
            id: row.get(0)?,
            room_id: row.get(1)?,
            user_id: row.get(2)?,
            sender_name: row.get(3)?,
            message_type,
            content: row.get(5)?,
            created_at,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{Database, NewUser, UserRepository};

    fn setup_db() -> Database {
        Database::open_in_memory().unwrap()
    }

    fn create_test_user(db: &Database) -> i64 {
        let repo = UserRepository::new(db);
        let user = NewUser::new("testuser", "password123", "Test User");
        repo.create(&user).unwrap().id
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

    #[test]
    fn test_save_and_get_by_id() {
        let db = setup_db();
        let user_id = create_test_user(&db);
        let log = NewChatLog::chat("lobby", user_id, "Alice", "Hello!");

        let id = ChatLogRepository::save(db.conn(), &log).unwrap();
        assert!(id > 0);

        let retrieved = ChatLogRepository::get_by_id(db.conn(), id)
            .unwrap()
            .unwrap();

        assert_eq!(retrieved.id, id);
        assert_eq!(retrieved.room_id, "lobby");
        assert_eq!(retrieved.user_id, Some(user_id));
        assert_eq!(retrieved.sender_name, "Alice");
        assert_eq!(retrieved.message_type, MessageType::Chat);
        assert_eq!(retrieved.content, "Hello!");
    }

    #[test]
    fn test_get_by_id_not_found() {
        let db = setup_db();
        let result = ChatLogRepository::get_by_id(db.conn(), 999).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_get_recent() {
        let db = setup_db();
        let user_id = create_test_user(&db);

        // Create multiple logs
        for i in 1..=5 {
            let log = NewChatLog::chat("lobby", user_id, "Alice", format!("Message {i}"));
            ChatLogRepository::save(db.conn(), &log).unwrap();
        }

        // Get recent 3
        let logs = ChatLogRepository::get_recent(db.conn(), "lobby", 3).unwrap();

        assert_eq!(logs.len(), 3);
        // Should be in chronological order (oldest first)
        assert!(logs[0].content.contains("3"));
        assert!(logs[1].content.contains("4"));
        assert!(logs[2].content.contains("5"));
    }

    #[test]
    fn test_get_recent_empty() {
        let db = setup_db();
        let logs = ChatLogRepository::get_recent(db.conn(), "lobby", 10).unwrap();
        assert!(logs.is_empty());
    }

    #[test]
    fn test_get_recent_different_rooms() {
        let db = setup_db();
        let user_id = create_test_user(&db);

        ChatLogRepository::save(
            db.conn(),
            &NewChatLog::chat("lobby", user_id, "Alice", "Lobby msg"),
        )
        .unwrap();
        ChatLogRepository::save(
            db.conn(),
            &NewChatLog::chat("room2", user_id, "Alice", "Room2 msg"),
        )
        .unwrap();

        let lobby_logs = ChatLogRepository::get_recent(db.conn(), "lobby", 10).unwrap();
        let room2_logs = ChatLogRepository::get_recent(db.conn(), "room2", 10).unwrap();

        assert_eq!(lobby_logs.len(), 1);
        assert_eq!(room2_logs.len(), 1);
        assert!(lobby_logs[0].content.contains("Lobby"));
        assert!(room2_logs[0].content.contains("Room2"));
    }

    #[test]
    fn test_count() {
        let db = setup_db();
        let user_id = create_test_user(&db);

        assert_eq!(ChatLogRepository::count(db.conn(), "lobby").unwrap(), 0);

        for i in 1..=3 {
            let log = NewChatLog::chat("lobby", user_id, "Alice", format!("Message {i}"));
            ChatLogRepository::save(db.conn(), &log).unwrap();
        }

        assert_eq!(ChatLogRepository::count(db.conn(), "lobby").unwrap(), 3);
    }

    #[test]
    fn test_delete_room() {
        let db = setup_db();
        let user_id = create_test_user(&db);

        ChatLogRepository::save(
            db.conn(),
            &NewChatLog::chat("lobby", user_id, "Alice", "Msg 1"),
        )
        .unwrap();
        ChatLogRepository::save(
            db.conn(),
            &NewChatLog::chat("lobby", user_id, "Alice", "Msg 2"),
        )
        .unwrap();
        ChatLogRepository::save(
            db.conn(),
            &NewChatLog::chat("room2", user_id, "Alice", "Msg 3"),
        )
        .unwrap();

        let deleted = ChatLogRepository::delete_room(db.conn(), "lobby").unwrap();
        assert_eq!(deleted, 2);

        assert_eq!(ChatLogRepository::count(db.conn(), "lobby").unwrap(), 0);
        assert_eq!(ChatLogRepository::count(db.conn(), "room2").unwrap(), 1);
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

    #[test]
    fn test_save_all_message_types() {
        let db = setup_db();
        let user_id = create_test_user(&db);

        let logs = vec![
            NewChatLog::chat("lobby", user_id, "Alice", "Hello"),
            NewChatLog::action("lobby", user_id, "Alice", "waves"),
            NewChatLog::system("lobby", "Notice"),
            NewChatLog::join("lobby", Some(user_id), "Alice"),
            NewChatLog::leave("lobby", Some(user_id), "Alice"),
        ];

        for log in &logs {
            ChatLogRepository::save(db.conn(), log).unwrap();
        }

        let retrieved = ChatLogRepository::get_recent(db.conn(), "lobby", 10).unwrap();

        assert_eq!(retrieved.len(), 5);
        assert_eq!(retrieved[0].message_type, MessageType::Chat);
        assert_eq!(retrieved[1].message_type, MessageType::Action);
        assert_eq!(retrieved[2].message_type, MessageType::System);
        assert_eq!(retrieved[3].message_type, MessageType::Join);
        assert_eq!(retrieved[4].message_type, MessageType::Leave);
    }

    #[test]
    fn test_save_system_message_null_user() {
        let db = setup_db();
        let log = NewChatLog::system("lobby", "Server maintenance");

        let id = ChatLogRepository::save(db.conn(), &log).unwrap();

        let retrieved = ChatLogRepository::get_by_id(db.conn(), id)
            .unwrap()
            .unwrap();

        assert!(retrieved.user_id.is_none());
    }
}
