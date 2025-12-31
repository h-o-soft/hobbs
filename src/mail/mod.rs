//! Mail module for HOBBS.
//!
//! This module provides internal mail functionality including:
//! - Mail sending and receiving
//! - Inbox and sent mail management
//! - Read/unread status tracking
//! - Logical deletion (per sender/recipient)
//! - System-generated mail (welcome, notifications)

mod repository;
mod service;
mod system;
mod types;

pub use repository::MailRepository;
pub use service::{MailService, SendMailRequest};
pub use system::{SystemMailService, WELCOME_MAIL_BODY, WELCOME_MAIL_SUBJECT};
pub use types::{Mail, MailUpdate, NewMail, MAX_BODY_LENGTH, MAX_SUBJECT_LENGTH};
