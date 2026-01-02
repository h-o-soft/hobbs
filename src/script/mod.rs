//! Script plugin module for executing Lua scripts.
//!
//! This module provides the ability to run Lua scripts within a sandboxed
//! environment, allowing SysOps and SubOps to create games and interactive
//! content (door games).

pub mod loader;
pub mod repository;
pub mod types;

pub use loader::ScriptLoader;
pub use repository::ScriptRepository;
pub use types::{Script, ScriptMetadata, SyncResult};
