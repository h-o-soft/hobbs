//! Chat module for HOBBS.
//!
//! This module provides real-time chat functionality including:
//! - Chat rooms with broadcast messaging
//! - Participant management (join/leave)
//! - Message types (chat, action, system, join, leave)

mod room;

pub use room::{ChatMessage, ChatParticipant, ChatRoom, MessageType};
