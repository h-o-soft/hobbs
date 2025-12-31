//! ANSI escape sequence implementation.
//!
//! Provides screen control using ANSI escape sequences for terminals
//! that support them.

use super::{Color, Screen};

/// Escape character for ANSI sequences.
const ESC: char = '\x1b';

/// ANSI-capable screen implementation.
///
/// This implementation generates standard ANSI escape sequences
/// for terminal control and decoration.
#[derive(Debug, Clone, Copy, Default)]
pub struct AnsiScreen;

impl AnsiScreen {
    /// Create a new ANSI screen.
    pub fn new() -> Self {
        Self
    }
}

impl Screen for AnsiScreen {
    fn fg(&self, color: Color) -> String {
        format!("{ESC}[{}m", color.fg_code())
    }

    fn bg(&self, color: Color) -> String {
        format!("{ESC}[{}m", color.bg_code())
    }

    fn bold(&self) -> String {
        format!("{ESC}[1m")
    }

    fn underline(&self) -> String {
        format!("{ESC}[4m")
    }

    fn reverse(&self) -> String {
        format!("{ESC}[7m")
    }

    fn reset(&self) -> String {
        format!("{ESC}[0m")
    }

    fn goto(&self, x: u16, y: u16) -> String {
        format!("{ESC}[{y};{x}H")
    }

    fn home(&self) -> String {
        format!("{ESC}[H")
    }

    fn clear_screen(&self) -> String {
        format!("{ESC}[2J")
    }

    fn clear_line(&self) -> String {
        format!("{ESC}[K")
    }

    fn clear_to_end(&self) -> String {
        format!("{ESC}[J")
    }

    fn hide_cursor(&self) -> String {
        format!("{ESC}[?25l")
    }

    fn show_cursor(&self) -> String {
        format!("{ESC}[?25h")
    }

    fn save_cursor(&self) -> String {
        format!("{ESC}[s")
    }

    fn restore_cursor(&self) -> String {
        format!("{ESC}[u")
    }

    fn cursor_up(&self, n: u16) -> String {
        if n == 0 {
            String::new()
        } else {
            format!("{ESC}[{n}A")
        }
    }

    fn cursor_down(&self, n: u16) -> String {
        if n == 0 {
            String::new()
        } else {
            format!("{ESC}[{n}B")
        }
    }

    fn cursor_forward(&self, n: u16) -> String {
        if n == 0 {
            String::new()
        } else {
            format!("{ESC}[{n}C")
        }
    }

    fn cursor_backward(&self, n: u16) -> String {
        if n == 0 {
            String::new()
        } else {
            format!("{ESC}[{n}D")
        }
    }

    fn is_ansi_enabled(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ansi_screen_new() {
        let screen = AnsiScreen::new();
        assert!(screen.is_ansi_enabled());
    }

    #[test]
    fn test_ansi_screen_default() {
        let screen = AnsiScreen::default();
        assert!(screen.is_ansi_enabled());
    }

    #[test]
    fn test_fg_colors() {
        let screen = AnsiScreen::new();
        assert_eq!(screen.fg(Color::Black), "\x1b[30m");
        assert_eq!(screen.fg(Color::Red), "\x1b[31m");
        assert_eq!(screen.fg(Color::Green), "\x1b[32m");
        assert_eq!(screen.fg(Color::Yellow), "\x1b[33m");
        assert_eq!(screen.fg(Color::Blue), "\x1b[34m");
        assert_eq!(screen.fg(Color::Magenta), "\x1b[35m");
        assert_eq!(screen.fg(Color::Cyan), "\x1b[36m");
        assert_eq!(screen.fg(Color::White), "\x1b[37m");
    }

    #[test]
    fn test_bg_colors() {
        let screen = AnsiScreen::new();
        assert_eq!(screen.bg(Color::Black), "\x1b[40m");
        assert_eq!(screen.bg(Color::Red), "\x1b[41m");
        assert_eq!(screen.bg(Color::Green), "\x1b[42m");
        assert_eq!(screen.bg(Color::Yellow), "\x1b[43m");
        assert_eq!(screen.bg(Color::Blue), "\x1b[44m");
        assert_eq!(screen.bg(Color::Magenta), "\x1b[45m");
        assert_eq!(screen.bg(Color::Cyan), "\x1b[46m");
        assert_eq!(screen.bg(Color::White), "\x1b[47m");
    }

    #[test]
    fn test_text_attributes() {
        let screen = AnsiScreen::new();
        assert_eq!(screen.bold(), "\x1b[1m");
        assert_eq!(screen.underline(), "\x1b[4m");
        assert_eq!(screen.reverse(), "\x1b[7m");
        assert_eq!(screen.reset(), "\x1b[0m");
    }

    #[test]
    fn test_cursor_movement() {
        let screen = AnsiScreen::new();
        assert_eq!(screen.goto(10, 20), "\x1b[20;10H");
        assert_eq!(screen.goto(1, 1), "\x1b[1;1H");
        assert_eq!(screen.home(), "\x1b[H");
    }

    #[test]
    fn test_clear() {
        let screen = AnsiScreen::new();
        assert_eq!(screen.clear_screen(), "\x1b[2J");
        assert_eq!(screen.clear_line(), "\x1b[K");
        assert_eq!(screen.clear_to_end(), "\x1b[J");
    }

    #[test]
    fn test_cursor_visibility() {
        let screen = AnsiScreen::new();
        assert_eq!(screen.hide_cursor(), "\x1b[?25l");
        assert_eq!(screen.show_cursor(), "\x1b[?25h");
    }

    #[test]
    fn test_cursor_save_restore() {
        let screen = AnsiScreen::new();
        assert_eq!(screen.save_cursor(), "\x1b[s");
        assert_eq!(screen.restore_cursor(), "\x1b[u");
    }

    #[test]
    fn test_cursor_relative_movement() {
        let screen = AnsiScreen::new();
        assert_eq!(screen.cursor_up(5), "\x1b[5A");
        assert_eq!(screen.cursor_down(3), "\x1b[3B");
        assert_eq!(screen.cursor_forward(10), "\x1b[10C");
        assert_eq!(screen.cursor_backward(2), "\x1b[2D");
    }

    #[test]
    fn test_cursor_relative_movement_zero() {
        let screen = AnsiScreen::new();
        assert_eq!(screen.cursor_up(0), "");
        assert_eq!(screen.cursor_down(0), "");
        assert_eq!(screen.cursor_forward(0), "");
        assert_eq!(screen.cursor_backward(0), "");
    }

    #[test]
    fn test_color_text() {
        let screen = AnsiScreen::new();
        let result = screen.color_text("Hello", Color::Red);
        assert_eq!(result, "\x1b[31mHello\x1b[0m");
    }

    #[test]
    fn test_bold_text() {
        let screen = AnsiScreen::new();
        let result = screen.bold_text("Important");
        assert_eq!(result, "\x1b[1mImportant\x1b[0m");
    }

    #[test]
    fn test_underline_text() {
        let screen = AnsiScreen::new();
        let result = screen.underline_text("Link");
        assert_eq!(result, "\x1b[4mLink\x1b[0m");
    }

    #[test]
    fn test_combined_sequences() {
        let screen = AnsiScreen::new();
        // Combine multiple attributes
        let output = format!(
            "{}{}{}Hello{}",
            screen.bold(),
            screen.fg(Color::Red),
            screen.bg(Color::White),
            screen.reset()
        );
        assert_eq!(output, "\x1b[1m\x1b[31m\x1b[47mHello\x1b[0m");
    }

    #[test]
    fn test_screen_clear_and_home() {
        let screen = AnsiScreen::new();
        // Common pattern: clear screen and go home
        let output = format!("{}{}", screen.clear_screen(), screen.home());
        assert_eq!(output, "\x1b[2J\x1b[H");
    }
}
