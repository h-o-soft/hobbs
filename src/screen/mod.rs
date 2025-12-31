//! Screen display module.
//!
//! Provides ANSI escape sequences for screen decoration and a plain text fallback.

mod ansi;
mod plain;

pub use ansi::AnsiScreen;
pub use plain::PlainScreen;

/// Terminal colors (ANSI standard 8 colors).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Color {
    /// Black (color code 0).
    Black = 0,
    /// Red (color code 1).
    Red = 1,
    /// Green (color code 2).
    Green = 2,
    /// Yellow (color code 3).
    Yellow = 3,
    /// Blue (color code 4).
    Blue = 4,
    /// Magenta (color code 5).
    Magenta = 5,
    /// Cyan (color code 6).
    Cyan = 6,
    /// White (color code 7).
    White = 7,
}

impl Color {
    /// Get the ANSI color code for foreground.
    pub fn fg_code(self) -> u8 {
        30 + self as u8
    }

    /// Get the ANSI color code for background.
    pub fn bg_code(self) -> u8 {
        40 + self as u8
    }
}

/// Screen output trait for terminal decoration.
///
/// This trait provides methods for generating terminal control sequences.
/// Implementations include `AnsiScreen` for ANSI-capable terminals and
/// `PlainScreen` for terminals without ANSI support.
pub trait Screen: Send + Sync {
    /// Set foreground (text) color.
    ///
    /// # Arguments
    ///
    /// * `color` - The color to set.
    ///
    /// # Returns
    ///
    /// An escape sequence string to set the color.
    fn fg(&self, color: Color) -> String;

    /// Set background color.
    ///
    /// # Arguments
    ///
    /// * `color` - The color to set.
    ///
    /// # Returns
    ///
    /// An escape sequence string to set the color.
    fn bg(&self, color: Color) -> String;

    /// Enable bold text.
    fn bold(&self) -> String;

    /// Enable underlined text.
    fn underline(&self) -> String;

    /// Enable reversed (inverse) colors.
    fn reverse(&self) -> String;

    /// Reset all text attributes to default.
    fn reset(&self) -> String;

    /// Move cursor to specified position.
    ///
    /// # Arguments
    ///
    /// * `x` - Column number (1-based).
    /// * `y` - Row number (1-based).
    ///
    /// # Returns
    ///
    /// An escape sequence string to move the cursor.
    fn goto(&self, x: u16, y: u16) -> String;

    /// Move cursor to home position (top-left corner).
    fn home(&self) -> String;

    /// Clear the entire screen.
    fn clear_screen(&self) -> String;

    /// Clear from cursor to end of line.
    fn clear_line(&self) -> String;

    /// Clear from cursor to end of screen.
    fn clear_to_end(&self) -> String;

    /// Format text with a foreground color.
    ///
    /// # Arguments
    ///
    /// * `text` - The text to colorize.
    /// * `color` - The foreground color.
    ///
    /// # Returns
    ///
    /// The text wrapped with color escape sequences.
    fn color_text(&self, text: &str, color: Color) -> String {
        format!("{}{}{}", self.fg(color), text, self.reset())
    }

    /// Format text as bold.
    ///
    /// # Arguments
    ///
    /// * `text` - The text to make bold.
    ///
    /// # Returns
    ///
    /// The text wrapped with bold escape sequences.
    fn bold_text(&self, text: &str) -> String {
        format!("{}{}{}", self.bold(), text, self.reset())
    }

    /// Format text as underlined.
    ///
    /// # Arguments
    ///
    /// * `text` - The text to underline.
    ///
    /// # Returns
    ///
    /// The text wrapped with underline escape sequences.
    fn underline_text(&self, text: &str) -> String {
        format!("{}{}{}", self.underline(), text, self.reset())
    }

    /// Hide cursor.
    fn hide_cursor(&self) -> String;

    /// Show cursor.
    fn show_cursor(&self) -> String;

    /// Save cursor position.
    fn save_cursor(&self) -> String;

    /// Restore cursor position.
    fn restore_cursor(&self) -> String;

    /// Move cursor up by n lines.
    fn cursor_up(&self, n: u16) -> String;

    /// Move cursor down by n lines.
    fn cursor_down(&self, n: u16) -> String;

    /// Move cursor forward (right) by n columns.
    fn cursor_forward(&self, n: u16) -> String;

    /// Move cursor backward (left) by n columns.
    fn cursor_backward(&self, n: u16) -> String;

    /// Check if ANSI escape sequences are enabled.
    fn is_ansi_enabled(&self) -> bool;
}

/// Create a screen instance based on ANSI support.
///
/// # Arguments
///
/// * `ansi_enabled` - Whether ANSI escape sequences are supported.
///
/// # Returns
///
/// A boxed screen implementation.
///
/// # Example
///
/// ```
/// use hobbs::screen::{create_screen, Color};
///
/// let screen = create_screen(true);
/// assert!(screen.is_ansi_enabled());
/// assert!(!screen.fg(Color::Red).is_empty());
///
/// let plain = create_screen(false);
/// assert!(!plain.is_ansi_enabled());
/// assert!(plain.fg(Color::Red).is_empty());
/// ```
pub fn create_screen(ansi_enabled: bool) -> Box<dyn Screen> {
    if ansi_enabled {
        Box::new(AnsiScreen)
    } else {
        Box::new(PlainScreen)
    }
}

/// Create a screen instance from a terminal profile.
///
/// This is a convenience function that creates the appropriate screen
/// implementation based on the terminal profile's ANSI support setting.
///
/// # Arguments
///
/// * `profile` - The terminal profile.
///
/// # Returns
///
/// A boxed screen implementation.
///
/// # Example
///
/// ```
/// use hobbs::screen::create_screen_from_profile;
/// use hobbs::terminal::TerminalProfile;
///
/// let standard = TerminalProfile::standard();
/// let screen = create_screen_from_profile(&standard);
/// assert!(screen.is_ansi_enabled());
///
/// let c64 = TerminalProfile::c64();
/// let plain = create_screen_from_profile(&c64);
/// assert!(!plain.is_ansi_enabled());
/// ```
pub fn create_screen_from_profile(profile: &crate::terminal::TerminalProfile) -> Box<dyn Screen> {
    create_screen(profile.ansi_enabled)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_fg_code() {
        assert_eq!(Color::Black.fg_code(), 30);
        assert_eq!(Color::Red.fg_code(), 31);
        assert_eq!(Color::Green.fg_code(), 32);
        assert_eq!(Color::Yellow.fg_code(), 33);
        assert_eq!(Color::Blue.fg_code(), 34);
        assert_eq!(Color::Magenta.fg_code(), 35);
        assert_eq!(Color::Cyan.fg_code(), 36);
        assert_eq!(Color::White.fg_code(), 37);
    }

    #[test]
    fn test_color_bg_code() {
        assert_eq!(Color::Black.bg_code(), 40);
        assert_eq!(Color::Red.bg_code(), 41);
        assert_eq!(Color::Green.bg_code(), 42);
        assert_eq!(Color::Yellow.bg_code(), 43);
        assert_eq!(Color::Blue.bg_code(), 44);
        assert_eq!(Color::Magenta.bg_code(), 45);
        assert_eq!(Color::Cyan.bg_code(), 46);
        assert_eq!(Color::White.bg_code(), 47);
    }

    #[test]
    fn test_create_screen_ansi() {
        let screen = create_screen(true);
        assert!(screen.is_ansi_enabled());
    }

    #[test]
    fn test_create_screen_plain() {
        let screen = create_screen(false);
        assert!(!screen.is_ansi_enabled());
    }

    #[test]
    fn test_color_equality() {
        assert_eq!(Color::Red, Color::Red);
        assert_ne!(Color::Red, Color::Blue);
    }

    #[test]
    fn test_color_copy() {
        let c1 = Color::Green;
        let c2 = c1;
        assert_eq!(c1, c2);
    }

    #[test]
    fn test_create_screen_from_profile_standard() {
        let profile = crate::terminal::TerminalProfile::standard();
        let screen = create_screen_from_profile(&profile);
        assert!(screen.is_ansi_enabled());
    }

    #[test]
    fn test_create_screen_from_profile_c64() {
        let profile = crate::terminal::TerminalProfile::c64();
        let screen = create_screen_from_profile(&profile);
        assert!(!screen.is_ansi_enabled());
    }

    #[test]
    fn test_create_screen_from_profile_c64_ansi() {
        let profile = crate::terminal::TerminalProfile::c64_ansi();
        let screen = create_screen_from_profile(&profile);
        assert!(screen.is_ansi_enabled());
    }
}
