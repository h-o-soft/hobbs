//! Chat module for HOBBS.
//!
//! This module provides real-time chat functionality including:
//! - Chat rooms with broadcast messaging
//! - Participant management (join/leave)
//! - Message types (chat, action, system, join, leave)
//! - Chat commands (/quit, /who, /me, /help)

mod command;
mod room;

pub use command::{
    format_help, format_who, get_command_help, parse_input, ChatCommand, ChatInput, CommandInfo,
};
pub use room::{ChatMessage, ChatParticipant, ChatRoom, MessageType};
