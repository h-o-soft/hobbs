//! Board module for HOBBS.
//!
//! This module provides bulletin board functionality including:
//! - Board management (create, read, update, delete)
//! - Board types (thread-based and flat)
//! - Role-based access control for read/write permissions

mod repository;
mod types;

pub use repository::BoardRepository;
pub use types::{Board, BoardType, BoardUpdate, NewBoard};
