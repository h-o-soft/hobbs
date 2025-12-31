//! Mail module for HOBBS.
//!
//! This module provides internal mail functionality including:
//! - Mail sending and receiving
//! - Inbox and sent mail management
//! - Read/unread status tracking
//! - Logical deletion (per sender/recipient)

mod repository;
mod types;

pub use repository::MailRepository;
pub use types::{Mail, MailUpdate, NewMail, MAX_BODY_LENGTH, MAX_SUBJECT_LENGTH};
