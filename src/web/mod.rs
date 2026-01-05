//! Web API module for HOBBS.
//!
//! This module provides a REST API and WebSocket interface for the BBS,
//! allowing browser-based access alongside the traditional Telnet interface.

pub mod dto;
pub mod error;
pub mod handlers;
pub mod middleware;
pub mod router;
pub mod server;

pub use error::ApiError;
pub use router::create_router;
pub use server::WebServer;
