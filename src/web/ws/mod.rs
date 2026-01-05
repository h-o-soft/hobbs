//! WebSocket module for real-time communication.
//!
//! This module provides WebSocket support for:
//! - Real-time chat communication
//! - Interoperability with Telnet clients

pub mod chat;
pub mod messages;

pub use chat::{chat_ws_handler, ChatWsState};
pub use messages::{ClientMessage, ServerMessage};
