//! Menu handling module.
//!
//! Provides menu actions and parsing for the main menu.

use thiserror::Error;

/// Menu action representing user's choice.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MenuAction {
    /// Go to board list.
    Board,
    /// Go to chat room selection.
    Chat,
    /// Go to mail menu.
    Mail,
    /// Go to file library.
    File,
    /// Go to scripts/games.
    Script,
    /// Go to RSS news reader.
    News,
    /// Go to user profile.
    Profile,
    /// Go to member list.
    MemberList,
    /// Go to admin menu.
    Admin,
    /// Show help.
    Help,
    /// Logout (return to login screen).
    Logout,
    /// Quit (disconnect).
    Quit,
    /// Login (from guest mode).
    Login,
    /// Register new account (from guest mode).
    Register,
    /// Invalid or unknown action.
    Invalid(String),
}

impl MenuAction {
    /// Parse a menu action from user input.
    ///
    /// # Arguments
    ///
    /// * `input` - User input string (case-insensitive).
    /// * `is_logged_in` - Whether the user is logged in.
    /// * `is_admin` - Whether the user has admin privileges.
    ///
    /// # Returns
    ///
    /// The parsed menu action.
    pub fn parse(input: &str, is_logged_in: bool, is_admin: bool) -> Self {
        let input = input.trim().to_uppercase();

        match input.as_str() {
            "B" | "1" => MenuAction::Board,
            "C" | "2" => MenuAction::Chat,
            "M" | "3" if is_logged_in => MenuAction::Mail,
            "F" | "4" => MenuAction::File,
            "D" | "5" => MenuAction::Script,
            "N" | "9" => MenuAction::News,
            "P" | "6" if is_logged_in => MenuAction::Profile,
            "W" | "8" => MenuAction::MemberList,
            "A" | "7" if is_admin => MenuAction::Admin,
            "H" | "?" => MenuAction::Help,
            "L" if is_logged_in => MenuAction::Logout,
            "L" if !is_logged_in => MenuAction::Login,
            "R" if !is_logged_in => MenuAction::Register,
            "Q" | "G" => MenuAction::Quit,
            "" => MenuAction::Invalid(String::new()),
            other => MenuAction::Invalid(other.to_string()),
        }
    }

    /// Check if this action requires login.
    pub fn requires_login(&self) -> bool {
        matches!(
            self,
            MenuAction::Mail | MenuAction::Profile | MenuAction::Logout
        )
    }

    /// Check if this action requires admin privileges.
    pub fn requires_admin(&self) -> bool {
        matches!(self, MenuAction::Admin)
    }

    /// Check if this is a guest-only action.
    pub fn is_guest_only(&self) -> bool {
        matches!(self, MenuAction::Login | MenuAction::Register)
    }

    /// Check if this action is invalid.
    pub fn is_invalid(&self) -> bool {
        matches!(self, MenuAction::Invalid(_))
    }

    /// Get the menu key for this action.
    pub fn key(&self) -> &'static str {
        match self {
            MenuAction::Board => "B",
            MenuAction::Chat => "C",
            MenuAction::Mail => "M",
            MenuAction::File => "F",
            MenuAction::Script => "D",
            MenuAction::News => "N",
            MenuAction::Profile => "P",
            MenuAction::MemberList => "W",
            MenuAction::Admin => "A",
            MenuAction::Help => "H",
            MenuAction::Logout => "L",
            MenuAction::Login => "L",
            MenuAction::Register => "R",
            MenuAction::Quit => "Q",
            MenuAction::Invalid(_) => "",
        }
    }
}

/// Menu-related errors.
#[derive(Debug, Error)]
pub enum MenuError {
    /// Permission denied for the requested action.
    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    /// Invalid menu selection.
    #[error("Invalid selection: {0}")]
    InvalidSelection(String),

    /// Feature not yet implemented.
    #[error("Feature not implemented: {0}")]
    NotImplemented(String),
}

/// Available menu items based on user state.
#[derive(Debug, Clone)]
pub struct MenuItems {
    /// Whether board menu is available.
    pub board: bool,
    /// Whether chat menu is available.
    pub chat: bool,
    /// Whether mail menu is available.
    pub mail: bool,
    /// Whether file menu is available.
    pub file: bool,
    /// Whether news (RSS) menu is available.
    pub news: bool,
    /// Whether profile menu is available.
    pub profile: bool,
    /// Whether member list is available.
    pub member_list: bool,
    /// Whether admin menu is available.
    pub admin: bool,
    /// Whether help is available.
    pub help: bool,
    /// Whether logout is available.
    pub logout: bool,
    /// Whether login is available.
    pub login: bool,
    /// Whether register is available.
    pub register: bool,
    /// Whether quit is available.
    pub quit: bool,
}

impl MenuItems {
    /// Create menu items for a logged-in user.
    pub fn for_member(is_admin: bool) -> Self {
        Self {
            board: true,
            chat: true,
            mail: true,
            file: true,
            news: true,
            profile: true,
            member_list: true,
            admin: is_admin,
            help: true,
            logout: true,
            login: false,
            register: false,
            quit: true,
        }
    }

    /// Create menu items for a guest user.
    pub fn for_guest() -> Self {
        Self {
            board: true,
            chat: true,
            mail: false,
            file: true,
            news: true,
            profile: false,
            member_list: true,
            admin: false,
            help: true,
            logout: false,
            login: true,
            register: true,
            quit: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_board() {
        assert_eq!(MenuAction::parse("B", true, false), MenuAction::Board);
        assert_eq!(MenuAction::parse("b", true, false), MenuAction::Board);
        assert_eq!(MenuAction::parse("1", true, false), MenuAction::Board);
    }

    #[test]
    fn test_parse_chat() {
        assert_eq!(MenuAction::parse("C", true, false), MenuAction::Chat);
        assert_eq!(MenuAction::parse("2", true, false), MenuAction::Chat);
    }

    #[test]
    fn test_parse_mail_logged_in() {
        assert_eq!(MenuAction::parse("M", true, false), MenuAction::Mail);
        assert_eq!(MenuAction::parse("3", true, false), MenuAction::Mail);
    }

    #[test]
    fn test_parse_mail_guest() {
        // Guest cannot access mail
        assert_eq!(
            MenuAction::parse("M", false, false),
            MenuAction::Invalid("M".to_string())
        );
        assert_eq!(
            MenuAction::parse("3", false, false),
            MenuAction::Invalid("3".to_string())
        );
    }

    #[test]
    fn test_parse_file() {
        assert_eq!(MenuAction::parse("F", true, false), MenuAction::File);
        assert_eq!(MenuAction::parse("4", true, false), MenuAction::File);
        // File is also available to guests
        assert_eq!(MenuAction::parse("F", false, false), MenuAction::File);
    }

    #[test]
    fn test_parse_script() {
        assert_eq!(MenuAction::parse("D", true, false), MenuAction::Script);
        assert_eq!(MenuAction::parse("5", true, false), MenuAction::Script);
        // Script (Door) is also available to guests
        assert_eq!(MenuAction::parse("D", false, false), MenuAction::Script);
    }

    #[test]
    fn test_parse_profile_logged_in() {
        assert_eq!(MenuAction::parse("P", true, false), MenuAction::Profile);
        assert_eq!(MenuAction::parse("6", true, false), MenuAction::Profile);
    }

    #[test]
    fn test_parse_profile_guest() {
        // Guest cannot access profile
        assert_eq!(
            MenuAction::parse("P", false, false),
            MenuAction::Invalid("P".to_string())
        );
    }

    #[test]
    fn test_parse_admin() {
        assert_eq!(MenuAction::parse("A", true, true), MenuAction::Admin);
        assert_eq!(MenuAction::parse("7", true, true), MenuAction::Admin);
    }

    #[test]
    fn test_parse_admin_no_permission() {
        assert_eq!(
            MenuAction::parse("A", true, false),
            MenuAction::Invalid("A".to_string())
        );
    }

    #[test]
    fn test_parse_help() {
        assert_eq!(MenuAction::parse("H", true, false), MenuAction::Help);
        assert_eq!(MenuAction::parse("?", true, false), MenuAction::Help);
    }

    #[test]
    fn test_parse_logout() {
        assert_eq!(MenuAction::parse("L", true, false), MenuAction::Logout);
    }

    #[test]
    fn test_parse_login() {
        assert_eq!(MenuAction::parse("L", false, false), MenuAction::Login);
    }

    #[test]
    fn test_parse_register() {
        assert_eq!(MenuAction::parse("R", false, false), MenuAction::Register);
    }

    #[test]
    fn test_parse_quit() {
        assert_eq!(MenuAction::parse("Q", true, false), MenuAction::Quit);
        assert_eq!(MenuAction::parse("G", true, false), MenuAction::Quit);
    }

    #[test]
    fn test_parse_invalid() {
        assert_eq!(
            MenuAction::parse("X", true, false),
            MenuAction::Invalid("X".to_string())
        );
        assert_eq!(
            MenuAction::parse("", true, false),
            MenuAction::Invalid(String::new())
        );
    }

    #[test]
    fn test_parse_case_insensitive() {
        assert_eq!(
            MenuAction::parse("b", true, false),
            MenuAction::parse("B", true, false)
        );
        assert_eq!(
            MenuAction::parse("c", true, false),
            MenuAction::parse("C", true, false)
        );
    }

    #[test]
    fn test_requires_login() {
        assert!(MenuAction::Mail.requires_login());
        assert!(MenuAction::Profile.requires_login());
        assert!(MenuAction::Logout.requires_login());
        assert!(!MenuAction::Board.requires_login());
        assert!(!MenuAction::Chat.requires_login());
    }

    #[test]
    fn test_requires_admin() {
        assert!(MenuAction::Admin.requires_admin());
        assert!(!MenuAction::Board.requires_admin());
        assert!(!MenuAction::Mail.requires_admin());
    }

    #[test]
    fn test_is_guest_only() {
        assert!(MenuAction::Login.is_guest_only());
        assert!(MenuAction::Register.is_guest_only());
        assert!(!MenuAction::Board.is_guest_only());
        assert!(!MenuAction::Logout.is_guest_only());
    }

    #[test]
    fn test_is_invalid() {
        assert!(MenuAction::Invalid("X".to_string()).is_invalid());
        assert!(!MenuAction::Board.is_invalid());
    }

    #[test]
    fn test_key() {
        assert_eq!(MenuAction::Board.key(), "B");
        assert_eq!(MenuAction::Chat.key(), "C");
        assert_eq!(MenuAction::Mail.key(), "M");
        assert_eq!(MenuAction::Quit.key(), "Q");
    }

    #[test]
    fn test_menu_items_for_member() {
        let items = MenuItems::for_member(false);
        assert!(items.board);
        assert!(items.chat);
        assert!(items.mail);
        assert!(items.file);
        assert!(items.profile);
        assert!(!items.admin);
        assert!(items.help);
        assert!(items.logout);
        assert!(!items.login);
        assert!(!items.register);
        assert!(items.quit);
    }

    #[test]
    fn test_menu_items_for_admin() {
        let items = MenuItems::for_member(true);
        assert!(items.admin);
    }

    #[test]
    fn test_menu_items_for_guest() {
        let items = MenuItems::for_guest();
        assert!(items.board);
        assert!(items.chat);
        assert!(!items.mail);
        assert!(items.file);
        assert!(!items.profile);
        assert!(!items.admin);
        assert!(items.help);
        assert!(!items.logout);
        assert!(items.login);
        assert!(items.register);
        assert!(items.quit);
    }
}
