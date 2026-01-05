//! API handlers for Web UI.

pub mod auth;
pub mod board;
pub mod mail;
pub mod rss;
pub mod user;

pub use auth::*;
pub use board::*;
pub use mail::*;
pub use rss::*;
pub use user::*;
