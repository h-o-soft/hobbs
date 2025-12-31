//! HOBBS - Hobbyist Bulletin Board System
//!
//! A retro BBS host program accessible via Telnet, implemented in Rust.

pub mod config;
pub mod error;
pub mod logging;
pub mod server;
pub mod terminal;

pub use config::Config;
pub use error::{HobbsError, Result};
pub use server::{
    decode_shiftjis, decode_shiftjis_strict, encode_shiftjis, encode_shiftjis_strict,
    initial_negotiation, DecodeResult, EchoMode, EncodeResult, InputResult, LineBuffer,
    MultiLineBuffer, NegotiationState, TelnetCommand, TelnetParser, TelnetServer,
};
pub use terminal::TerminalProfile;
