//! Terminal profile module.
//!
//! This module provides terminal profile definitions that describe the
//! characteristics of different terminal types (screen size, CJK width,
//! ANSI support, etc.).

mod profile;
pub mod width;

pub use profile::TerminalProfile;
