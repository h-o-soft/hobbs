//! Telnet server module.
//!
//! This module provides the TCP listener and connection handling for the
//! Telnet server.

pub mod encoding;
pub mod input;
mod listener;
mod session;
pub mod telnet;

pub use encoding::{
    decode_shiftjis, decode_shiftjis_strict, encode_shiftjis, encode_shiftjis_strict, DecodeResult,
    EncodeResult,
};
pub use input::{EchoMode, InputResult, LineBuffer, MultiLineBuffer};
pub use listener::{ConnectionPermit, TelnetServer};
pub use session::{SessionInfo, SessionManager, SessionState, TelnetSession};
pub use telnet::{
    iac, initial_negotiation, option, NegotiationState, TelnetCommand, TelnetParser,
};
