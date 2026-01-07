//! API handlers for Web UI.

pub mod admin;
pub mod auth;
pub mod board;
pub mod config;
pub mod file;
pub mod mail;
pub mod rss;
pub mod user;

pub use admin::*;
pub use auth::*;
pub use board::*;
pub use config::*;
pub use file::*;
pub use mail::*;
pub use rss::*;
pub use user::*;
