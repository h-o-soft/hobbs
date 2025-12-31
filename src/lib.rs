//! HOBBS - Hobbyist Bulletin Board System
//!
//! A retro BBS host program accessible via Telnet, implemented in Rust.

pub mod config;
pub mod error;

pub use config::Config;
pub use error::{HobbsError, Result};
