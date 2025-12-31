//! Terminal profile definitions.
//!
//! This module defines terminal profiles that describe the characteristics
//! of different terminal types (screen size, CJK width, ANSI support, etc.).

/// A terminal profile that describes the characteristics of a terminal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerminalProfile {
    /// Profile name.
    pub name: String,
    /// Screen width in columns.
    pub width: u16,
    /// Screen height in rows.
    pub height: u16,
    /// Display width of CJK (full-width) characters.
    /// Set to 1 for terminals that display CJK characters as single-width,
    /// or 2 for terminals that display them as double-width.
    pub cjk_width: u8,
    /// Whether ANSI escape sequences are supported.
    pub ansi_enabled: bool,
}

impl TerminalProfile {
    /// Create a new terminal profile with the given parameters.
    pub fn new(
        name: impl Into<String>,
        width: u16,
        height: u16,
        cjk_width: u8,
        ansi_enabled: bool,
    ) -> Self {
        Self {
            name: name.into(),
            width,
            height,
            cjk_width,
            ansi_enabled,
        }
    }

    /// Create a standard terminal profile (80x24, CJK double-width, ANSI enabled).
    ///
    /// This is the default profile for modern terminals like xterm, TeraTerm, etc.
    pub fn standard() -> Self {
        Self {
            name: "standard".to_string(),
            width: 80,
            height: 24,
            cjk_width: 2,
            ansi_enabled: true,
        }
    }

    /// Create a Commodore 64 terminal profile (40x25, CJK single-width, no ANSI).
    ///
    /// This profile is for Commodore 64 terminals that display all characters
    /// as single-width and do not support ANSI escape sequences.
    pub fn c64() -> Self {
        Self {
            name: "c64".to_string(),
            width: 40,
            height: 25,
            cjk_width: 1,
            ansi_enabled: false,
        }
    }

    /// Create a Commodore 64 ANSI terminal profile (40x25, CJK single-width, ANSI enabled).
    ///
    /// This profile is for Commodore 64 terminals with ANSI support added
    /// (e.g., through software terminal emulation).
    pub fn c64_ansi() -> Self {
        Self {
            name: "c64_ansi".to_string(),
            width: 40,
            height: 25,
            cjk_width: 1,
            ansi_enabled: true,
        }
    }

    /// Calculate the display width of a string for this terminal profile.
    ///
    /// For terminals with `cjk_width == 1`, all characters are counted as 1.
    /// For terminals with `cjk_width == 2`, ASCII characters are counted as 1
    /// and non-ASCII characters (assumed to be CJK) are counted as 2.
    ///
    /// # Arguments
    ///
    /// * `s` - The string to measure.
    ///
    /// # Returns
    ///
    /// The display width of the string in columns.
    ///
    /// # Example
    ///
    /// ```
    /// use hobbs::terminal::TerminalProfile;
    ///
    /// let standard = TerminalProfile::standard();
    /// assert_eq!(standard.display_width("Hello"), 5);
    /// assert_eq!(standard.display_width("こんにちは"), 10); // 5 CJK chars × 2
    ///
    /// let c64 = TerminalProfile::c64();
    /// assert_eq!(c64.display_width("Hello"), 5);
    /// assert_eq!(c64.display_width("こんにちは"), 5); // 5 CJK chars × 1
    /// ```
    pub fn display_width(&self, s: &str) -> usize {
        if self.cjk_width == 1 {
            s.chars().count()
        } else {
            s.chars().map(|c| if c.is_ascii() { 1 } else { 2 }).sum()
        }
    }

    /// Truncate a string to fit within the specified display width.
    ///
    /// This function truncates the string so that its display width does not
    /// exceed the specified maximum width. It respects the terminal's CJK width
    /// setting when calculating character widths.
    ///
    /// # Arguments
    ///
    /// * `s` - The string to truncate.
    /// * `max_width` - The maximum display width in columns.
    ///
    /// # Returns
    ///
    /// A new string that fits within the specified width.
    ///
    /// # Example
    ///
    /// ```
    /// use hobbs::terminal::TerminalProfile;
    ///
    /// let standard = TerminalProfile::standard();
    /// assert_eq!(standard.truncate_to_width("Hello, World!", 5), "Hello");
    /// assert_eq!(standard.truncate_to_width("こんにちは", 6), "こんに"); // 3 chars × 2 = 6
    ///
    /// let c64 = TerminalProfile::c64();
    /// assert_eq!(c64.truncate_to_width("こんにちは", 3), "こんに"); // 3 chars × 1 = 3
    /// ```
    pub fn truncate_to_width(&self, s: &str, max_width: usize) -> String {
        let mut result = String::new();
        let mut current_width = 0;

        for c in s.chars() {
            let char_width = if self.cjk_width == 1 || c.is_ascii() {
                1
            } else {
                2
            };

            if current_width + char_width > max_width {
                break;
            }

            result.push(c);
            current_width += char_width;
        }

        result
    }

    /// Pad a string to exactly the specified display width.
    ///
    /// If the string is shorter than the specified width, it is padded with spaces.
    /// If the string is longer, it is truncated.
    ///
    /// # Arguments
    ///
    /// * `s` - The string to pad.
    /// * `target_width` - The target display width in columns.
    ///
    /// # Returns
    ///
    /// A new string with exactly the specified display width.
    pub fn pad_to_width(&self, s: &str, target_width: usize) -> String {
        let current_width = self.display_width(s);

        if current_width >= target_width {
            self.truncate_to_width(s, target_width)
        } else {
            let padding = target_width - current_width;
            format!("{}{}", s, " ".repeat(padding))
        }
    }

    /// Center a string within the specified display width.
    ///
    /// If the string is shorter than the specified width, it is centered with spaces.
    /// If the string is longer, it is truncated.
    ///
    /// # Arguments
    ///
    /// * `s` - The string to center.
    /// * `target_width` - The target display width in columns.
    ///
    /// # Returns
    ///
    /// A new string centered within the specified display width.
    pub fn center_to_width(&self, s: &str, target_width: usize) -> String {
        let current_width = self.display_width(s);

        if current_width >= target_width {
            self.truncate_to_width(s, target_width)
        } else {
            let total_padding = target_width - current_width;
            let left_padding = total_padding / 2;
            let right_padding = total_padding - left_padding;
            format!(
                "{}{}{}",
                " ".repeat(left_padding),
                s,
                " ".repeat(right_padding)
            )
        }
    }
}

impl Default for TerminalProfile {
    fn default() -> Self {
        Self::standard()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_standard_profile() {
        let profile = TerminalProfile::standard();
        assert_eq!(profile.name, "standard");
        assert_eq!(profile.width, 80);
        assert_eq!(profile.height, 24);
        assert_eq!(profile.cjk_width, 2);
        assert!(profile.ansi_enabled);
    }

    #[test]
    fn test_c64_profile() {
        let profile = TerminalProfile::c64();
        assert_eq!(profile.name, "c64");
        assert_eq!(profile.width, 40);
        assert_eq!(profile.height, 25);
        assert_eq!(profile.cjk_width, 1);
        assert!(!profile.ansi_enabled);
    }

    #[test]
    fn test_c64_ansi_profile() {
        let profile = TerminalProfile::c64_ansi();
        assert_eq!(profile.name, "c64_ansi");
        assert_eq!(profile.width, 40);
        assert_eq!(profile.height, 25);
        assert_eq!(profile.cjk_width, 1);
        assert!(profile.ansi_enabled);
    }

    #[test]
    fn test_custom_profile() {
        let profile = TerminalProfile::new("custom", 132, 43, 2, true);
        assert_eq!(profile.name, "custom");
        assert_eq!(profile.width, 132);
        assert_eq!(profile.height, 43);
        assert_eq!(profile.cjk_width, 2);
        assert!(profile.ansi_enabled);
    }

    #[test]
    fn test_default_profile() {
        let profile = TerminalProfile::default();
        assert_eq!(profile, TerminalProfile::standard());
    }

    #[test]
    fn test_display_width_ascii_standard() {
        let profile = TerminalProfile::standard();
        assert_eq!(profile.display_width("Hello"), 5);
        assert_eq!(profile.display_width("Hello, World!"), 13);
        assert_eq!(profile.display_width(""), 0);
    }

    #[test]
    fn test_display_width_cjk_standard() {
        let profile = TerminalProfile::standard();
        assert_eq!(profile.display_width("こんにちは"), 10); // 5 chars × 2
        assert_eq!(profile.display_width("漢字"), 4); // 2 chars × 2
        assert_eq!(profile.display_width("テスト"), 6); // 3 chars × 2
    }

    #[test]
    fn test_display_width_mixed_standard() {
        let profile = TerminalProfile::standard();
        assert_eq!(profile.display_width("Hello世界"), 9); // 5 + 2×2
        assert_eq!(profile.display_width("a日b本c語"), 9); // 3 + 3×2
    }

    #[test]
    fn test_display_width_c64() {
        let profile = TerminalProfile::c64();
        assert_eq!(profile.display_width("Hello"), 5);
        assert_eq!(profile.display_width("こんにちは"), 5); // 5 chars × 1
        assert_eq!(profile.display_width("Hello世界"), 7); // 5 + 2×1
    }

    #[test]
    fn test_truncate_ascii_standard() {
        let profile = TerminalProfile::standard();
        assert_eq!(profile.truncate_to_width("Hello, World!", 5), "Hello");
        assert_eq!(
            profile.truncate_to_width("Hello, World!", 13),
            "Hello, World!"
        );
        assert_eq!(
            profile.truncate_to_width("Hello, World!", 100),
            "Hello, World!"
        );
    }

    #[test]
    fn test_truncate_cjk_standard() {
        let profile = TerminalProfile::standard();
        assert_eq!(profile.truncate_to_width("こんにちは", 6), "こんに"); // 3 chars × 2 = 6
        assert_eq!(profile.truncate_to_width("こんにちは", 7), "こんに"); // Can't fit 4th char (would be 8)
        assert_eq!(profile.truncate_to_width("こんにちは", 10), "こんにちは");
    }

    #[test]
    fn test_truncate_mixed_standard() {
        let profile = TerminalProfile::standard();
        assert_eq!(profile.truncate_to_width("Hello世界", 7), "Hello世"); // 5 + 2 = 7
        assert_eq!(profile.truncate_to_width("Hello世界", 6), "Hello"); // Can't fit 世 (would be 7)
    }

    #[test]
    fn test_truncate_c64() {
        let profile = TerminalProfile::c64();
        assert_eq!(profile.truncate_to_width("こんにちは", 3), "こんに");
        assert_eq!(profile.truncate_to_width("Hello世界", 6), "Hello世");
    }

    #[test]
    fn test_truncate_empty() {
        let profile = TerminalProfile::standard();
        assert_eq!(profile.truncate_to_width("", 10), "");
        assert_eq!(profile.truncate_to_width("Hello", 0), "");
    }

    #[test]
    fn test_pad_to_width() {
        let profile = TerminalProfile::standard();
        assert_eq!(profile.pad_to_width("Hello", 10), "Hello     ");
        assert_eq!(profile.pad_to_width("Hello", 5), "Hello");
        assert_eq!(profile.pad_to_width("Hello, World!", 5), "Hello");
    }

    #[test]
    fn test_pad_cjk() {
        let profile = TerminalProfile::standard();
        assert_eq!(profile.pad_to_width("こんにちは", 14), "こんにちは    "); // 10 + 4 spaces
    }

    #[test]
    fn test_center_to_width() {
        let profile = TerminalProfile::standard();
        assert_eq!(profile.center_to_width("Hello", 11), "   Hello   ");
        assert_eq!(profile.center_to_width("Hello", 10), "  Hello   "); // 5 padding, 2 left, 3 right
        assert_eq!(profile.center_to_width("Hello", 5), "Hello");
        assert_eq!(profile.center_to_width("Hello, World!", 5), "Hello");
    }

    #[test]
    fn test_center_cjk() {
        let profile = TerminalProfile::standard();
        assert_eq!(profile.center_to_width("こんにちは", 14), "  こんにちは  ");
        // 10 + 4 (2 each side)
    }

    #[test]
    fn test_profile_equality() {
        let p1 = TerminalProfile::standard();
        let p2 = TerminalProfile::standard();
        let p3 = TerminalProfile::c64();
        assert_eq!(p1, p2);
        assert_ne!(p1, p3);
    }

    #[test]
    fn test_profile_clone() {
        let p1 = TerminalProfile::standard();
        let p2 = p1.clone();
        assert_eq!(p1, p2);
    }

    #[test]
    fn test_half_width_katakana() {
        // Half-width katakana should be treated as non-ASCII (width 2) on standard
        let standard = TerminalProfile::standard();
        assert_eq!(standard.display_width("ｱｲｳ"), 6); // 3 chars × 2

        // On C64, all chars are width 1
        let c64 = TerminalProfile::c64();
        assert_eq!(c64.display_width("ｱｲｳ"), 3); // 3 chars × 1
    }
}
