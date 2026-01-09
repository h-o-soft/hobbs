//! Terminal profile definitions.
//!
//! This module defines terminal profiles that describe the characteristics
//! of different terminal types (screen size, CJK width, ANSI support, encoding, etc.).

use crate::server::encoding::{CharacterEncoding, OutputMode};

/// A terminal profile that describes the characteristics of a terminal.
///
/// This structure unifies display settings with encoding and output mode,
/// allowing each terminal type to have sensible defaults.
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
    /// Default character encoding for wire communication.
    pub encoding: CharacterEncoding,
    /// Default output mode for escape sequence handling.
    pub output_mode: OutputMode,
    /// Template directory name (relative to templates/).
    /// Typically "80" for 80-column or "40" for 40-column terminals.
    pub template_dir: String,
}

impl TerminalProfile {
    /// Create a new terminal profile with the given parameters.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        name: impl Into<String>,
        width: u16,
        height: u16,
        cjk_width: u8,
        ansi_enabled: bool,
        encoding: CharacterEncoding,
        output_mode: OutputMode,
        template_dir: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            width,
            height,
            cjk_width,
            ansi_enabled,
            encoding,
            output_mode,
            template_dir: template_dir.into(),
        }
    }

    /// Create a standard terminal profile (80x24, CJK double-width, ANSI enabled, ShiftJIS).
    ///
    /// This is the default profile for Japanese terminals like TeraTerm, etc.
    pub fn standard() -> Self {
        Self {
            name: "standard".to_string(),
            width: 80,
            height: 24,
            cjk_width: 2,
            ansi_enabled: true,
            encoding: CharacterEncoding::ShiftJIS,
            output_mode: OutputMode::Ansi,
            template_dir: "80".to_string(),
        }
    }

    /// Create a standard UTF-8 terminal profile (80x24, CJK double-width, ANSI enabled, UTF-8).
    ///
    /// This is the default profile for modern UTF-8 terminals like xterm, etc.
    pub fn standard_utf8() -> Self {
        Self {
            name: "standard_utf8".to_string(),
            width: 80,
            height: 24,
            cjk_width: 2,
            ansi_enabled: true,
            encoding: CharacterEncoding::Utf8,
            output_mode: OutputMode::Ansi,
            template_dir: "80".to_string(),
        }
    }

    /// Create a DOS terminal profile (80x25, CJK single-width, ANSI enabled, CP437).
    ///
    /// This profile is for IBM PC compatible DOS terminals.
    pub fn dos() -> Self {
        Self {
            name: "dos".to_string(),
            width: 80,
            height: 25,
            cjk_width: 1,
            ansi_enabled: true,
            encoding: CharacterEncoding::Cp437,
            output_mode: OutputMode::Ansi,
            template_dir: "80".to_string(),
        }
    }

    /// Create a Commodore 64 terminal profile (40x25, CJK single-width, no ANSI, PETSCII).
    ///
    /// This profile is for Commodore 64 terminals in plain mode (no escape sequences).
    pub fn c64() -> Self {
        Self {
            name: "c64".to_string(),
            width: 40,
            height: 25,
            cjk_width: 1,
            ansi_enabled: false,
            encoding: CharacterEncoding::Petscii,
            output_mode: OutputMode::Plain,
            template_dir: "40".to_string(),
        }
    }

    /// Create a Commodore 64 terminal profile with PETSCII control codes.
    ///
    /// This profile is for Commodore 64 terminals that use PETSCII control codes
    /// for colors and cursor movement (not ANSI escape sequences).
    pub fn c64_petscii() -> Self {
        Self {
            name: "c64_petscii".to_string(),
            width: 40,
            height: 25,
            cjk_width: 1,
            ansi_enabled: false,
            encoding: CharacterEncoding::Petscii,
            output_mode: OutputMode::PetsciiCtrl,
            template_dir: "40".to_string(),
        }
    }

    /// Create a Commodore 64 ANSI terminal profile (40x25, CJK single-width, ANSI enabled).
    ///
    /// This profile is for Commodore 64 terminals with ANSI support added
    /// (e.g., through software terminal emulation like CCGMS).
    pub fn c64_ansi() -> Self {
        Self {
            name: "c64_ansi".to_string(),
            width: 40,
            height: 25,
            cjk_width: 1,
            ansi_enabled: true,
            encoding: CharacterEncoding::Petscii,
            output_mode: OutputMode::Ansi,
            template_dir: "40".to_string(),
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

impl TerminalProfile {
    /// Create a terminal profile from a profile name string.
    ///
    /// Returns the matching preset profile, or the standard profile for unknown names.
    ///
    /// # Arguments
    ///
    /// * `name` - Profile name ("standard", "standard_utf8", "dos", "c64", "c64_petscii", "c64_ansi")
    ///
    /// # Example
    ///
    /// ```
    /// use hobbs::terminal::TerminalProfile;
    ///
    /// let profile = TerminalProfile::from_name("c64");
    /// assert_eq!(profile.width, 40);
    /// ```
    pub fn from_name(name: &str) -> Self {
        match name.to_lowercase().as_str() {
            "standard_utf8" | "utf8" => Self::standard_utf8(),
            "dos" | "ibmpc" | "cp437" => Self::dos(),
            "c64" => Self::c64(),
            "c64_petscii" | "petscii" => Self::c64_petscii(),
            "c64_ansi" => Self::c64_ansi(),
            _ => Self::standard(),
        }
    }

    /// Create a terminal profile from a config definition.
    ///
    /// This method creates a profile from a ProfileConfig struct,
    /// parsing string values for encoding and output_mode.
    ///
    /// # Arguments
    ///
    /// * `config` - The profile configuration
    ///
    /// # Example
    ///
    /// ```
    /// use hobbs::terminal::TerminalProfile;
    /// use hobbs::config::ProfileConfig;
    ///
    /// // Custom PC-98 profile
    /// let config = ProfileConfig {
    ///     name: "pc98".to_string(),
    ///     width: 80,
    ///     height: 25,
    ///     cjk_width: 2,
    ///     ansi_enabled: true,
    ///     encoding: "shiftjis".to_string(),
    ///     output_mode: "ansi".to_string(),
    ///     template_dir: "80".to_string(),
    /// };
    /// let profile = TerminalProfile::from_config(&config);
    /// assert_eq!(profile.name, "pc98");
    /// ```
    pub fn from_config(config: &crate::config::ProfileConfig) -> Self {
        let encoding = config
            .encoding
            .parse()
            .unwrap_or(CharacterEncoding::ShiftJIS);
        let output_mode = config.output_mode.parse().unwrap_or(OutputMode::Ansi);

        Self {
            name: config.name.clone(),
            width: config.width,
            height: config.height,
            cjk_width: config.cjk_width,
            ansi_enabled: config.ansi_enabled,
            encoding,
            output_mode,
            template_dir: config.template_dir.clone(),
        }
    }

    /// Create a terminal profile from a name, checking custom profiles first.
    ///
    /// This method first looks for a matching custom profile in the provided list,
    /// then falls back to built-in profiles.
    ///
    /// # Arguments
    ///
    /// * `name` - Profile name
    /// * `custom_profiles` - List of custom profile configurations
    ///
    /// # Example
    ///
    /// ```
    /// use hobbs::terminal::TerminalProfile;
    ///
    /// // Without custom profiles, uses built-in
    /// let profile = TerminalProfile::from_name_with_custom("c64", &[]);
    /// assert_eq!(profile.width, 40);
    /// ```
    pub fn from_name_with_custom(
        name: &str,
        custom_profiles: &[crate::config::ProfileConfig],
    ) -> Self {
        // First check custom profiles
        let name_lower = name.to_lowercase();
        if let Some(config) = custom_profiles
            .iter()
            .find(|p| p.name.to_lowercase() == name_lower)
        {
            return Self::from_config(config);
        }

        // Fall back to built-in profiles
        Self::from_name(name)
    }

    /// Get all available profile names.
    pub fn available_profiles() -> &'static [&'static str] {
        &[
            "standard",
            "standard_utf8",
            "dos",
            "c64",
            "c64_petscii",
            "c64_ansi",
        ]
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
        assert_eq!(profile.encoding, CharacterEncoding::ShiftJIS);
        assert_eq!(profile.output_mode, OutputMode::Ansi);
        assert_eq!(profile.template_dir, "80");
    }

    #[test]
    fn test_standard_utf8_profile() {
        let profile = TerminalProfile::standard_utf8();
        assert_eq!(profile.name, "standard_utf8");
        assert_eq!(profile.width, 80);
        assert_eq!(profile.height, 24);
        assert_eq!(profile.cjk_width, 2);
        assert!(profile.ansi_enabled);
        assert_eq!(profile.encoding, CharacterEncoding::Utf8);
        assert_eq!(profile.output_mode, OutputMode::Ansi);
        assert_eq!(profile.template_dir, "80");
    }

    #[test]
    fn test_dos_profile() {
        let profile = TerminalProfile::dos();
        assert_eq!(profile.name, "dos");
        assert_eq!(profile.width, 80);
        assert_eq!(profile.height, 25);
        assert_eq!(profile.cjk_width, 1);
        assert!(profile.ansi_enabled);
        assert_eq!(profile.encoding, CharacterEncoding::Cp437);
        assert_eq!(profile.output_mode, OutputMode::Ansi);
        assert_eq!(profile.template_dir, "80");
    }

    #[test]
    fn test_c64_profile() {
        let profile = TerminalProfile::c64();
        assert_eq!(profile.name, "c64");
        assert_eq!(profile.width, 40);
        assert_eq!(profile.height, 25);
        assert_eq!(profile.cjk_width, 1);
        assert!(!profile.ansi_enabled);
        assert_eq!(profile.encoding, CharacterEncoding::Petscii);
        assert_eq!(profile.output_mode, OutputMode::Plain);
        assert_eq!(profile.template_dir, "40");
    }

    #[test]
    fn test_c64_petscii_profile() {
        let profile = TerminalProfile::c64_petscii();
        assert_eq!(profile.name, "c64_petscii");
        assert_eq!(profile.width, 40);
        assert_eq!(profile.height, 25);
        assert_eq!(profile.cjk_width, 1);
        assert!(!profile.ansi_enabled);
        assert_eq!(profile.encoding, CharacterEncoding::Petscii);
        assert_eq!(profile.output_mode, OutputMode::PetsciiCtrl);
        assert_eq!(profile.template_dir, "40");
    }

    #[test]
    fn test_c64_ansi_profile() {
        let profile = TerminalProfile::c64_ansi();
        assert_eq!(profile.name, "c64_ansi");
        assert_eq!(profile.width, 40);
        assert_eq!(profile.height, 25);
        assert_eq!(profile.cjk_width, 1);
        assert!(profile.ansi_enabled);
        assert_eq!(profile.encoding, CharacterEncoding::Petscii);
        assert_eq!(profile.output_mode, OutputMode::Ansi);
        assert_eq!(profile.template_dir, "40");
    }

    #[test]
    fn test_custom_profile() {
        let profile = TerminalProfile::new(
            "custom",
            132,
            43,
            2,
            true,
            CharacterEncoding::Utf8,
            OutputMode::Ansi,
            "80",
        );
        assert_eq!(profile.name, "custom");
        assert_eq!(profile.width, 132);
        assert_eq!(profile.height, 43);
        assert_eq!(profile.cjk_width, 2);
        assert!(profile.ansi_enabled);
        assert_eq!(profile.encoding, CharacterEncoding::Utf8);
        assert_eq!(profile.output_mode, OutputMode::Ansi);
        assert_eq!(profile.template_dir, "80");
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
    fn test_from_name_standard() {
        let profile = TerminalProfile::from_name("standard");
        assert_eq!(profile, TerminalProfile::standard());
    }

    #[test]
    fn test_from_name_standard_utf8() {
        let profile = TerminalProfile::from_name("standard_utf8");
        assert_eq!(profile, TerminalProfile::standard_utf8());
        // Also test alias
        assert_eq!(
            TerminalProfile::from_name("utf8"),
            TerminalProfile::standard_utf8()
        );
    }

    #[test]
    fn test_from_name_dos() {
        let profile = TerminalProfile::from_name("dos");
        assert_eq!(profile, TerminalProfile::dos());
        // Also test aliases
        assert_eq!(TerminalProfile::from_name("ibmpc"), TerminalProfile::dos());
        assert_eq!(TerminalProfile::from_name("cp437"), TerminalProfile::dos());
    }

    #[test]
    fn test_from_name_c64() {
        let profile = TerminalProfile::from_name("c64");
        assert_eq!(profile, TerminalProfile::c64());
    }

    #[test]
    fn test_from_name_c64_petscii() {
        let profile = TerminalProfile::from_name("c64_petscii");
        assert_eq!(profile, TerminalProfile::c64_petscii());
        // Also test alias
        assert_eq!(
            TerminalProfile::from_name("petscii"),
            TerminalProfile::c64_petscii()
        );
    }

    #[test]
    fn test_from_name_c64_ansi() {
        let profile = TerminalProfile::from_name("c64_ansi");
        assert_eq!(profile, TerminalProfile::c64_ansi());
    }

    #[test]
    fn test_from_name_case_insensitive() {
        assert_eq!(TerminalProfile::from_name("C64"), TerminalProfile::c64());
        assert_eq!(
            TerminalProfile::from_name("C64_ANSI"),
            TerminalProfile::c64_ansi()
        );
        assert_eq!(
            TerminalProfile::from_name("STANDARD"),
            TerminalProfile::standard()
        );
        assert_eq!(TerminalProfile::from_name("DOS"), TerminalProfile::dos());
    }

    #[test]
    fn test_from_name_unknown() {
        let profile = TerminalProfile::from_name("unknown");
        assert_eq!(profile, TerminalProfile::standard());
    }

    #[test]
    fn test_available_profiles() {
        let profiles = TerminalProfile::available_profiles();
        assert_eq!(profiles.len(), 6);
        assert!(profiles.contains(&"standard"));
        assert!(profiles.contains(&"standard_utf8"));
        assert!(profiles.contains(&"dos"));
        assert!(profiles.contains(&"c64"));
        assert!(profiles.contains(&"c64_petscii"));
        assert!(profiles.contains(&"c64_ansi"));
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

    #[test]
    fn test_from_config() {
        use crate::config::ProfileConfig;

        let config = ProfileConfig {
            name: "pc98".to_string(),
            width: 80,
            height: 25,
            cjk_width: 2,
            ansi_enabled: true,
            encoding: "shiftjis".to_string(),
            output_mode: "ansi".to_string(),
            template_dir: "80".to_string(),
        };

        let profile = TerminalProfile::from_config(&config);
        assert_eq!(profile.name, "pc98");
        assert_eq!(profile.width, 80);
        assert_eq!(profile.height, 25);
        assert_eq!(profile.cjk_width, 2);
        assert!(profile.ansi_enabled);
        assert_eq!(profile.encoding, CharacterEncoding::ShiftJIS);
        assert_eq!(profile.output_mode, OutputMode::Ansi);
        assert_eq!(profile.template_dir, "80");
    }

    #[test]
    fn test_from_config_with_petscii() {
        use crate::config::ProfileConfig;

        let config = ProfileConfig {
            name: "vic20".to_string(),
            width: 22,
            height: 23,
            cjk_width: 1,
            ansi_enabled: false,
            encoding: "petscii".to_string(),
            output_mode: "petscii_ctrl".to_string(),
            template_dir: "40".to_string(),
        };

        let profile = TerminalProfile::from_config(&config);
        assert_eq!(profile.name, "vic20");
        assert_eq!(profile.width, 22);
        assert_eq!(profile.height, 23);
        assert_eq!(profile.cjk_width, 1);
        assert!(!profile.ansi_enabled);
        assert_eq!(profile.encoding, CharacterEncoding::Petscii);
        assert_eq!(profile.output_mode, OutputMode::PetsciiCtrl);
        assert_eq!(profile.template_dir, "40");
    }

    #[test]
    fn test_from_config_invalid_encoding() {
        use crate::config::ProfileConfig;

        let config = ProfileConfig {
            name: "test".to_string(),
            width: 80,
            height: 24,
            cjk_width: 2,
            ansi_enabled: true,
            encoding: "invalid".to_string(),
            output_mode: "ansi".to_string(),
            template_dir: "80".to_string(),
        };

        let profile = TerminalProfile::from_config(&config);
        // Should fall back to ShiftJIS
        assert_eq!(profile.encoding, CharacterEncoding::ShiftJIS);
    }

    #[test]
    fn test_from_name_with_custom() {
        use crate::config::ProfileConfig;

        let custom_profiles = vec![ProfileConfig {
            name: "custom1".to_string(),
            width: 132,
            height: 44,
            cjk_width: 1,
            ansi_enabled: true,
            encoding: "utf8".to_string(),
            output_mode: "ansi".to_string(),
            template_dir: "80".to_string(),
        }];

        // Custom profile should be found
        let profile = TerminalProfile::from_name_with_custom("custom1", &custom_profiles);
        assert_eq!(profile.name, "custom1");
        assert_eq!(profile.width, 132);
        assert_eq!(profile.height, 44);

        // Built-in profile should still work
        let c64 = TerminalProfile::from_name_with_custom("c64", &custom_profiles);
        assert_eq!(c64.name, "c64");
        assert_eq!(c64.width, 40);

        // Unknown should fall back to standard
        let unknown = TerminalProfile::from_name_with_custom("unknown", &custom_profiles);
        assert_eq!(unknown.name, "standard");
    }

    #[test]
    fn test_from_name_with_custom_case_insensitive() {
        use crate::config::ProfileConfig;

        let custom_profiles = vec![ProfileConfig {
            name: "MyProfile".to_string(),
            width: 100,
            height: 50,
            cjk_width: 2,
            ansi_enabled: true,
            encoding: "utf8".to_string(),
            output_mode: "ansi".to_string(),
            template_dir: "80".to_string(),
        }];

        // Should match case-insensitively
        let profile = TerminalProfile::from_name_with_custom("myprofile", &custom_profiles);
        assert_eq!(profile.name, "MyProfile");
        assert_eq!(profile.width, 100);

        let profile2 = TerminalProfile::from_name_with_custom("MYPROFILE", &custom_profiles);
        assert_eq!(profile2.name, "MyProfile");
    }
}
