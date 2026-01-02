//! Script plugin module for executing Lua scripts.
//!
//! This module provides the ability to run Lua scripts within a sandboxed
//! environment, allowing SysOps and SubOps to create games and interactive
//! content (door games).

pub mod api;
pub mod data_repository;
pub mod engine;
pub mod loader;
pub mod log_repository;
pub mod repository;
pub mod service;
pub mod types;

pub use api::BbsApi;
pub use data_repository::ScriptDataRepository;
pub use engine::{ResourceLimits, ScriptContext, ScriptEngine};
pub use loader::ScriptLoader;
pub use log_repository::{ScriptLog, ScriptLogRepository};
pub use repository::ScriptRepository;
pub use service::{ExecutionResult, ScriptService};
pub use types::{Script, ScriptMetadata, SyncResult};
