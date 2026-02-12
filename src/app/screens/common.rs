//! Common utilities for screen handlers.

use std::cell::Cell;
use std::sync::Arc;
use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::time::timeout;

use crate::chat::ChatRoomManager;
use crate::config::Config;
use crate::db::Database;
use crate::error::{HobbsError, Result};
use crate::i18n::I18n;
use crate::rate_limit::RateLimiters;
use crate::server::{
    convert_caret_escape, encode_for_client, process_output_mode, CharacterEncoding, EchoMode,
    InputResult, LineBuffer, SessionManager, TelnetSession,
};
use crate::template::{create_system_context, TemplateContext, TemplateLoader, Value};
use crate::terminal::TerminalProfile;

/// Maximum number of lines in multiline input (to prevent memory exhaustion).
pub const MAX_MULTILINE_LINES: usize = 1000;

/// Shared context for screen handlers.
pub struct ScreenContext {
    /// Database connection.
    pub db: Arc<Database>,
    /// Application configuration.
    pub config: Arc<Config>,
    /// Template loader.
    pub template_loader: Arc<TemplateLoader>,
    /// Terminal profile.
    pub profile: TerminalProfile,
    /// Current i18n instance.
    pub i18n: Arc<I18n>,
    /// Line buffer for input.
    pub line_buffer: LineBuffer,
    /// Chat room manager.
    pub chat_manager: Arc<ChatRoomManager>,
    /// Session manager.
    pub session_manager: Arc<SessionManager>,
    /// Rate limiters for user actions.
    pub rate_limiters: Arc<RateLimiters>,
    /// Lines since last pause (for auto-paging).
    lines_since_pause: Cell<usize>,
    /// Auto-paging enabled flag.
    auto_paging_enabled: bool,
    /// Paging threshold (lines before pause).
    paging_threshold: usize,
}

impl ScreenContext {
    /// Create a new screen context.
    pub fn new(
        db: Arc<Database>,
        config: Arc<Config>,
        template_loader: Arc<TemplateLoader>,
        profile: TerminalProfile,
        i18n: Arc<I18n>,
        encoding: CharacterEncoding,
        chat_manager: Arc<ChatRoomManager>,
        session_manager: Arc<SessionManager>,
        rate_limiters: Arc<RateLimiters>,
    ) -> Self {
        // Calculate paging threshold
        let paging_threshold = if config.terminal.paging_lines > 0 {
            config.terminal.paging_lines
        } else {
            // Default: terminal height - 4 (leaving room for prompt)
            (profile.height.saturating_sub(4).max(5)) as usize
        };

        Self {
            db,
            config: Arc::clone(&config),
            template_loader,
            profile,
            i18n,
            line_buffer: LineBuffer::with_encoding(1024, encoding),
            chat_manager,
            session_manager,
            rate_limiters,
            lines_since_pause: Cell::new(0),
            auto_paging_enabled: config.terminal.auto_paging,
            paging_threshold,
        }
    }

    /// Create a new screen context with user-specific auto-paging setting.
    pub fn new_with_user_paging(
        db: Arc<Database>,
        config: Arc<Config>,
        template_loader: Arc<TemplateLoader>,
        profile: TerminalProfile,
        i18n: Arc<I18n>,
        encoding: CharacterEncoding,
        chat_manager: Arc<ChatRoomManager>,
        session_manager: Arc<SessionManager>,
        rate_limiters: Arc<RateLimiters>,
        auto_paging: bool,
    ) -> Self {
        // Calculate paging threshold
        let paging_threshold = if config.terminal.paging_lines > 0 {
            config.terminal.paging_lines
        } else {
            // Default: terminal height - 4 (leaving room for prompt)
            (profile.height.saturating_sub(4).max(5)) as usize
        };

        Self {
            db,
            config,
            template_loader,
            profile,
            i18n,
            line_buffer: LineBuffer::with_encoding(1024, encoding),
            chat_manager,
            session_manager,
            rate_limiters,
            lines_since_pause: Cell::new(0),
            auto_paging_enabled: auto_paging,
            paging_threshold,
        }
    }

    /// Set auto-paging enabled state.
    pub fn set_auto_paging(&mut self, enabled: bool) {
        self.auto_paging_enabled = enabled;
    }

    /// Check if auto-paging is enabled.
    pub fn auto_paging_enabled(&self) -> bool {
        self.auto_paging_enabled
    }

    /// Create a template context with system variables.
    pub fn create_context(&self) -> TemplateContext {
        let mut context = create_system_context(Arc::clone(&self.i18n));
        context.set_cjk_width(self.profile.cjk_width as usize);
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

    /// Send data to the client.
    /// Converts LF to CRLF for Telnet compatibility.
    /// Processes output according to the session's output mode (strips ANSI for Plain mode).
    /// If auto-paging is enabled, sends line-by-line and pauses when threshold is reached.
    pub async fn send(&self, session: &mut TelnetSession, data: &str) -> Result<()> {
        // Convert LF to CRLF for Telnet (but avoid converting already-CRLF sequences)
        let data = data.replace("\r\n", "\n").replace('\n', "\r\n");

        if self.auto_paging_enabled {
            // Split by CRLF to send line-by-line with paging support
            let segments: Vec<&str> = data.split("\r\n").collect();
            let last_idx = segments.len() - 1;

            for (i, segment) in segments.iter().enumerate() {
                let is_last = i == last_idx;

                if is_last {
                    // Last segment: send without trailing CRLF (might be a prompt)
                    if !segment.is_empty() {
                        let processed = process_output_mode(segment, session.output_mode());
                        let encoded = encode_for_client(&processed, session.encoding());
                        session.stream_mut().write_all(&encoded).await?;
                    }
                } else {
                    // Count display lines BEFORE sending
                    let line_count = self.count_display_lines(segment);
                    let current = self.lines_since_pause.get();

                    // Check if we need to pause BEFORE sending this line
                    if current > 0 && current + line_count > self.paging_threshold {
                        session.stream_mut().flush().await?;
                        self.pause_for_more(session).await?;
                    }

                    // Send line with CRLF
                    let line_with_crlf = format!("{}\r\n", segment);
                    let processed =
                        process_output_mode(&line_with_crlf, session.output_mode());
                    let encoded = encode_for_client(&processed, session.encoding());
                    session.stream_mut().write_all(&encoded).await?;

                    // Update counter after sending
                    let new_current = self.lines_since_pause.get() + line_count;
                    self.lines_since_pause.set(new_current);
                }
            }

            session.stream_mut().flush().await?;
        } else {
            // No paging - send everything at once
            let data = process_output_mode(&data, session.output_mode());
            let encoded = encode_for_client(&data, session.encoding());
            session.stream_mut().write_all(&encoded).await?;
            session.stream_mut().flush().await?;
        }

        Ok(())
    }

    /// Send a line to the client with CRLF.
    ///
    /// If auto-paging is enabled, this method will count lines and
    /// pause for user input when the threshold is reached.
    pub async fn send_line(&self, session: &mut TelnetSession, data: &str) -> Result<()> {
        self.send(session, &format!("{}\r\n", data)).await?;
        Ok(())
    }

    /// Count the number of display lines considering terminal width.
    ///
    /// Takes into account:
    /// - Embedded newlines in the text
    /// - Text wrapping due to terminal width
    /// - Full-width characters (CJK) taking 2 columns
    fn count_display_lines(&self, data: &str) -> usize {
        let width = self.profile.width as usize;
        if width == 0 {
            return 1;
        }

        let mut total_lines = 0;

        // Process each logical line (split by newlines)
        for line in data.split('\n') {
            let display_width = self.profile.display_width(line);
            if display_width == 0 {
                // Empty line still counts as 1 line
                total_lines += 1;
            } else {
                // Calculate how many display lines this text will take
                // (ceiling division: how many rows needed for this text)
                total_lines += (display_width + width - 1) / width;
            }
        }

        // If data doesn't contain newlines, ensure at least 1 line is counted
        if total_lines == 0 {
            total_lines = 1;
        }

        total_lines
    }

    /// Send raw data to the client without paging support.
    /// Used internally by pause_for_more() to avoid recursive async calls.
    async fn send_raw(&self, session: &mut TelnetSession, data: &str) -> Result<()> {
        let data = process_output_mode(data, session.output_mode());
        let encoded = encode_for_client(&data, session.encoding());
        session.stream_mut().write_all(&encoded).await?;
        session.stream_mut().flush().await?;
        Ok(())
    }

    /// Pause and wait for user input (for auto-paging).
    async fn pause_for_more(&self, session: &mut TelnetSession) -> Result<()> {
        // Reset counter BEFORE sending anything to avoid recursive pauses
        self.lines_since_pause.set(0);

        self.send_raw(session, self.i18n.t("common.more")).await?;

        let mut buf = [0u8; 1];
        loop {
            match session.stream_mut().read(&mut buf).await {
                Ok(0) => break,
                Ok(_) => {
                    if buf[0] == b'\r' || buf[0] == b'\n' {
                        break;
                    }
                }
                Err(_) => break,
            }
        }

        // Move to new line
        self.send_raw(session, "\r\n").await?;
        Ok(())
    }

    /// Reset the line counter (call after input operations).
    pub fn reset_line_counter(&self) {
        self.lines_since_pause.set(0);
    }

    /// Word-wrap text to fit the terminal width.
    ///
    /// - ASCII characters are grouped into words (break at spaces only)
    /// - Non-ASCII (full-width) characters are individually wrappable
    /// - Spaces between tokens are preserved where they fit
    pub fn word_wrap(&self, text: &str) -> String {
        let max_width = self.profile.width as usize;
        if max_width == 0 {
            return text.to_string();
        }
        let cjk = self.profile.cjk_width;

        let mut result = Vec::new();
        for line in text.lines() {
            if line.is_empty() {
                result.push(String::new());
                continue;
            }

            let mut current = String::new();
            let mut current_width: usize = 0;
            let mut space_pending = false;
            let mut chars = line.chars().peekable();

            while let Some(&c) = chars.peek() {
                if c.is_whitespace() {
                    // Consume whitespace, mark as pending separator
                    chars.next();
                    while chars.peek().map_or(false, |ch| ch.is_whitespace()) {
                        chars.next();
                    }
                    if !current.is_empty() {
                        space_pending = true;
                    }
                    continue;
                }

                if c.is_ascii() {
                    // Collect consecutive ASCII non-space chars as one word
                    let mut word = String::new();
                    while let Some(&ch) = chars.peek() {
                        if ch.is_whitespace() || !ch.is_ascii() {
                            break;
                        }
                        word.push(ch);
                        chars.next();
                    }
                    let word_width = self.profile.display_width(&word);
                    let space_w = if space_pending { 1 } else { 0 };

                    if current.is_empty() {
                        if word_width > max_width {
                            // ASCII word longer than terminal - char-wrap
                            let (last, last_w) = Self::char_wrap_into(
                                &word,
                                max_width,
                                &self.profile,
                                &mut result,
                            );
                            current = last;
                            current_width = last_w;
                        } else {
                            current = word;
                            current_width = word_width;
                        }
                    } else if current_width + space_w + word_width <= max_width {
                        if space_pending {
                            current.push(' ');
                            current_width += 1;
                        }
                        current.push_str(&word);
                        current_width += word_width;
                    } else {
                        result.push(current);
                        if word_width > max_width {
                            let (last, last_w) = Self::char_wrap_into(
                                &word,
                                max_width,
                                &self.profile,
                                &mut result,
                            );
                            current = last;
                            current_width = last_w;
                        } else {
                            current = word;
                            current_width = word_width;
                        }
                    }
                    space_pending = false;
                } else {
                    // Non-ASCII (full-width) char - individually wrappable
                    chars.next();
                    let char_width = if cjk == 1 { 1 } else { 2 };
                    let space_w = if space_pending { 1 } else { 0 };

                    if !current.is_empty()
                        && current_width + space_w + char_width > max_width
                    {
                        result.push(current);
                        current = String::new();
                        current_width = 0;
                        space_pending = false;
                    }

                    if space_pending && !current.is_empty() {
                        current.push(' ');
                        current_width += 1;
                    }
                    space_pending = false;

                    current.push(c);
                    current_width += char_width;
                }
            }

            if !current.is_empty() {
                result.push(current);
            }
        }
        result.join("\r\n")
    }

    /// Wrap text at character boundaries to fit within max_width.
    /// Full-width lines are pushed to result.
    /// Returns the last partial line and its display width so the caller
    /// can continue appending subsequent words to it.
    fn char_wrap_into(
        text: &str,
        max_width: usize,
        profile: &TerminalProfile,
        result: &mut Vec<String>,
    ) -> (String, usize) {
        let mut current = String::new();
        let mut current_width: usize = 0;
        let cjk = profile.cjk_width;

        for c in text.chars() {
            let char_width = if cjk == 1 || c.is_ascii() { 1 } else { 2 };
            if current_width + char_width > max_width && !current.is_empty() {
                result.push(current);
                current = String::new();
                current_width = 0;
            }
            current.push(c);
            current_width += char_width;
        }
        (current, current_width)
    }

    /// Read a line of input from the client.
    pub async fn read_line(&mut self, session: &mut TelnetSession) -> Result<String> {
        self.line_buffer.clear();
        let mut buf = [0u8; 1];

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
            let read_result = timeout(read_timeout, session.stream_mut().read(&mut buf)).await;

            match read_result {
                Ok(Ok(0)) => {
                    // Connection closed
                    return Ok(String::new());
                }
                Ok(Ok(_)) => {
                    let (result, echo) = self.line_buffer.process_byte(buf[0]);

                    // Handle echo based on mode
                    if !echo.is_empty() {
                        match self.line_buffer.echo_mode() {
                            EchoMode::Normal => {
                                let _ = session.stream_mut().write_all(&echo).await;
                                let _ = session.stream_mut().flush().await;
                            }
                            EchoMode::Password => {
                                // For password mode, echo asterisks for regular chars, but allow backspace
                                if echo.len() == 1 && echo[0] != b'\x08' {
                                    let _ = session.stream_mut().write_all(b"*").await;
                                    let _ = session.stream_mut().flush().await;
                                } else if echo.len() > 1 && echo[0] == b'\x08' {
                                    // Backspace echo
                                    let _ = session.stream_mut().write_all(&echo).await;
                                    let _ = session.stream_mut().flush().await;
                                }
                            }
                            EchoMode::Masked(c) => {
                                if echo.len() == 1 && echo[0] != b'\x08' {
                                    let _ = session.stream_mut().write_all(&[c as u8]).await;
                                    let _ = session.stream_mut().flush().await;
                                } else if echo.len() > 1 && echo[0] == b'\x08' {
                                    let _ = session.stream_mut().write_all(&echo).await;
                                    let _ = session.stream_mut().flush().await;
                                }
                            }
                        }
                    }

                    match result {
                        InputResult::Line(ref line) => {
                            self.reset_line_counter();
                            return Ok(line.clone());
                        }
                        InputResult::Buffering => {
                            // Continue reading
                        }
                        InputResult::Cancel | InputResult::Eof => {
                            self.reset_line_counter();
                            return Ok(String::new());
                        }
                    }
                }
                Ok(Err(e)) => {
                    return Err(e.into());
                }
                Err(_) => {
                    // Timeout elapsed
                    return Err(HobbsError::Io(std::io::Error::new(
                        std::io::ErrorKind::TimedOut,
                        "Read timeout",
                    )));
                }
            }
        }
    }

    /// Read a line with a short timeout (non-blocking for chat).
    ///
    /// Returns:
    /// - `Ok(Some(line))` if a complete line was read
    /// - `Ok(None)` if timeout elapsed with no input
    /// - `Err` on connection error
    pub async fn read_line_nonblocking(
        &mut self,
        session: &mut TelnetSession,
        timeout_ms: u64,
    ) -> Result<Option<String>> {
        let mut buf = [0u8; 1];
        let read_timeout = Duration::from_millis(timeout_ms);

        // Try to read the first byte with timeout
        match timeout(read_timeout, session.stream_mut().read(&mut buf)).await {
            Ok(Ok(0)) => {
                // Connection closed
                return Ok(Some(String::new()));
            }
            Ok(Ok(_)) => {
                // Got a byte, process it and continue reading
                let (result, echo) = self.line_buffer.process_byte(buf[0]);

                // Echo the character
                if !echo.is_empty() {
                    match self.line_buffer.echo_mode() {
                        EchoMode::Normal => {
                            let _ = session.stream_mut().write_all(&echo).await;
                            let _ = session.stream_mut().flush().await;
                        }
                        EchoMode::Password => {
                            if echo.len() == 1 && echo[0] != b'\x08' {
                                let _ = session.stream_mut().write_all(b"*").await;
                                let _ = session.stream_mut().flush().await;
                            } else if echo.len() > 1 && echo[0] == b'\x08' {
                                let _ = session.stream_mut().write_all(&echo).await;
                                let _ = session.stream_mut().flush().await;
                            }
                        }
                        EchoMode::Masked(c) => {
                            if echo.len() == 1 && echo[0] != b'\x08' {
                                let _ = session.stream_mut().write_all(&[c as u8]).await;
                                let _ = session.stream_mut().flush().await;
                            } else if echo.len() > 1 && echo[0] == b'\x08' {
                                let _ = session.stream_mut().write_all(&echo).await;
                                let _ = session.stream_mut().flush().await;
                            }
                        }
                    }
                }

                match result {
                    InputResult::Line(line) => return Ok(Some(line)),
                    InputResult::Buffering => {
                        // Continue reading until we get a complete line
                        return self.finish_line_reading(session).await.map(Some);
                    }
                    InputResult::Cancel | InputResult::Eof => {
                        return Ok(Some(String::new()));
                    }
                }
            }
            Ok(Err(e)) => return Err(e.into()),
            Err(_) => {
                // Timeout - no input available
                return Ok(None);
            }
        }
    }

    /// Continue reading until we get a complete line.
    async fn finish_line_reading(&mut self, session: &mut TelnetSession) -> Result<String> {
        let mut buf = [0u8; 1];

        loop {
            match session.stream_mut().read(&mut buf).await {
                Ok(0) => return Ok(String::new()),
                Ok(_) => {
                    let (result, echo) = self.line_buffer.process_byte(buf[0]);

                    // Echo handling
                    if !echo.is_empty() {
                        match self.line_buffer.echo_mode() {
                            EchoMode::Normal => {
                                let _ = session.stream_mut().write_all(&echo).await;
                                let _ = session.stream_mut().flush().await;
                            }
                            EchoMode::Password => {
                                if echo.len() == 1 && echo[0] != b'\x08' {
                                    let _ = session.stream_mut().write_all(b"*").await;
                                    let _ = session.stream_mut().flush().await;
                                } else if echo.len() > 1 && echo[0] == b'\x08' {
                                    let _ = session.stream_mut().write_all(&echo).await;
                                    let _ = session.stream_mut().flush().await;
                                }
                            }
                            EchoMode::Masked(c) => {
                                if echo.len() == 1 && echo[0] != b'\x08' {
                                    let _ = session.stream_mut().write_all(&[c as u8]).await;
                                    let _ = session.stream_mut().flush().await;
                                } else if echo.len() > 1 && echo[0] == b'\x08' {
                                    let _ = session.stream_mut().write_all(&echo).await;
                                    let _ = session.stream_mut().flush().await;
                                }
                            }
                        }
                    }

                    match result {
                        InputResult::Line(line) => return Ok(line),
                        InputResult::Buffering => continue,
                        InputResult::Cancel | InputResult::Eof => {
                            return Ok(String::new());
                        }
                    }
                }
                Err(e) => return Err(e.into()),
            }
        }
    }

    /// Read a single character.
    pub async fn read_char(&self, session: &mut TelnetSession) -> Result<char> {
        let mut buf = [0u8; 1];

        // Determine timeout based on session state
        let timeout_secs = if session.is_logged_in() {
            self.config.server.idle_timeout_secs
        } else if session.is_guest() {
            self.config.server.guest_timeout_secs
        } else {
            self.config.server.read_timeout_secs
        };
        let read_timeout = Duration::from_secs(timeout_secs);

        loop {
            let read_result = timeout(read_timeout, session.stream_mut().read(&mut buf)).await;

            match read_result {
                Ok(Ok(0)) => return Ok('\0'),
                Ok(Ok(_)) => {
                    let ch = buf[0] as char;
                    if ch.is_ascii_graphic() || ch == '\r' || ch == '\n' {
                        return Ok(ch);
                    }
                }
                Ok(Err(e)) => return Err(e.into()),
                Err(_) => {
                    return Err(HobbsError::Io(std::io::Error::new(
                        std::io::ErrorKind::TimedOut,
                        "Read timeout",
                    )));
                }
            }
        }
    }

    /// Wait for Enter key press.
    pub async fn wait_for_enter(&self, session: &mut TelnetSession) -> Result<()> {
        self.send(session, self.i18n.t("common.press_enter"))
            .await?;
        let mut buf = [0u8; 1];

        // Determine timeout based on session state
        let timeout_secs = if session.is_logged_in() {
            self.config.server.idle_timeout_secs
        } else if session.is_guest() {
            self.config.server.guest_timeout_secs
        } else {
            self.config.server.read_timeout_secs
        };
        let read_timeout = Duration::from_secs(timeout_secs);

        loop {
            let read_result = timeout(read_timeout, session.stream_mut().read(&mut buf)).await;

            match read_result {
                Ok(Ok(0)) => break,
                Ok(Ok(_)) => {
                    if buf[0] == b'\r' || buf[0] == b'\n' {
                        break;
                    }
                }
                Ok(Err(_)) => break,
                Err(_) => {
                    return Err(HobbsError::Io(std::io::Error::new(
                        std::io::ErrorKind::TimedOut,
                        "Read timeout",
                    )));
                }
            }
        }
        self.reset_line_counter();
        Ok(())
    }

    /// Read multiline input.
    ///
    /// Input ends when a line containing only "." is entered.
    /// Returns `None` if the user cancels by entering "/c" or "/cancel".
    ///
    /// # Returns
    ///
    /// - `Ok(Some(text))` - User completed input
    /// - `Ok(None)` - User cancelled input
    pub async fn read_multiline(&mut self, session: &mut TelnetSession) -> Result<Option<String>> {
        let mut lines = Vec::new();

        loop {
            self.send(session, "> ").await?;
            let line = self.read_line(session).await?;
            let trimmed = line.trim();

            // Check for end marker
            if trimmed == "." {
                break;
            }

            // Check for cancel commands
            if trimmed.eq_ignore_ascii_case("/c") || trimmed.eq_ignore_ascii_case("/cancel") {
                self.send_line(session, self.i18n.t("common.input_cancelled"))
                    .await?;
                return Ok(None);
            }

            // Check line count limit
            if lines.len() >= MAX_MULTILINE_LINES {
                self.send_line(
                    session,
                    &self.i18n.t_with(
                        "common.too_many_lines",
                        &[("max", &MAX_MULTILINE_LINES.to_string())],
                    ),
                )
                .await?;
                return Ok(None);
            }

            lines.push(line);
        }

        Ok(Some(lines.join("\n")))
    }

    /// Parse a number from input.
    pub fn parse_number(&self, input: &str) -> Option<i64> {
        input.trim().parse().ok()
    }

    /// Render a template.
    ///
    /// Applies caret escape conversion (`^[` → ESC) so templates can use
    /// `^[[34m` notation for ANSI escape sequences.
    pub fn render_template(&self, name: &str, context: &TemplateContext) -> Result<String> {
        let rendered = self
            .template_loader
            .render(name, self.profile.width, context)
            .map_err(|e| crate::error::HobbsError::from(e))?;
        Ok(convert_caret_escape(&rendered))
    }

    /// Set line buffer echo mode.
    pub fn set_echo_mode(&mut self, mode: EchoMode) {
        self.line_buffer.set_echo_mode(mode);
    }
}

/// Pagination helper.
#[derive(Debug, Clone)]
pub struct Pagination {
    /// Current page (1-indexed).
    pub page: usize,
    /// Items per page.
    pub per_page: usize,
    /// Total items.
    pub total: usize,
}

impl Pagination {
    /// Create a new pagination.
    pub fn new(page: usize, per_page: usize, total: usize) -> Self {
        Self {
            page: page.max(1),
            per_page,
            total,
        }
    }

    /// Get the total number of pages.
    pub fn total_pages(&self) -> usize {
        if self.total == 0 {
            1
        } else {
            (self.total + self.per_page - 1) / self.per_page
        }
    }

    /// Check if there is a next page.
    pub fn has_next(&self) -> bool {
        self.page < self.total_pages()
    }

    /// Check if there is a previous page.
    pub fn has_prev(&self) -> bool {
        self.page > 1
    }

    /// Get the offset for database queries.
    pub fn offset(&self) -> usize {
        (self.page - 1) * self.per_page
    }

    /// Go to the next page.
    pub fn next(&mut self) {
        if self.has_next() {
            self.page += 1;
        }
    }

    /// Go to the previous page.
    pub fn prev(&mut self) {
        if self.has_prev() {
            self.page -= 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pagination_new() {
        let p = Pagination::new(1, 10, 100);
        assert_eq!(p.page, 1);
        assert_eq!(p.per_page, 10);
        assert_eq!(p.total, 100);
    }

    #[test]
    fn test_pagination_total_pages() {
        assert_eq!(Pagination::new(1, 10, 100).total_pages(), 10);
        assert_eq!(Pagination::new(1, 10, 95).total_pages(), 10);
        assert_eq!(Pagination::new(1, 10, 91).total_pages(), 10);
        assert_eq!(Pagination::new(1, 10, 0).total_pages(), 1);
    }

    #[test]
    fn test_pagination_has_next() {
        assert!(Pagination::new(1, 10, 100).has_next());
        assert!(Pagination::new(9, 10, 100).has_next());
        assert!(!Pagination::new(10, 10, 100).has_next());
    }

    #[test]
    fn test_pagination_has_prev() {
        assert!(!Pagination::new(1, 10, 100).has_prev());
        assert!(Pagination::new(2, 10, 100).has_prev());
        assert!(Pagination::new(10, 10, 100).has_prev());
    }

    #[test]
    fn test_pagination_offset() {
        assert_eq!(Pagination::new(1, 10, 100).offset(), 0);
        assert_eq!(Pagination::new(2, 10, 100).offset(), 10);
        assert_eq!(Pagination::new(10, 10, 100).offset(), 90);
    }

    #[test]
    fn test_pagination_next_prev() {
        let mut p = Pagination::new(5, 10, 100);
        p.next();
        assert_eq!(p.page, 6);
        p.prev();
        assert_eq!(p.page, 5);

        // Test bounds
        let mut p = Pagination::new(10, 10, 100);
        p.next();
        assert_eq!(p.page, 10); // No change at last page

        let mut p = Pagination::new(1, 10, 100);
        p.prev();
        assert_eq!(p.page, 1); // No change at first page
    }

    #[test]
    fn test_pagination_zero_page_normalized() {
        let p = Pagination::new(0, 10, 100);
        assert_eq!(p.page, 1); // Should be normalized to 1
    }

    #[test]
    fn test_max_multiline_lines_constant() {
        // Verify the constant is set to a reasonable value
        assert_eq!(MAX_MULTILINE_LINES, 1000);
        // Should be large enough for typical posts but prevent memory exhaustion
        assert!(MAX_MULTILINE_LINES >= 100);
        assert!(MAX_MULTILINE_LINES <= 10000);
    }
}
