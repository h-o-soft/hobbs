//! HOBBS - Hobbyist Bulletin Board System
//!
//! A retro BBS host program accessible via Telnet, implemented in Rust.

pub mod admin;
pub mod app;
pub mod auth;
pub mod board;
pub mod chat;
pub mod config;
pub mod datetime;
pub mod db;
pub mod error;
pub mod file;
pub mod i18n;
pub mod logging;
pub mod mail;
pub mod rate_limit;
pub mod rss;
pub mod screen;
pub mod script;
pub mod server;
pub mod template;
pub mod terminal;
pub mod web;
pub mod xmodem;

pub use admin::{
    can_change_role, can_edit_user, format_duration, format_session_state, generate_password,
    is_admin, is_sysop, require_admin, AdminError, AdminService, BoardAdminService,
    ContentAdminService, CreateBoardRequest, FolderAdminService, PostDeletionMode,
    SessionAdminService, SessionStatistics, UserAdminService, UserDetail, DEFAULT_PASSWORD_LENGTH,
    DELETED_POST_MESSAGE,
};
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
pub use datetime::{format_datetime, format_datetime_default};
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
pub use rss::{
    fetch_feed, start_rss_updater, start_rss_updater_with_config, start_rss_updater_with_interval,
    validate_url, AddFeedRequest, NewRssFeed, NewRssItem, ParsedFeed, ParsedItem, RssFeed,
    RssFeedRepository, RssFeedUpdate, RssFeedWithUnread, RssFetcher, RssItem, RssItemRepository,
    RssReadPosition, RssReadPositionRepository, RssService, RssUpdater,
    DEFAULT_CHECK_INTERVAL_SECS, DEFAULT_FETCH_INTERVAL, MAX_CONSECUTIVE_ERRORS,
    MAX_DESCRIPTION_LENGTH as MAX_RSS_DESCRIPTION_LENGTH, MAX_FEED_SIZE, MAX_ITEMS_PER_FEED,
};
pub use screen::{
    create_screen, create_screen_from_profile, AnsiScreen, Color, PlainScreen, Screen,
};
pub use server::{
    decode_from_client, decode_shiftjis, decode_shiftjis_strict, encode_for_client,
    encode_shiftjis, encode_shiftjis_strict, initial_negotiation, CharacterEncoding, DecodeResult,
    EchoMode, EncodeResult, InputResult, LineBuffer, MultiLineBuffer, NegotiationState,
    SessionInfo, SessionManager as TelnetSessionManager, SessionState, TelnetCommand, TelnetParser,
    TelnetServer, TelnetSession,
};
pub use terminal::TerminalProfile;

pub use i18n::{I18n, I18nError, I18nManager, DEFAULT_LOCALE};
pub use template::{
    create_system_context, Node, Parser, Renderer, TemplateContext, TemplateEngine, TemplateError,
    TemplateLoader, Value, WIDTH_40, WIDTH_80,
};

pub use app::{Application, MenuAction, MenuError, SessionHandler};
pub use script::{
    BbsApi, ExecutionResult, ResourceLimits, Script, ScriptContext, ScriptEngine, ScriptLoader,
    ScriptMetadata, ScriptRepository, ScriptService, SyncResult,
};
pub use rate_limit::{ActionRateLimiter, RateLimitConfig, RateLimitResult, RateLimiters};
pub use xmodem::{xmodem_receive, xmodem_send, TransferError, TransferResult};
