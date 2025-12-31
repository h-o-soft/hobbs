//! Telnet server module.
//!
//! This module provides the TCP listener and connection handling for the
//! Telnet server.

mod listener;

pub use listener::{ConnectionPermit, TelnetServer};
