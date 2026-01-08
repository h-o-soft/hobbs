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
    convert_ansi_to_petscii_ctrl, convert_caret_escape, decode_cp437, decode_from_client,
    decode_from_client_detailed, decode_petscii, decode_shiftjis, decode_shiftjis_strict,
    encode_cp437, encode_for_client, encode_for_client_detailed, encode_petscii, encode_shiftjis,
    encode_shiftjis_strict, process_output_mode, strip_ansi_sequences, CharacterEncoding,
    DecodeResult, EncodeResult, OutputMode,
};
pub use input::{EchoMode, InputResult, LineBuffer, MultiLineBuffer};
pub use listener::{ConnectionPermit, TelnetServer};
pub use session::{SessionInfo, SessionManager, SessionState, TelnetSession};
pub use telnet::{iac, initial_negotiation, option, NegotiationState, TelnetCommand, TelnetParser};
