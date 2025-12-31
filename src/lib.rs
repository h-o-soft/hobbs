//! HOBBS - Hobbyist Bulletin Board System
//!
//! A retro BBS host program accessible via Telnet, implemented in Rust.

pub mod auth;
pub mod board;
pub mod config;
pub mod db;
pub mod error;
pub mod logging;
pub mod server;
pub mod terminal;

pub use auth::{
    can_modify_resource, change_password, check_permission, get_profile, get_profile_by_username,
    hash_password, register, register_with_role, require_member, require_subop, require_sysop,
    reset_password, update_profile, validate_password, verify_password, AuthSession, LimitResult,
    LoginLimiter, PasswordError, PermissionError, ProfileError, ProfileUpdateRequest,
    RegistrationError, RegistrationRequest, SessionError, SessionManager, UserProfile,
    ValidationError, MAX_PROFILE_LENGTH,
};
pub use board::{Board, BoardRepository, BoardType, BoardUpdate, NewBoard};
pub use config::Config;
pub use db::{Database, NewUser, Role, User, UserRepository, UserUpdate};
pub use error::{HobbsError, Result};
pub use server::{
    decode_from_client, decode_shiftjis, decode_shiftjis_strict, encode_for_client,
    encode_shiftjis, encode_shiftjis_strict, initial_negotiation, CharacterEncoding, DecodeResult,
    EchoMode, EncodeResult, InputResult, LineBuffer, MultiLineBuffer, NegotiationState,
    TelnetCommand, TelnetParser, TelnetServer,
};
pub use terminal::TerminalProfile;
