//! Screen modules for HOBBS.
//!
//! This module provides individual screen handlers for different features.

mod admin;
mod board;
mod chat;
mod common;
mod file;
mod help;
mod mail;
mod member;
mod profile;
mod rss;
mod script;

pub use admin::AdminScreen;
pub use board::BoardScreen;
pub use chat::ChatScreen;
pub use common::ScreenContext;
pub use file::FileScreen;
pub use help::HelpScreen;
pub use mail::MailScreen;
pub use member::MemberScreen;
pub use profile::ProfileScreen;
pub use rss::RssScreen;
pub use script::ScriptScreen;

use crate::server::CharacterEncoding;

/// Result of a screen action.
#[derive(Debug, Clone, PartialEq)]
pub enum ScreenResult {
    /// Go back to the previous screen.
    Back,
    /// Continue in the current screen.
    Continue,
    /// User wants to logout.
    Logout,
    /// User wants to quit.
    Quit,
    /// User changed language/encoding/terminal settings.
    SettingsChanged {
        /// New language setting (e.g., "en", "ja").
        language: String,
        /// New character encoding setting.
        encoding: CharacterEncoding,
        /// New terminal profile (if changed).
        terminal_profile: Option<String>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_screen_result_variants() {
        assert_eq!(ScreenResult::Back, ScreenResult::Back);
        assert_eq!(ScreenResult::Continue, ScreenResult::Continue);
        assert_eq!(ScreenResult::Logout, ScreenResult::Logout);
        assert_eq!(ScreenResult::Quit, ScreenResult::Quit);
    }
}
