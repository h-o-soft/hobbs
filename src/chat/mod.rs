//! Chat module for HOBBS.
//!
//! This module provides real-time chat functionality including:
//! - Chat rooms with broadcast messaging
//! - Participant management (join/leave)
//! - Message types (chat, action, system, join, leave)
//! - Chat commands (/quit, /who, /me, /help)
//! - Chat log storage and retrieval
//! - Room management

mod command;
mod log;
mod manager;
mod room;

pub use command::{
    format_help, format_who, get_command_help, parse_input, ChatCommand, ChatInput, CommandInfo,
};
pub use log::{ChatLog, ChatLogRepository, NewChatLog, DEFAULT_RECENT_LOG_COUNT};
pub use manager::{ChatRoomManager, DeleteRoomError, RoomInfo};
pub use room::{
    ChatMessage, ChatParticipant, ChatRoom, JoinResult, MessageType, MAX_PARTICIPANTS_PER_ROOM,
};
