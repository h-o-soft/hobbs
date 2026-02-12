//! Session handler module.
//!
//! Provides the main session loop and screen transitions.

use std::sync::Arc;
use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::time::timeout;
use tracing::{error, info, warn};

use super::menu::{MenuAction, MenuItems};
use crate::auth::{verify_password, LimitResult, LoginLimiter, RegistrationRequest};
use crate::chat::ChatRoomManager;
use crate::config::Config;
use crate::datetime::format_datetime_default;
use crate::db::{Database, Role, UserRepository};
use crate::error::{HobbsError, Result};
use crate::i18n::{I18n, I18nManager};
use crate::mail::MailRepository;
use crate::rate_limit::RateLimiters;
use crate::screen::{create_screen_from_profile, Screen};
use crate::server::{
    convert_caret_escape, encode_for_client, initial_negotiation, process_output_mode,
    CharacterEncoding, EchoMode, InputResult, LineBuffer, SessionManager, SessionState,
    TelnetParser, TelnetSession,
};
use crate::template::{create_system_context, TemplateContext, TemplateLoader, Value};
use crate::terminal::TerminalProfile;

/// Session handler for managing a single client session.
pub struct SessionHandler {
    /// Database connection.
    db: Arc<Database>,
    /// Application configuration.
    config: Arc<Config>,
    /// Internationalization manager.
    i18n_manager: Arc<I18nManager>,
    /// Template loader.
    template_loader: Arc<TemplateLoader>,
    /// Session manager.
    session_manager: Arc<SessionManager>,
    /// Chat room manager.
    chat_manager: Arc<ChatRoomManager>,
    /// Rate limiters for user actions.
    rate_limiters: Arc<RateLimiters>,
    /// Terminal profile.
    profile: TerminalProfile,
    /// Screen renderer.
    screen: Box<dyn Screen>,
    /// Current i18n instance.
    i18n: Arc<I18n>,
    /// Line buffer for input.
    line_buffer: LineBuffer,
    /// Telnet parser for filtering IAC commands.
    telnet_parser: TelnetParser,
    /// Pending bytes from previous read (bytes after line terminator).
    pending_bytes: Vec<u8>,
    /// Login limiter.
    login_limiter: LoginLimiter,
}

impl SessionHandler {
    /// Create a new session handler.
    ///
    /// Uses the default terminal profile from config if not specified.
    pub fn new(
        db: Arc<Database>,
        config: Arc<Config>,
        i18n_manager: Arc<I18nManager>,
        template_loader: Arc<TemplateLoader>,
        session_manager: Arc<SessionManager>,
        chat_manager: Arc<ChatRoomManager>,
        rate_limiters: Arc<RateLimiters>,
    ) -> Self {
        // Use default profile from config
        let profile = TerminalProfile::from_name(&config.terminal.default_profile);
        let screen = create_screen_from_profile(&profile);
        let lang = &config.locale.language;
        let i18n = i18n_manager
            .get(lang)
            .cloned()
            .map(Arc::new)
            .unwrap_or_else(|| Arc::new(I18n::empty(lang)));
        let line_buffer = LineBuffer::with_encoding(1024, CharacterEncoding::default());

        Self {
            db,
            config,
            i18n_manager,
            template_loader,
            session_manager,
            chat_manager,
            rate_limiters,
            profile,
            screen,
            i18n,
            line_buffer,
            telnet_parser: TelnetParser::new(),
            pending_bytes: Vec::new(),
            login_limiter: LoginLimiter::new(),
        }
    }

    /// Create a new session handler with a specific terminal profile.
    pub fn with_profile(
        db: Arc<Database>,
        config: Arc<Config>,
        i18n_manager: Arc<I18nManager>,
        template_loader: Arc<TemplateLoader>,
        session_manager: Arc<SessionManager>,
        chat_manager: Arc<ChatRoomManager>,
        rate_limiters: Arc<RateLimiters>,
        profile: TerminalProfile,
    ) -> Self {
        let screen = create_screen_from_profile(&profile);
        let lang = &config.locale.language;
        let i18n = i18n_manager
            .get(lang)
            .cloned()
            .map(Arc::new)
            .unwrap_or_else(|| Arc::new(I18n::empty(lang)));
        let line_buffer = LineBuffer::with_encoding(1024, CharacterEncoding::default());

        Self {
            db,
            config,
            i18n_manager,
            template_loader,
            session_manager,
            chat_manager,
            rate_limiters,
            profile,
            screen,
            i18n,
            line_buffer,
            telnet_parser: TelnetParser::new(),
            pending_bytes: Vec::new(),
            login_limiter: LoginLimiter::new(),
        }
    }

    /// Run the session loop.
    pub async fn run(&mut self, session: &mut TelnetSession) -> Result<()> {
        // Set output mode from profile (encoding is set later via language selection or login)
        session.set_output_mode(self.profile.output_mode);

        // Register session
        self.session_manager.register(session).await;

        // Perform Telnet negotiation
        if let Err(e) = self.negotiate(session).await {
            warn!("Telnet negotiation failed: {}", e);
        }

        // Show welcome screen (ASCII-only, works with any encoding)
        self.show_welcome(session).await?;

        // Main session loop
        loop {
            // Check for force disconnect
            if self.session_manager.should_disconnect(session.id()).await {
                info!("Session {} force disconnected", session.id());
                self.send_line(session, self.i18n.t("session.force_disconnected"))
                    .await?;
                break;
            }

            // Update session info
            self.session_manager.update(session).await;

            match session.state() {
                SessionState::Welcome => {
                    // Prompt for login/guest choice
                    match self.welcome_prompt(session).await? {
                        WelcomeChoice::Login => {
                            // Login: user's saved encoding will be applied after login
                            session.set_state(SessionState::Login);
                        }
                        WelcomeChoice::Register => {
                            // Register: select encoding first, then register
                            self.show_language_selection(session).await?;
                            session.set_state(SessionState::Registration);
                        }
                        WelcomeChoice::Guest => {
                            // Guest: select encoding first, then proceed to menu
                            self.show_language_selection(session).await?;
                            session.set_guest(true);
                            session.set_state(SessionState::MainMenu);
                        }
                        WelcomeChoice::Quit => {
                            break;
                        }
                    }
                }
                SessionState::Login => {
                    if self.handle_login(session).await? {
                        session.set_state(SessionState::MainMenu);
                    } else {
                        session.set_state(SessionState::Welcome);
                    }
                }
                SessionState::Registration => {
                    if self.handle_registration(session).await? {
                        session.set_state(SessionState::MainMenu);
                    } else {
                        session.set_state(SessionState::Welcome);
                    }
                }
                SessionState::MainMenu => match self.handle_main_menu(session).await? {
                    MenuResult::Continue => {}
                    MenuResult::Logout => {
                        session.clear_user();
                        session.set_state(SessionState::Welcome);
                    }
                    MenuResult::Quit => {
                        break;
                    }
                },
                SessionState::Board => {
                    let mut screen_ctx = self.create_screen_context();
                    match super::screens::BoardScreen::run_list(&mut screen_ctx, session).await? {
                        super::screens::ScreenResult::Logout => {
                            session.clear_user();
                            session.set_state(SessionState::Welcome);
                        }
                        super::screens::ScreenResult::Quit => {
                            break;
                        }
                        _ => {
                            session.set_state(SessionState::MainMenu);
                        }
                    }
                }
                SessionState::Chat => {
                    let mut screen_ctx = self.create_screen_context();
                    match super::screens::ChatScreen::run_list(&mut screen_ctx, session).await? {
                        super::screens::ScreenResult::Logout => {
                            session.clear_user();
                            session.set_state(SessionState::Welcome);
                        }
                        super::screens::ScreenResult::Quit => {
                            break;
                        }
                        _ => {
                            session.set_state(SessionState::MainMenu);
                        }
                    }
                }
                SessionState::Mail => {
                    let mut screen_ctx = self.create_screen_context();
                    match super::screens::MailScreen::run_inbox(&mut screen_ctx, session).await? {
                        super::screens::ScreenResult::Logout => {
                            session.clear_user();
                            session.set_state(SessionState::Welcome);
                        }
                        super::screens::ScreenResult::Quit => {
                            break;
                        }
                        _ => {
                            session.set_state(SessionState::MainMenu);
                        }
                    }
                }
                SessionState::Files => {
                    let mut screen_ctx = self.create_screen_context();
                    match super::screens::FileScreen::run_browser(&mut screen_ctx, session, None)
                        .await?
                    {
                        super::screens::ScreenResult::Logout => {
                            session.clear_user();
                            session.set_state(SessionState::Welcome);
                        }
                        super::screens::ScreenResult::Quit => {
                            break;
                        }
                        _ => {
                            session.set_state(SessionState::MainMenu);
                        }
                    }
                }
                SessionState::Script => {
                    let mut screen_ctx = self.create_screen_context();
                    match super::screens::ScriptScreen::run(&mut screen_ctx, session).await? {
                        super::screens::ScreenResult::Logout => {
                            session.clear_user();
                            session.set_state(SessionState::Welcome);
                        }
                        super::screens::ScreenResult::Quit => {
                            break;
                        }
                        _ => {
                            session.set_state(SessionState::MainMenu);
                        }
                    }
                }
                SessionState::Admin => {
                    let mut screen_ctx = self.create_screen_context();
                    match super::screens::AdminScreen::run(&mut screen_ctx, session).await? {
                        super::screens::ScreenResult::Logout => {
                            session.clear_user();
                            session.set_state(SessionState::Welcome);
                        }
                        super::screens::ScreenResult::Quit => {
                            break;
                        }
                        _ => {
                            session.set_state(SessionState::MainMenu);
                        }
                    }
                }
                SessionState::News => {
                    let mut screen_ctx = self.create_screen_context();
                    match super::screens::RssScreen::run(&mut screen_ctx, session).await? {
                        super::screens::ScreenResult::Logout => {
                            session.clear_user();
                            session.set_state(SessionState::Welcome);
                        }
                        super::screens::ScreenResult::Quit => {
                            break;
                        }
                        _ => {
                            session.set_state(SessionState::MainMenu);
                        }
                    }
                }
                SessionState::Closing => {
                    break;
                }
            }
        }

        // Show goodbye message
        self.send_line(session, self.i18n.t("session.goodbye"))
            .await?;

        // Unregister session
        self.session_manager.unregister(session.id()).await;

        Ok(())
    }

    /// Perform Telnet negotiation.
    async fn negotiate(&self, session: &mut TelnetSession) -> Result<()> {
        let negotiation_bytes = initial_negotiation();
        session.stream_mut().write_all(&negotiation_bytes).await?;
        session.stream_mut().flush().await?;
        Ok(())
    }

    /// Show language/encoding selection screen.
    ///
    /// This screen is shown in ASCII-only to work regardless of the current
    /// encoding setting. After selection, the encoding and language are applied.
    async fn show_language_selection(&mut self, session: &mut TelnetSession) -> Result<()> {
        // Display ASCII-only selection screen
        let selection_screen = r#"
=======================================
Select language / Gengo sentaku:
=======================================

[E] English (UTF-8)
[J] Nihongo (ShiftJIS)
[U] Nihongo (UTF-8)

"#;
        self.send(session, selection_screen).await?;
        self.send(session, "> ").await?;

        // Read user input
        let input = self.read_line(session).await?;
        let input = input.trim().to_uppercase();

        // Apply selection
        match input.as_str() {
            "E" | "1" => {
                // English (UTF-8)
                self.set_language("en");
                session.set_encoding(CharacterEncoding::Utf8);
                self.line_buffer.set_encoding(CharacterEncoding::Utf8);
            }
            "J" | "2" => {
                // Japanese (ShiftJIS)
                self.set_language("ja");
                session.set_encoding(CharacterEncoding::ShiftJIS);
                self.line_buffer.set_encoding(CharacterEncoding::ShiftJIS);
            }
            "U" | "3" => {
                // Japanese (UTF-8)
                self.set_language("ja");
                session.set_encoding(CharacterEncoding::Utf8);
                self.line_buffer.set_encoding(CharacterEncoding::Utf8);
            }
            _ => {
                // Default to English (UTF-8) for invalid input
                self.set_language("en");
                session.set_encoding(CharacterEncoding::Utf8);
                self.line_buffer.set_encoding(CharacterEncoding::Utf8);
            }
        }

        Ok(())
    }

    /// Set the current language for i18n.
    fn set_language(&mut self, lang: &str) {
        self.i18n = self
            .i18n_manager
            .get(lang)
            .cloned()
            .map(Arc::new)
            .unwrap_or_else(|| Arc::new(I18n::empty(lang)));
    }

    /// Set the terminal profile.
    ///
    /// Updates the profile and recreates the screen renderer.
    fn set_terminal_profile(&mut self, profile_name: &str) {
        let new_profile = TerminalProfile::from_name(profile_name);
        if new_profile != self.profile {
            self.profile = new_profile.clone();
            self.screen = create_screen_from_profile(&new_profile);
        }
    }

    /// Show the welcome screen.
    async fn show_welcome(&self, session: &mut TelnetSession) -> Result<()> {
        let context = self.create_context();
        let content = self
            .template_loader
            .render("welcome", self.profile.width, &context)?;
        let content = convert_caret_escape(&content);
        self.send(session, &content).await
    }

    /// Handle welcome prompt.
    ///
    /// Loops until a valid choice (L/R/G/Q) is explicitly selected.
    async fn welcome_prompt(&mut self, session: &mut TelnetSession) -> Result<WelcomeChoice> {
        loop {
            self.send(session, self.i18n.t("welcome.prompt")).await?;

            let input = self.read_line(session).await?;
            let input = input.trim().to_uppercase();

            match input.as_str() {
                "L" | "1" => return Ok(WelcomeChoice::Login),
                "R" | "2" => return Ok(WelcomeChoice::Register),
                "G" | "3" => return Ok(WelcomeChoice::Guest),
                "Q" | "4" => return Ok(WelcomeChoice::Quit),
                _ => {
                    self.send_line(session, self.i18n.t("welcome.invalid_choice"))
                        .await?;
                    self.send_line(session, "").await?;
                }
            }
        }
    }

    /// Handle login.
    async fn handle_login(&mut self, session: &mut TelnetSession) -> Result<bool> {
        self.send_line(session, self.i18n.t("login.title")).await?;

        // Get username
        self.send(session, &format!("{}: ", self.i18n.t("login.username")))
            .await?;
        let username = self.read_line(session).await?;
        let username = username.trim();

        if username.is_empty() {
            return Ok(false);
        }

        // Check login limiter
        let peer_addr = session.peer_addr().ip().to_string();
        match self.login_limiter.check(&peer_addr) {
            LimitResult::Locked(_) => {
                self.send_line(session, self.i18n.t("login.locked_out"))
                    .await?;
                return Ok(false);
            }
            LimitResult::Allowed => {}
        }

        // Get password
        self.send(session, &format!("{}: ", self.i18n.t("login.password")))
            .await?;
        self.line_buffer.set_echo_mode(EchoMode::Password);
        let password = self.read_line(session).await?;
        self.line_buffer.set_echo_mode(EchoMode::Normal);
        self.send_line(session, "").await?; // New line after password

        // Verify credentials
        let user_repo = UserRepository::new(self.db.pool());

        match user_repo.get_by_username(username).await {
            Ok(Some(user)) => {
                if verify_password(&password, &user.password).is_ok() {
                    // Check if user is active
                    if !user.is_active {
                        warn!(
                            username = %username,
                            user_id = user.id,
                            ip = %peer_addr,
                            "Telnet login failed: account disabled"
                        );
                        self.send_line(session, self.i18n.t("login.account_disabled"))
                            .await?;
                        return Ok(false);
                    }

                    // Login successful
                    self.login_limiter.clear(&peer_addr);
                    session.set_user(user.id, user.username.clone());

                    // Apply user's encoding preference
                    session.set_encoding(user.encoding);
                    self.line_buffer.set_encoding(user.encoding);

                    // Save user settings for later application
                    let user_id = user.id;
                    let user_role = user.role;
                    let user_language = user.language.clone();
                    let user_terminal = user.terminal.clone();
                    let user_name = user.username.clone();

                    // Save previous last login before updating
                    let previous_login = user.last_login.clone();

                    // Update last login
                    if let Err(e) = user_repo.update_last_login(user.id).await {
                        warn!("Failed to update last login: {}", e);
                    }

                    // Now apply language and terminal preferences (after user_repo borrow ends)
                    self.set_language(&user_language);
                    self.set_terminal_profile(&user_terminal);

                    // Show login success message
                    self.send_line(
                        session,
                        &self
                            .i18n
                            .t_with("login.success", &[("username", &user_name)]),
                    )
                    .await?;

                    // Show previous login time if available
                    if let Some(prev) = previous_login {
                        let formatted =
                            format_datetime_default(&prev, &self.config.server.timezone);
                        self.send_line(
                            session,
                            &format!("{}: {}", self.i18n.t("profile.last_login_short"), formatted),
                        )
                        .await?;
                    }

                    info!(
                        username = %user_name,
                        user_id = user_id,
                        role = ?user_role,
                        ip = %peer_addr,
                        "Telnet login successful"
                    );

                    Ok(true)
                } else {
                    warn!(
                        username = %username,
                        user_id = user.id,
                        ip = %peer_addr,
                        "Telnet login failed: invalid password"
                    );
                    self.login_limiter.record_failure(&peer_addr);
                    self.send_line(session, self.i18n.t("login.invalid_credentials"))
                        .await?;
                    Ok(false)
                }
            }
            Ok(None) => {
                warn!(
                    username = %username,
                    ip = %peer_addr,
                    "Telnet login failed: user not found"
                );
                self.login_limiter.record_failure(&peer_addr);
                self.send_line(session, self.i18n.t("login.invalid_credentials"))
                    .await?;
                Ok(false)
            }
            Err(e) => {
                error!("Database error during login: {}", e);
                self.send_line(session, self.i18n.t("error.database"))
                    .await?;
                Ok(false)
            }
        }
    }

    /// Handle registration.
    async fn handle_registration(&mut self, session: &mut TelnetSession) -> Result<bool> {
        self.send_line(session, self.i18n.t("register.title"))
            .await?;

        // Show password warning for Telnet connections
        self.send_line(session, "").await?;
        self.send_line(session, self.i18n.t("register.password_warning"))
            .await?;
        self.send_line(session, "").await?;

        // Get username
        self.send(session, &format!("{}: ", self.i18n.t("register.username")))
            .await?;
        let username = self.read_line(session).await?;
        let username = username.trim().to_string();

        if username.is_empty() {
            return Ok(false);
        }

        // Check if username exists (scope to release borrow before read_line)
        {
            let user_repo = UserRepository::new(self.db.pool());
            if user_repo.get_by_username(&username).await?.is_some() {
                self.send_line(session, self.i18n.t("register.username_taken"))
                    .await?;
                return Ok(false);
            }
        }

        // Get password
        self.send(session, &format!("{}: ", self.i18n.t("register.password")))
            .await?;
        self.line_buffer.set_echo_mode(EchoMode::Password);
        let password = self.read_line(session).await?;
        self.line_buffer.set_echo_mode(EchoMode::Normal);
        self.send_line(session, "").await?;

        // Confirm password
        self.send(
            session,
            &format!("{}: ", self.i18n.t("register.confirm_password")),
        )
        .await?;
        self.line_buffer.set_echo_mode(EchoMode::Password);
        let confirm = self.read_line(session).await?;
        self.line_buffer.set_echo_mode(EchoMode::Normal);
        self.send_line(session, "").await?;

        if password != confirm {
            self.send_line(session, self.i18n.t("register.password_mismatch"))
                .await?;
            return Ok(false);
        }

        // Validate password length
        if password.len() < 8 {
            self.send_line(session, self.i18n.t("register.password_too_short"))
                .await?;
            return Ok(false);
        }

        // Get nickname
        self.send(session, &format!("{}: ", self.i18n.t("register.nickname")))
            .await?;
        let nickname = self.read_line(session).await?;
        let nickname = nickname.trim().to_string();

        let nickname = if nickname.is_empty() {
            username.clone()
        } else {
            nickname
        };

        // Create user (new scope for UserRepository)
        // Save the encoding and language from the language selection screen
        let request = RegistrationRequest::new(username.clone(), password, nickname)
            .with_encoding(session.encoding())
            .with_language(self.i18n.locale());
        let user_repo = UserRepository::new(self.db.pool());

        // Check if this is the first user - make them SysOp
        let is_first_user = user_repo.count().await.unwrap_or(0) == 0;

        let result = if is_first_user {
            crate::auth::register_with_role(&user_repo, request, Role::SysOp).await
        } else {
            crate::auth::register(&user_repo, request).await
        };

        match result {
            Ok(user) => {
                session.set_user(user.id, user.username.clone());
                self.send_line(
                    session,
                    &self
                        .i18n
                        .t_with("register.success", &[("username", &user.username)]),
                )
                .await?;
                Ok(true)
            }
            Err(e) => {
                error!("Registration error: {}", e);
                self.send_line(session, self.i18n.t("register.failed"))
                    .await?;
                Ok(false)
            }
        }
    }

    /// Handle main menu.
    async fn handle_main_menu(&mut self, session: &mut TelnetSession) -> Result<MenuResult> {
        // Show main menu
        self.show_main_menu(session).await?;

        // Get user input
        self.send(session, "> ").await?;
        let input = self.read_line(session).await?;

        // Parse action
        let is_logged_in = session.is_logged_in();
        let is_admin = self.is_admin(session).await;
        let action = MenuAction::parse(&input, is_logged_in, is_admin);

        // Handle action
        match action {
            MenuAction::Board => {
                session.set_state(SessionState::Board);
            }
            MenuAction::Chat => {
                session.set_state(SessionState::Chat);
            }
            MenuAction::Mail => {
                if is_logged_in {
                    session.set_state(SessionState::Mail);
                } else {
                    self.send_line(session, self.i18n.t("menu.login_required"))
                        .await?;
                }
            }
            MenuAction::File => {
                session.set_state(SessionState::Files);
            }
            MenuAction::Script => {
                session.set_state(SessionState::Script);
            }
            MenuAction::News => {
                session.set_state(SessionState::News);
            }
            MenuAction::Profile => {
                if is_logged_in {
                    let mut screen_ctx = self.create_screen_context();
                    match super::screens::ProfileScreen::run(&mut screen_ctx, session).await? {
                        super::screens::ScreenResult::Logout => {
                            return Ok(MenuResult::Logout);
                        }
                        super::screens::ScreenResult::Quit => {
                            return Ok(MenuResult::Quit);
                        }
                        super::screens::ScreenResult::SettingsChanged {
                            language,
                            encoding,
                            terminal_profile,
                        } => {
                            // Apply new settings to session
                            session.set_encoding(encoding);
                            self.line_buffer.set_encoding(encoding);
                            self.set_language(&language);
                            if let Some(profile) = terminal_profile {
                                self.set_terminal_profile(&profile);
                            }
                        }
                        _ => {}
                    }
                } else {
                    self.send_line(session, self.i18n.t("menu.login_required"))
                        .await?;
                }
            }
            MenuAction::MemberList => {
                let mut screen_ctx = self.create_screen_context();
                match super::screens::MemberScreen::run(&mut screen_ctx, session).await? {
                    super::screens::ScreenResult::Logout => {
                        return Ok(MenuResult::Logout);
                    }
                    super::screens::ScreenResult::Quit => {
                        return Ok(MenuResult::Quit);
                    }
                    _ => {}
                }
            }
            MenuAction::Admin => {
                if is_admin {
                    session.set_state(SessionState::Admin);
                } else {
                    self.send_line(session, self.i18n.t("menu.admin_required"))
                        .await?;
                }
            }
            MenuAction::Help => {
                let mut screen_ctx = self.create_screen_context();
                let _ = super::screens::HelpScreen::run(&mut screen_ctx, session).await;
            }
            MenuAction::Logout => {
                return Ok(MenuResult::Logout);
            }
            MenuAction::Login => {
                session.set_state(SessionState::Login);
            }
            MenuAction::Register => {
                session.set_state(SessionState::Registration);
            }
            MenuAction::Quit => {
                return Ok(MenuResult::Quit);
            }
            MenuAction::Invalid(s) => {
                if !s.is_empty() {
                    self.send_line(
                        session,
                        &self.i18n.t_with("menu.invalid_selection", &[("input", &s)]),
                    )
                    .await?;
                }
            }
        }

        Ok(MenuResult::Continue)
    }

    /// Show the main menu.
    async fn show_main_menu(&self, session: &mut TelnetSession) -> Result<()> {
        let is_logged_in = session.is_logged_in();
        let is_admin = self.is_admin(session).await;

        let mut context = self.create_context();

        // Set user info
        if let Some(user_id) = session.user_id() {
            let user_repo = UserRepository::new(self.db.pool());
            if let Ok(Some(user)) = user_repo.get_by_id(user_id).await {
                context.set("user.name", Value::string(user.username.clone()));
                context.set("user.nickname", Value::string(user.nickname.clone()));
                context.set("user.logged_in", Value::bool(true));
                context.set("user.is_admin", Value::bool(is_admin));

                // Set role name
                let role_name = match user.role {
                    Role::Guest => self.i18n.t("role.guest"),
                    Role::Member => self.i18n.t("role.member"),
                    Role::SubOp => self.i18n.t("role.subop"),
                    Role::SysOp => self.i18n.t("role.sysop"),
                };
                context.set("user.role_name", Value::string(role_name.to_string()));

                // Set unread mail count
                let mail_repo = MailRepository::new(self.db.pool());
                let unread_count = mail_repo.count_unread(user_id).await.unwrap_or(0);
                context.set("user.unread_mail", Value::number(unread_count));
            } else {
                // Fallback if user not found
                context.set(
                    "user.name",
                    Value::string(self.i18n.t("user.guest").to_string()),
                );
                context.set(
                    "user.nickname",
                    Value::string(self.i18n.t("user.guest").to_string()),
                );
                context.set("user.logged_in", Value::bool(false));
                context.set("user.is_admin", Value::bool(false));
                context.set(
                    "user.role_name",
                    Value::string(self.i18n.t("role.guest").to_string()),
                );
                context.set("user.unread_mail", Value::number(0));
            }
        } else {
            // Guest user
            context.set(
                "user.name",
                Value::string(self.i18n.t("user.guest").to_string()),
            );
            context.set(
                "user.nickname",
                Value::string(self.i18n.t("user.guest").to_string()),
            );
            context.set("user.logged_in", Value::bool(false));
            context.set("user.is_admin", Value::bool(false));
            context.set(
                "user.role_name",
                Value::string(self.i18n.t("role.guest").to_string()),
            );
            context.set("user.unread_mail", Value::number(0));
        }

        // Set chat online count
        let online_count = self.chat_manager.total_participants().await;
        context.set("chat.online_count", Value::number(online_count as i64));

        // Set menu availability
        let menu_items = if is_logged_in {
            MenuItems::for_member(is_admin)
        } else {
            MenuItems::for_guest()
        };

        context.set("menu.board", Value::bool(menu_items.board));
        context.set("menu.chat", Value::bool(menu_items.chat));
        context.set("menu.mail", Value::bool(menu_items.mail));
        context.set("menu.file", Value::bool(menu_items.file));
        context.set("menu.news", Value::bool(menu_items.news));
        context.set("menu.profile", Value::bool(menu_items.profile));
        context.set("menu.member_list", Value::bool(menu_items.member_list));
        context.set("menu.admin", Value::bool(menu_items.admin));
        context.set("menu.help", Value::bool(menu_items.help));
        context.set("menu.logout", Value::bool(menu_items.logout));
        context.set("menu.login", Value::bool(menu_items.login));
        context.set("menu.register", Value::bool(menu_items.register));

        let content = self
            .template_loader
            .render("main_menu", self.profile.width, &context)?;
        let content = convert_caret_escape(&content);
        self.send(session, &content).await
    }

    /// Show help screen.
    async fn show_help(&self, session: &mut TelnetSession) -> Result<()> {
        let context = self.create_context();
        let content = self
            .template_loader
            .render("help", self.profile.width, &context)?;
        let content = convert_caret_escape(&content);
        self.send(session, &content).await?;

        // Wait for key press
        self.send(session, self.i18n.t("common.press_enter"))
            .await?;
        let mut buf = [0u8; 1];
        let _ = session.stream_mut().read(&mut buf).await;

        Ok(())
    }

    /// Check if the user is an admin.
    async fn is_admin(&self, session: &TelnetSession) -> bool {
        if let Some(user_id) = session.user_id() {
            let user_repo = UserRepository::new(self.db.pool());
            if let Ok(Some(user)) = user_repo.get_by_id(user_id).await {
                return user.role >= Role::SubOp;
            }
        }
        false
    }

    /// Create a template context.
    fn create_context(&self) -> TemplateContext {
        let mut context = create_system_context(Arc::clone(&self.i18n));
        context.set("bbs.name", Value::string(self.config.bbs.name.clone()));
        context.set(
            "bbs.description",
            Value::string(self.config.bbs.description.clone()),
        );
        context.set(
            "bbs.sysop",
            Value::string(self.config.bbs.sysop_name.clone()),
        );
        context
    }

    /// Create a screen context for screen handlers.
    fn create_screen_context(&self) -> super::screens::ScreenContext {
        super::screens::ScreenContext::new(
            Arc::clone(&self.db),
            Arc::clone(&self.config),
            Arc::clone(&self.template_loader),
            self.profile.clone(),
            Arc::clone(&self.i18n),
            self.line_buffer.encoding(),
            Arc::clone(&self.chat_manager),
            Arc::clone(&self.session_manager),
            Arc::clone(&self.rate_limiters),
        )
    }

    /// Send data to the client.
    /// Converts LF to CRLF for Telnet compatibility.
    /// Processes output according to the session's output mode (strips ANSI for Plain mode).
    async fn send(&self, session: &mut TelnetSession, data: &str) -> Result<()> {
        // Convert LF to CRLF for Telnet (but avoid converting already-CRLF sequences)
        let data = data.replace("\r\n", "\n").replace('\n', "\r\n");
        // Process output according to session's output mode
        let data = process_output_mode(&data, session.output_mode());
        let encoded = encode_for_client(&data, session.encoding());
        session.stream_mut().write_all(&encoded).await?;
        session.stream_mut().flush().await?;
        Ok(())
    }

    /// Send a line (with CRLF) to the client.
    async fn send_line(&self, session: &mut TelnetSession, data: &str) -> Result<()> {
        self.send(session, data).await?;
        self.send(session, "\r\n").await
    }

    /// Read a line from the client.
    ///
    /// Filters out Telnet IAC commands from the input stream.
    /// Each read operation has a timeout to prevent Slowloris-type DoS attacks.
    /// Timeout duration varies based on session state:
    /// - Unauthenticated: read_timeout_secs (default 30s)
    /// - Guest: guest_timeout_secs (default 120s)
    /// - Logged in: idle_timeout_secs (default 300s)
    async fn read_line(&mut self, session: &mut TelnetSession) -> Result<String> {
        self.line_buffer.clear();

        // Process any pending bytes from previous read first
        if !self.pending_bytes.is_empty() {
            let pending = std::mem::take(&mut self.pending_bytes);
            if let Some(result) = self.process_input_bytes(session, &pending).await? {
                return Ok(result);
            }
        }

        let mut buf = [0u8; 64];

        // Determine timeout based on session state
        let timeout_secs = if session.is_logged_in() {
            // Logged-in users get the full idle timeout
            self.config.server.idle_timeout_secs
        } else if session.is_guest() {
            // Guest users get a medium timeout
            self.config.server.guest_timeout_secs
        } else {
            // Unauthenticated connections get a short timeout (DoS protection)
            self.config.server.read_timeout_secs
        };
        let read_timeout = Duration::from_secs(timeout_secs);

        loop {
            // Apply timeout to each read operation
            let read_result = timeout(read_timeout, session.stream_mut().read(&mut buf)).await;

            match read_result {
                Ok(Ok(0)) => {
                    return Err(HobbsError::Io(std::io::Error::new(
                        std::io::ErrorKind::UnexpectedEof,
                        "Connection closed",
                    )));
                }
                Ok(Ok(n)) => {
                    // Filter out IAC commands
                    let (data, _commands) = self.telnet_parser.parse(&buf[..n]);

                    if let Some(result) = self.process_input_bytes(session, &data).await? {
                        return Ok(result);
                    }
                }
                Ok(Err(e)) => {
                    return Err(HobbsError::Io(e));
                }
                Err(_) => {
                    // Timeout elapsed - no data received within the timeout period
                    return Err(HobbsError::Io(std::io::Error::new(
                        std::io::ErrorKind::TimedOut,
                        "Read timeout",
                    )));
                }
            }
        }
    }

    /// Process input bytes and return the line if complete.
    ///
    /// Saves any bytes after the line terminator to pending_bytes for the next read.
    async fn process_input_bytes(
        &mut self,
        session: &mut TelnetSession,
        data: &[u8],
    ) -> Result<Option<String>> {
        for (i, &byte) in data.iter().enumerate() {
            let (result, echo) = self.line_buffer.process_byte(byte);

            // Echo back
            if !echo.is_empty() {
                session.stream_mut().write_all(&echo).await?;
                session.stream_mut().flush().await?;
            }

            match result {
                InputResult::Line(line) => {
                    session.touch();
                    // Save remaining bytes for next read
                    // Skip the next byte if it's LF after CR (or CR after LF)
                    let remaining_start = if i + 1 < data.len() {
                        let next_byte = data[i + 1];
                        if (byte == 0x0D && next_byte == 0x0A)
                            || (byte == 0x0A && next_byte == 0x0D)
                        {
                            i + 2
                        } else {
                            i + 1
                        }
                    } else {
                        i + 1
                    };
                    if remaining_start < data.len() {
                        self.pending_bytes
                            .extend_from_slice(&data[remaining_start..]);
                    }
                    return Ok(Some(line));
                }
                InputResult::Cancel => {
                    return Ok(Some(String::new()));
                }
                InputResult::Eof => {
                    return Err(HobbsError::Io(std::io::Error::new(
                        std::io::ErrorKind::UnexpectedEof,
                        "EOF received",
                    )));
                }
                InputResult::Buffering => {}
            }
        }
        Ok(None)
    }
}

/// Welcome screen choice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WelcomeChoice {
    Login,
    Register,
    Guest,
    Quit,
}

/// Menu handling result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MenuResult {
    Continue,
    Logout,
    Quit,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rate_limit::RateLimiters;

    // Integration tests would require a full server setup
    // Unit tests for helper functions can be added here

    #[test]
    fn test_welcome_choice_variants() {
        assert_ne!(WelcomeChoice::Login, WelcomeChoice::Register);
        assert_ne!(WelcomeChoice::Guest, WelcomeChoice::Quit);
    }

    #[test]
    fn test_menu_result_variants() {
        assert_ne!(MenuResult::Continue, MenuResult::Logout);
        assert_ne!(MenuResult::Logout, MenuResult::Quit);
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_set_language_updates_i18n() {
        use crate::db::Database;
        use crate::template::TemplateLoader;
        use tempfile::TempDir;

        // Setup
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let db = Database::open(&db_path).await.unwrap();
        let db = Arc::new(db);

        let config = Arc::new(Config::default());

        // Create I18nManager with test locales
        let mut i18n_manager = I18nManager::new();
        let ja = I18n::from_str(
            "ja",
            r#"[menu]
main = "メインメニュー""#,
        )
        .unwrap();
        let en = I18n::from_str(
            "en",
            r#"[menu]
main = "Main Menu""#,
        )
        .unwrap();
        i18n_manager.add_locale(ja);
        i18n_manager.add_locale(en);
        let i18n_manager = Arc::new(i18n_manager);

        // Create minimal template loader
        let templates_dir = temp_dir.path().join("templates");
        std::fs::create_dir_all(&templates_dir).unwrap();
        let template_loader = Arc::new(TemplateLoader::new(&templates_dir));

        let session_manager = Arc::new(SessionManager::new(300));
        let chat_manager = Arc::new(ChatRoomManager::new());
        let rate_limiters = Arc::new(RateLimiters::new());
        let profile = TerminalProfile::default();

        // Create handler
        let mut handler = SessionHandler::with_profile(
            db,
            config,
            i18n_manager.clone(),
            template_loader,
            session_manager,
            chat_manager,
            rate_limiters,
            profile,
        );

        // Test: Initial language is from config (defaults to "ja")
        assert_eq!(handler.i18n.locale(), "ja");
        assert_eq!(handler.i18n.t("menu.main"), "メインメニュー");

        // Test: Set language to English
        handler.set_language("en");
        assert_eq!(handler.i18n.locale(), "en");
        assert_eq!(handler.i18n.t("menu.main"), "Main Menu");

        // Test: Set language back to Japanese
        handler.set_language("ja");
        assert_eq!(handler.i18n.locale(), "ja");
        assert_eq!(handler.i18n.t("menu.main"), "メインメニュー");

        // Test: Set to non-existent language falls back to empty I18n
        handler.set_language("fr");
        assert_eq!(handler.i18n.locale(), "fr");
        assert_eq!(handler.i18n.t("menu.main"), "menu.main"); // Fallback to key
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_set_language_affects_create_context() {
        use crate::db::Database;
        use crate::template::TemplateLoader;
        use tempfile::TempDir;

        // Setup
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let db = Database::open(&db_path).await.unwrap();
        let db = Arc::new(db);

        let config = Arc::new(Config::default());

        // Create I18nManager with test locales
        let mut i18n_manager = I18nManager::new();
        let ja = I18n::from_str(
            "ja",
            r#"[test]
value = "日本語の値""#,
        )
        .unwrap();
        let en = I18n::from_str(
            "en",
            r#"[test]
value = "English value""#,
        )
        .unwrap();
        i18n_manager.add_locale(ja);
        i18n_manager.add_locale(en);
        let i18n_manager = Arc::new(i18n_manager);

        // Create minimal template loader
        let templates_dir = temp_dir.path().join("templates");
        std::fs::create_dir_all(&templates_dir).unwrap();
        let template_loader = Arc::new(TemplateLoader::new(&templates_dir));

        let session_manager = Arc::new(SessionManager::new(300));
        let chat_manager = Arc::new(ChatRoomManager::new());
        let rate_limiters = Arc::new(RateLimiters::new());
        let profile = TerminalProfile::default();

        // Create handler
        let mut handler = SessionHandler::with_profile(
            db,
            config,
            i18n_manager,
            template_loader,
            session_manager,
            chat_manager,
            rate_limiters,
            profile,
        );

        // Set to Japanese
        handler.set_language("ja");
        let _context = handler.create_context();
        // The context contains i18n, verify via translation in the Arc<I18n>
        assert_eq!(handler.i18n.locale(), "ja");

        // Set to English
        handler.set_language("en");
        let _context = handler.create_context();
        assert_eq!(handler.i18n.locale(), "en");
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_set_language_affects_screen_context() {
        use crate::db::Database;
        use crate::template::TemplateLoader;
        use tempfile::TempDir;

        // Setup
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let db = Database::open(&db_path).await.unwrap();
        let db = Arc::new(db);

        let config = Arc::new(Config::default());

        // Create I18nManager with test locales
        let mut i18n_manager = I18nManager::new();
        let ja = I18n::from_str(
            "ja",
            r#"[screen]
title = "タイトル""#,
        )
        .unwrap();
        let en = I18n::from_str(
            "en",
            r#"[screen]
title = "Title""#,
        )
        .unwrap();
        i18n_manager.add_locale(ja);
        i18n_manager.add_locale(en);
        let i18n_manager = Arc::new(i18n_manager);

        // Create minimal template loader
        let templates_dir = temp_dir.path().join("templates");
        std::fs::create_dir_all(&templates_dir).unwrap();
        let template_loader = Arc::new(TemplateLoader::new(&templates_dir));

        let session_manager = Arc::new(SessionManager::new(300));
        let chat_manager = Arc::new(ChatRoomManager::new());
        let rate_limiters = Arc::new(RateLimiters::new());
        let profile = TerminalProfile::default();

        // Create handler
        let mut handler = SessionHandler::with_profile(
            db,
            config,
            i18n_manager,
            template_loader,
            session_manager,
            chat_manager,
            rate_limiters,
            profile,
        );

        // Set to Japanese and create screen context
        handler.set_language("ja");
        let screen_ctx = handler.create_screen_context();
        assert_eq!(screen_ctx.i18n.locale(), "ja");
        assert_eq!(screen_ctx.i18n.t("screen.title"), "タイトル");

        // Set to English and create screen context
        handler.set_language("en");
        let screen_ctx = handler.create_screen_context();
        assert_eq!(screen_ctx.i18n.locale(), "en");
        assert_eq!(screen_ctx.i18n.t("screen.title"), "Title");
    }
}
