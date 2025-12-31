//! Board module for HOBBS.
//!
//! This module provides bulletin board functionality including:
//! - Board management (create, read, update, delete)
//! - Thread management for thread-based boards
//! - Post management for both thread and flat boards
//! - Board types (thread-based and flat)
//! - Role-based access control for read/write permissions

mod post;
mod post_repository;
mod repository;
mod thread;
mod thread_repository;
mod types;

pub use post::{NewFlatPost, NewThreadPost, Post, PostUpdate};
pub use post_repository::PostRepository;
pub use repository::BoardRepository;
pub use thread::{NewThread, Thread, ThreadUpdate};
pub use thread_repository::ThreadRepository;
pub use types::{Board, BoardType, BoardUpdate, NewBoard};
