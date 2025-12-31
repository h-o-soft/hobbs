//! HOBBS - Hobbyist Bulletin Board System
//!
//! A retro BBS host program accessible via Telnet, implemented in Rust.

pub mod auth;
pub mod board;
pub mod chat;
pub mod config;
pub mod db;
pub mod error;
pub mod file;
pub mod logging;
pub mod mail;
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
pub use board::{
    Board, BoardRepository, BoardService, BoardType, BoardUpdate, NewBoard, NewFlatPost, NewThread,
    NewThreadPost, PaginatedResult, Pagination, Post, PostRepository, PostUpdate, ReadPosition,
    Thread, ThreadRepository, ThreadUpdate, UnreadRepository,
};
pub use chat::{
    format_help, format_who, get_command_help, parse_input, ChatCommand, ChatInput, ChatLog,
    ChatLogRepository, ChatMessage, ChatParticipant, ChatRoom, CommandInfo, MessageType,
    NewChatLog, DEFAULT_RECENT_LOG_COUNT,
};
pub use config::Config;
pub use db::{Database, NewUser, Role, User, UserRepository, UserUpdate};
pub use error::{HobbsError, Result};
pub use file::{
    DownloadResult, FileMetadata, FileRepository, FileService, FileStorage, FileUpdate, Folder,
    FolderRepository, FolderUpdate, NewFile, NewFolder, UploadRequest, DEFAULT_MAX_FILE_SIZE,
    MAX_DESCRIPTION_LENGTH, MAX_FILENAME_LENGTH, MAX_FOLDER_DEPTH,
};
pub use mail::{
    Mail, MailRepository, MailService, MailUpdate, NewMail, SendMailRequest, SystemMailService,
    MAX_BODY_LENGTH as MAX_MAIL_BODY_LENGTH, MAX_SUBJECT_LENGTH as MAX_MAIL_SUBJECT_LENGTH,
    WELCOME_MAIL_BODY, WELCOME_MAIL_SUBJECT,
};
pub use server::{
    decode_from_client, decode_shiftjis, decode_shiftjis_strict, encode_for_client,
    encode_shiftjis, encode_shiftjis_strict, initial_negotiation, CharacterEncoding, DecodeResult,
    EchoMode, EncodeResult, InputResult, LineBuffer, MultiLineBuffer, NegotiationState,
    TelnetCommand, TelnetParser, TelnetServer,
};
pub use terminal::TerminalProfile;
