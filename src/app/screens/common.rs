//! Common utilities for screen handlers.

use std::sync::Arc;

use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::chat::ChatRoomManager;
use crate::config::Config;
use crate::db::Database;
use crate::error::Result;
use crate::i18n::I18n;
use crate::server::{
    encode_for_client, CharacterEncoding, EchoMode, InputResult, LineBuffer, TelnetSession,
};
use crate::template::{create_system_context, TemplateContext, TemplateLoader, Value};
use crate::terminal::TerminalProfile;

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
    ) -> Self {
        Self {
            db,
            config,
            template_loader,
            profile,
            i18n,
            line_buffer: LineBuffer::with_encoding(1024, encoding),
            chat_manager,
        }
    }

    /// Create a template context with system variables.
    pub fn create_context(&self) -> TemplateContext {
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

    /// Send data to the client.
    /// Converts LF to CRLF for Telnet compatibility.
    pub async fn send(&self, session: &mut TelnetSession, data: &str) -> Result<()> {
        // Convert LF to CRLF for Telnet (but avoid converting already-CRLF sequences)
        let data = data.replace("\r\n", "\n").replace('\n', "\r\n");
        let encoded = encode_for_client(&data, session.encoding());
        session.stream_mut().write_all(&encoded).await?;
        session.stream_mut().flush().await?;
        Ok(())
    }

    /// Send a line to the client with CRLF.
    pub async fn send_line(&self, session: &mut TelnetSession, data: &str) -> Result<()> {
        self.send(session, &format!("{}\r\n", data)).await
    }

    /// Read a line of input from the client.
    pub async fn read_line(&mut self, session: &mut TelnetSession) -> Result<String> {
        self.line_buffer.clear();
        let mut buf = [0u8; 1];

        loop {
            match session.stream_mut().read(&mut buf).await {
                Ok(0) => {
                    // Connection closed
                    return Ok(String::new());
                }
                Ok(_) => {
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
                        InputResult::Line(line) => {
                            return Ok(line);
                        }
                        InputResult::Buffering => {
                            // Continue reading
                        }
                        InputResult::Cancel | InputResult::Eof => {
                            return Ok(String::new());
                        }
                    }
                }
                Err(e) => {
                    return Err(e.into());
                }
            }
        }
    }

    /// Read a single character.
    pub async fn read_char(&self, session: &mut TelnetSession) -> Result<char> {
        let mut buf = [0u8; 1];
        loop {
            match session.stream_mut().read(&mut buf).await {
                Ok(0) => return Ok('\0'),
                Ok(_) => {
                    let ch = buf[0] as char;
                    if ch.is_ascii_graphic() || ch == '\r' || ch == '\n' {
                        return Ok(ch);
                    }
                }
                Err(e) => return Err(e.into()),
            }
        }
    }

    /// Wait for Enter key press.
    pub async fn wait_for_enter(&self, session: &mut TelnetSession) -> Result<()> {
        self.send(session, self.i18n.t("common.press_enter"))
            .await?;
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
        Ok(())
    }

    /// Parse a number from input.
    pub fn parse_number(&self, input: &str) -> Option<i64> {
        input.trim().parse().ok()
    }

    /// Render a template.
    pub fn render_template(&self, name: &str, context: &TemplateContext) -> Result<String> {
        self.template_loader
            .render(name, self.profile.width, context)
            .map_err(Into::into)
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
}
