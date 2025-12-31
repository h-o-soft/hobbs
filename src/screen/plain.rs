//! Plain text screen implementation.
//!
//! Provides a no-op implementation of the Screen trait for terminals
//! that do not support ANSI escape sequences.

use super::{Color, Screen};

/// Plain text screen implementation (no ANSI support).
///
/// All methods return empty strings, allowing code to use the Screen trait
/// without generating any escape sequences.
#[derive(Debug, Clone, Copy, Default)]
pub struct PlainScreen;

impl PlainScreen {
    /// Create a new plain screen.
    pub fn new() -> Self {
        Self
    }
}

impl Screen for PlainScreen {
    fn fg(&self, _color: Color) -> String {
        String::new()
    }

    fn bg(&self, _color: Color) -> String {
        String::new()
    }

    fn bold(&self) -> String {
        String::new()
    }

    fn underline(&self) -> String {
        String::new()
    }

    fn reverse(&self) -> String {
        String::new()
    }

    fn reset(&self) -> String {
        String::new()
    }

    fn goto(&self, _x: u16, _y: u16) -> String {
        String::new()
    }

    fn home(&self) -> String {
        String::new()
    }

    fn clear_screen(&self) -> String {
        String::new()
    }

    fn clear_line(&self) -> String {
        String::new()
    }

    fn clear_to_end(&self) -> String {
        String::new()
    }

    fn hide_cursor(&self) -> String {
        String::new()
    }

    fn show_cursor(&self) -> String {
        String::new()
    }

    fn save_cursor(&self) -> String {
        String::new()
    }

    fn restore_cursor(&self) -> String {
        String::new()
    }

    fn cursor_up(&self, _n: u16) -> String {
        String::new()
    }

    fn cursor_down(&self, _n: u16) -> String {
        String::new()
    }

    fn cursor_forward(&self, _n: u16) -> String {
        String::new()
    }

    fn cursor_backward(&self, _n: u16) -> String {
        String::new()
    }

    fn is_ansi_enabled(&self) -> bool {
        false
    }

    fn color_text(&self, text: &str, _color: Color) -> String {
        text.to_string()
    }

    fn bold_text(&self, text: &str) -> String {
        text.to_string()
    }

    fn underline_text(&self, text: &str) -> String {
        text.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plain_screen_new() {
        let screen = PlainScreen::new();
        assert!(!screen.is_ansi_enabled());
    }

    #[test]
    fn test_plain_screen_default() {
        let screen = PlainScreen::default();
        assert!(!screen.is_ansi_enabled());
    }

    #[test]
    fn test_fg_colors_empty() {
        let screen = PlainScreen::new();
        assert_eq!(screen.fg(Color::Black), "");
        assert_eq!(screen.fg(Color::Red), "");
        assert_eq!(screen.fg(Color::Green), "");
        assert_eq!(screen.fg(Color::Yellow), "");
        assert_eq!(screen.fg(Color::Blue), "");
        assert_eq!(screen.fg(Color::Magenta), "");
        assert_eq!(screen.fg(Color::Cyan), "");
        assert_eq!(screen.fg(Color::White), "");
    }

    #[test]
    fn test_bg_colors_empty() {
        let screen = PlainScreen::new();
        assert_eq!(screen.bg(Color::Black), "");
        assert_eq!(screen.bg(Color::Red), "");
        assert_eq!(screen.bg(Color::White), "");
    }

    #[test]
    fn test_text_attributes_empty() {
        let screen = PlainScreen::new();
        assert_eq!(screen.bold(), "");
        assert_eq!(screen.underline(), "");
        assert_eq!(screen.reverse(), "");
        assert_eq!(screen.reset(), "");
    }

    #[test]
    fn test_cursor_movement_empty() {
        let screen = PlainScreen::new();
        assert_eq!(screen.goto(10, 20), "");
        assert_eq!(screen.home(), "");
        assert_eq!(screen.cursor_up(5), "");
        assert_eq!(screen.cursor_down(3), "");
        assert_eq!(screen.cursor_forward(10), "");
        assert_eq!(screen.cursor_backward(2), "");
    }

    #[test]
    fn test_clear_empty() {
        let screen = PlainScreen::new();
        assert_eq!(screen.clear_screen(), "");
        assert_eq!(screen.clear_line(), "");
        assert_eq!(screen.clear_to_end(), "");
    }

    #[test]
    fn test_cursor_visibility_empty() {
        let screen = PlainScreen::new();
        assert_eq!(screen.hide_cursor(), "");
        assert_eq!(screen.show_cursor(), "");
    }

    #[test]
    fn test_cursor_save_restore_empty() {
        let screen = PlainScreen::new();
        assert_eq!(screen.save_cursor(), "");
        assert_eq!(screen.restore_cursor(), "");
    }

    #[test]
    fn test_color_text_plain() {
        let screen = PlainScreen::new();
        let result = screen.color_text("Hello", Color::Red);
        assert_eq!(result, "Hello");
    }

    #[test]
    fn test_bold_text_plain() {
        let screen = PlainScreen::new();
        let result = screen.bold_text("Important");
        assert_eq!(result, "Important");
    }

    #[test]
    fn test_underline_text_plain() {
        let screen = PlainScreen::new();
        let result = screen.underline_text("Link");
        assert_eq!(result, "Link");
    }

    #[test]
    fn test_combined_plain_no_escape() {
        let screen = PlainScreen::new();
        // Even with combined calls, no escape sequences
        let output = format!(
            "{}{}{}Hello{}",
            screen.bold(),
            screen.fg(Color::Red),
            screen.bg(Color::White),
            screen.reset()
        );
        assert_eq!(output, "Hello");
    }
}
