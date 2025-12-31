//! HOBBS - Hobbyist Bulletin Board System
//!
//! A retro BBS host program accessible via Telnet, implemented in Rust.

pub mod config;
pub mod error;
pub mod logging;
pub mod server;

pub use config::Config;
pub use error::{HobbsError, Result};
pub use server::{
    decode_shiftjis, decode_shiftjis_strict, encode_shiftjis, encode_shiftjis_strict, DecodeResult,
    EncodeResult, TelnetServer,
};
