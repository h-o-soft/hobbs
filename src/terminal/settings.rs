//! Effective terminal settings of a session, and the rules that decide them.
//!
//! A session's character encoding, terminal profile, output mode and language
//! used to be set in several places at different times. They now live together
//! in [`TerminalSettings`] (held by `TelnetSession`), and every change goes
//! through one of the pure functions in [`resolve`].
//!
//! Known quirks that are still kept are marked `QUIRK(Qn)` (see
//! docs/terminal_model.md).

use crate::i18n::DEFAULT_LOCALE;
use crate::server::encoding::{CharacterEncoding, OutputMode};

use super::TerminalProfile;

/// The terminal settings in effect for a session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerminalSettings {
    /// Terminal profile (screen size, CJK width, template set, ...).
    ///
    /// `profile.encoding` / `profile.output_mode` are the profile's defaults;
    /// the values actually used on the wire are `encoding` / `output_mode`
    /// below, which may differ.
    pub profile: TerminalProfile,
    /// Character encoding used on the wire.
    pub encoding: CharacterEncoding,
    /// Output mode used on the wire (ANSI pass-through / strip / PETSCII).
    pub output_mode: OutputMode,
    /// UI language (locale code such as "ja" or "en").
    pub language: String,
}

impl Default for TerminalSettings {
    fn default() -> Self {
        Self {
            profile: TerminalProfile::default(),
            encoding: CharacterEncoding::default(),
            output_mode: OutputMode::default(),
            language: DEFAULT_LOCALE.to_string(),
        }
    }
}

/// The connection type a client picks right after connecting.
///
/// It decides the encoding (what the client can display) and the language
/// before login. This is the "漢字コード" / charset question common to
/// pasokon-tsushin hosts and BBS software (plan.md D2, D3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionType {
    /// 1) Japanese, ShiftJIS
    JapaneseShiftJis,
    /// 2) Japanese, UTF-8
    JapaneseUtf8,
    /// 3) English, UTF-8
    EnglishUtf8,
    /// 4) English, CP437 (DOS / ANSI terminals)
    EnglishCp437,
    /// 5) Commodore 64, PETSCII
    Commodore64,
}

impl ConnectionType {
    /// All connection types, in menu order (menu number = index + 1).
    pub const ALL: [ConnectionType; 5] = [
        ConnectionType::JapaneseShiftJis,
        ConnectionType::JapaneseUtf8,
        ConnectionType::EnglishUtf8,
        ConnectionType::EnglishCp437,
        ConnectionType::Commodore64,
    ];

    /// Menu number (1-5).
    pub fn number(self) -> usize {
        Self::ALL.iter().position(|&c| c == self).unwrap() + 1
    }

    /// Parse a menu choice. An empty line selects `default`.
    pub fn from_input(input: &str, default: ConnectionType) -> Option<ConnectionType> {
        let input = input.trim();
        if input.is_empty() {
            return Some(default);
        }
        input
            .parse::<usize>()
            .ok()
            .and_then(|n| n.checked_sub(1))
            .and_then(|i| Self::ALL.get(i).copied())
    }

    /// The default choice for a server, from `terminal.default_profile` and
    /// `locale.language`.
    pub fn default_for(profile: &TerminalProfile, language: &str) -> ConnectionType {
        match profile.encoding {
            CharacterEncoding::ShiftJIS => ConnectionType::JapaneseShiftJis,
            CharacterEncoding::Utf8 if language == "ja" => ConnectionType::JapaneseUtf8,
            CharacterEncoding::Utf8 => ConnectionType::EnglishUtf8,
            CharacterEncoding::Cp437 => ConnectionType::EnglishCp437,
            CharacterEncoding::Petscii => ConnectionType::Commodore64,
        }
    }

    /// Character encoding on the wire.
    pub fn encoding(self) -> CharacterEncoding {
        match self {
            ConnectionType::JapaneseShiftJis => CharacterEncoding::ShiftJIS,
            ConnectionType::JapaneseUtf8 | ConnectionType::EnglishUtf8 => CharacterEncoding::Utf8,
            ConnectionType::EnglishCp437 => CharacterEncoding::Cp437,
            ConnectionType::Commodore64 => CharacterEncoding::Petscii,
        }
    }

    /// UI language before login.
    pub fn language(self) -> &'static str {
        match self {
            ConnectionType::JapaneseShiftJis | ConnectionType::JapaneseUtf8 => "ja",
            _ => "en",
        }
    }

    /// The built-in profile for this connection type.
    pub fn profile(self) -> TerminalProfile {
        match self {
            ConnectionType::JapaneseShiftJis => TerminalProfile::standard(),
            ConnectionType::JapaneseUtf8 | ConnectionType::EnglishUtf8 => {
                TerminalProfile::standard_utf8()
            }
            ConnectionType::EnglishCp437 => TerminalProfile::dos(),
            ConnectionType::Commodore64 => TerminalProfile::c64(),
        }
    }
}

/// Rules deciding the settings at each point of a session.
pub mod resolve {
    use super::*;

    /// Make an output mode usable with the encoding on the wire.
    ///
    /// PETSCII control codes only make sense in PETSCII, and ANSI escape
    /// sequences cannot be sent in PETSCII:
    /// - PETSCII encoding + `Ansi` → `PetsciiCtrl`
    /// - other encodings + `PetsciiCtrl` → `Ansi`
    fn compatible_output_mode(mode: OutputMode, encoding: CharacterEncoding) -> OutputMode {
        match (mode, encoding) {
            (OutputMode::Ansi, CharacterEncoding::Petscii) => OutputMode::PetsciiCtrl,
            (OutputMode::PetsciiCtrl, e) if e != CharacterEncoding::Petscii => OutputMode::Ansi,
            (m, _) => m,
        }
    }

    /// Settings when a client connects.
    ///
    /// - profile: the given connect profile (`terminal.default_profile`,
    ///   built-in profiles only).
    /// - output_mode: the connect profile's output mode, made compatible with
    ///   the encoding (see `compatible_output_mode`).
    /// - language: the given language (`locale.language`).
    /// - encoding: unchanged from the session's current value
    ///   (ShiftJIS for a new session).
    ///   QUIRK(Q8): the connect profile's encoding is not used.
    pub fn on_connect(
        current: &TerminalSettings,
        profile: TerminalProfile,
        language: &str,
    ) -> TerminalSettings {
        TerminalSettings {
            output_mode: compatible_output_mode(profile.output_mode, current.encoding),
            profile,
            encoding: current.encoding,
            language: language.to_string(),
        }
    }

    /// Whether text in `encoding` can show Japanese.
    fn can_show_japanese(encoding: CharacterEncoding) -> bool {
        matches!(
            encoding,
            CharacterEncoding::ShiftJIS | CharacterEncoding::Utf8
        )
    }

    /// Whether a saved profile's layout (width, height, CJK width) can be
    /// used on a connection with `encoding`.
    ///
    /// ShiftJIS and UTF-8 profiles are interchangeable (the same kind of PC
    /// terminal); CP437 and PETSCII profiles only fit their own encoding.
    fn layout_fits(profile: &TerminalProfile, encoding: CharacterEncoding) -> bool {
        profile.encoding == encoding
            || (can_show_japanese(profile.encoding) && can_show_japanese(encoding))
    }

    /// Settings after the user picks a connection type (before login).
    ///
    /// - encoding / language: from the connection type.
    /// - profile: `default_profile` if its encoding matches the connection
    ///   type (so an operator's 40-column ShiftJIS default is kept), otherwise
    ///   the connection type's built-in profile.
    /// - output_mode: the profile's, made compatible with the encoding.
    pub fn on_connection_selected(
        connection: ConnectionType,
        default_profile: &TerminalProfile,
    ) -> TerminalSettings {
        let encoding = connection.encoding();
        let profile = if default_profile.encoding == encoding {
            default_profile.clone()
        } else {
            connection.profile()
        };
        TerminalSettings {
            output_mode: compatible_output_mode(profile.output_mode, encoding),
            profile,
            encoding,
            language: connection.language().to_string(),
        }
    }

    /// Settings after a successful login.
    ///
    /// The connection type chosen for this session decides what the client
    /// can display, so it wins over saved settings (plan.md D2):
    /// - encoding: unchanged (the connection type's). The saved encoding is
    ///   not used, so a user who once chose PETSCII is not locked out when
    ///   connecting from another terminal (Issue #269).
    /// - language: the user's saved language, or English when the encoding
    ///   cannot show Japanese (CP437, PETSCII).
    /// - profile: the saved profile (`from_name(terminal)`) if its layout fits
    ///   the encoding, otherwise the connection's profile.
    ///   QUIRK(Q2): custom profiles from config are not consulted, and an
    ///   unknown name falls back to "standard".
    /// - output_mode: the profile's, made compatible with the encoding.
    pub fn on_login(
        current: &TerminalSettings,
        language: &str,
        terminal: &str,
    ) -> TerminalSettings {
        let saved = TerminalProfile::from_name(terminal);
        let profile = if layout_fits(&saved, current.encoding) {
            saved
        } else {
            current.profile.clone()
        };
        let language = if can_show_japanese(current.encoding) {
            language
        } else {
            "en"
        };
        TerminalSettings {
            output_mode: compatible_output_mode(profile.output_mode, current.encoding),
            profile,
            encoding: current.encoding,
            language: language.to_string(),
        }
    }

    /// Settings after the user saves the settings screen.
    ///
    /// - encoding / language: the new values.
    /// - profile: `TerminalProfile::from_name(terminal)` if a profile was
    ///   selected, otherwise unchanged. QUIRK(Q2) as in [`on_login`].
    /// - output_mode: the (new) profile's output mode, made compatible with
    ///   the encoding.
    pub fn on_settings_changed(
        current: &TerminalSettings,
        language: &str,
        encoding: CharacterEncoding,
        terminal: Option<&str>,
    ) -> TerminalSettings {
        let profile = match terminal {
            Some(name) => TerminalProfile::from_name(name),
            None => current.profile.clone(),
        };
        TerminalSettings {
            output_mode: compatible_output_mode(profile.output_mode, encoding),
            profile,
            encoding,
            language: language.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::resolve::*;
    use super::*;

    fn connected(profile: &str) -> TerminalSettings {
        on_connect(
            &TerminalSettings::default(),
            TerminalProfile::from_name(profile),
            "en",
        )
    }

    fn selected(connection: ConnectionType) -> TerminalSettings {
        on_connection_selected(connection, &TerminalProfile::standard())
    }

    #[test]
    fn test_default() {
        let s = TerminalSettings::default();
        assert_eq!(s.profile, TerminalProfile::standard());
        assert_eq!(s.encoding, CharacterEncoding::ShiftJIS);
        assert_eq!(s.output_mode, OutputMode::Ansi);
        assert_eq!(s.language, "ja");
    }

    /// on_connect for every built-in profile (golden a_connect__*).
    #[test]
    fn test_on_connect_table() {
        let table = [
            ("standard", OutputMode::Ansi),
            ("standard_utf8", OutputMode::Ansi),
            ("dos", OutputMode::Ansi),
            // The C64 profile's PetsciiCtrl is not used over ShiftJIS (Q8).
            ("c64", OutputMode::Ansi),
            ("40col_sjis", OutputMode::Ansi),
            ("jterm40", OutputMode::Ansi),
            ("40col_utf8", OutputMode::Ansi),
        ];
        for (name, mode) in table {
            let s = connected(name);
            assert_eq!(s.profile.name, name);
            assert_eq!(s.output_mode, mode, "{name}");
            // Q8: the session's encoding (ShiftJIS) until a connection type
            // is chosen.
            assert_eq!(s.encoding, CharacterEncoding::ShiftJIS, "{name}");
            assert_eq!(s.language, "en");
        }
    }

    #[test]
    fn test_connection_type_from_input() {
        let d = ConnectionType::EnglishCp437;
        assert_eq!(
            ConnectionType::from_input("1", d),
            Some(ConnectionType::JapaneseShiftJis)
        );
        assert_eq!(
            ConnectionType::from_input(" 5 ", d),
            Some(ConnectionType::Commodore64)
        );
        assert_eq!(ConnectionType::from_input("", d), Some(d));
        assert_eq!(ConnectionType::from_input("0", d), None);
        assert_eq!(ConnectionType::from_input("6", d), None);
        assert_eq!(ConnectionType::from_input("E", d), None);
        for c in ConnectionType::ALL {
            assert_eq!(
                ConnectionType::from_input(&c.number().to_string(), d),
                Some(c)
            );
        }
    }

    #[test]
    fn test_connection_type_default_for() {
        let table = [
            ("standard", "ja", ConnectionType::JapaneseShiftJis),
            ("standard", "en", ConnectionType::JapaneseShiftJis),
            ("jterm40", "ja", ConnectionType::JapaneseShiftJis),
            ("standard_utf8", "ja", ConnectionType::JapaneseUtf8),
            ("standard_utf8", "en", ConnectionType::EnglishUtf8),
            ("dos", "en", ConnectionType::EnglishCp437),
            ("c64", "en", ConnectionType::Commodore64),
        ];
        for (profile, lang, expected) in table {
            assert_eq!(
                ConnectionType::default_for(&TerminalProfile::from_name(profile), lang),
                expected,
                "{profile} {lang}"
            );
        }
    }

    /// Connection type → settings before login (golden a_connect__*, d_*).
    #[test]
    fn test_on_connection_selected_table() {
        let table = [
            (
                ConnectionType::JapaneseShiftJis,
                "standard",
                CharacterEncoding::ShiftJIS,
                OutputMode::Ansi,
                "ja",
            ),
            (
                ConnectionType::JapaneseUtf8,
                "standard_utf8",
                CharacterEncoding::Utf8,
                OutputMode::Ansi,
                "ja",
            ),
            (
                ConnectionType::EnglishUtf8,
                "standard_utf8",
                CharacterEncoding::Utf8,
                OutputMode::Ansi,
                "en",
            ),
            (
                ConnectionType::EnglishCp437,
                "dos",
                CharacterEncoding::Cp437,
                OutputMode::Ansi,
                "en",
            ),
            (
                ConnectionType::Commodore64,
                "c64",
                CharacterEncoding::Petscii,
                OutputMode::PetsciiCtrl,
                "en",
            ),
        ];
        for (c, profile, enc, mode, lang) in table {
            let s = selected(c);
            assert_eq!(s.profile.name, profile, "{c:?}");
            assert_eq!(s.encoding, enc, "{c:?}");
            assert_eq!(s.output_mode, mode, "{c:?}");
            assert_eq!(s.language, lang, "{c:?}");
        }
    }

    #[test]
    fn test_on_connection_selected_keeps_matching_default_profile() {
        // An operator's 40-column ShiftJIS default is kept for ShiftJIS.
        let s = on_connection_selected(
            ConnectionType::JapaneseShiftJis,
            &TerminalProfile::jterm40(),
        );
        assert_eq!(s.profile.name, "jterm40");
        // ... but not for other connection types.
        let s = on_connection_selected(ConnectionType::Commodore64, &TerminalProfile::jterm40());
        assert_eq!(s.profile.name, "c64");
    }

    /// Login (golden b_login__*): the connection type wins over the saved
    /// encoding; the saved layout is used when it fits.
    #[test]
    fn test_on_login_table() {
        // (connection, saved terminal, saved language) -> (profile, language)
        let table = [
            (
                ConnectionType::JapaneseShiftJis,
                "standard",
                "ja",
                "standard",
                "ja",
            ),
            (
                ConnectionType::JapaneseShiftJis,
                "jterm40",
                "ja",
                "jterm40",
                "ja",
            ),
            (
                ConnectionType::JapaneseShiftJis,
                "40col_utf8",
                "ja",
                "40col_utf8",
                "ja",
            ),
            (
                ConnectionType::JapaneseUtf8,
                "standard",
                "en",
                "standard",
                "en",
            ),
            (
                ConnectionType::EnglishUtf8,
                "standard_utf8",
                "ja",
                "standard_utf8",
                "ja",
            ),
            // Saved layouts that do not fit the connection fall back to the
            // connection's profile.
            (
                ConnectionType::JapaneseShiftJis,
                "c64",
                "ja",
                "standard",
                "ja",
            ),
            (
                ConnectionType::JapaneseShiftJis,
                "dos",
                "en",
                "standard",
                "en",
            ),
            (ConnectionType::Commodore64, "standard", "ja", "c64", "en"),
            (ConnectionType::EnglishCp437, "standard", "ja", "dos", "en"),
            (ConnectionType::Commodore64, "c64_ansi", "en", "c64", "en"),
            (ConnectionType::EnglishCp437, "dos", "en", "dos", "en"),
        ];
        for (c, terminal, lang, profile, expected_lang) in table {
            let before = selected(c);
            let s = on_login(&before, lang, terminal);
            assert_eq!(s.profile.name, profile, "{c:?} {terminal}");
            assert_eq!(s.encoding, c.encoding(), "{c:?} {terminal}");
            assert_eq!(s.language, expected_lang, "{c:?} {terminal}");
            let expected_mode = if c == ConnectionType::Commodore64 {
                OutputMode::PetsciiCtrl
            } else {
                OutputMode::Ansi
            };
            assert_eq!(s.output_mode, expected_mode, "{c:?} {terminal}");
        }
    }

    /// Issue #269: a user whose saved encoding is PETSCII can still log in
    /// from a PC; the saved encoding is not applied.
    #[test]
    fn test_on_login_ignores_saved_encoding() {
        let before = selected(ConnectionType::JapaneseUtf8);
        let s = on_login(&before, "ja", "c64");
        assert_eq!(s.encoding, CharacterEncoding::Utf8);
        assert_eq!(s.output_mode, OutputMode::Ansi);
        assert_eq!(s.profile.name, "standard_utf8");
    }

    #[test]
    fn test_on_login_unknown_profile_falls_back_to_standard() {
        // Q2
        let s = on_login(
            &selected(ConnectionType::JapaneseShiftJis),
            "en",
            "no_such_profile",
        );
        assert_eq!(s.profile, TerminalProfile::standard());
        assert_eq!(s.output_mode, OutputMode::Ansi);
    }

    /// The output mode always matches the encoding actually used.
    #[test]
    fn test_output_mode_follows_effective_encoding() {
        let before = selected(ConnectionType::JapaneseShiftJis);
        let s = on_settings_changed(&before, "ja", CharacterEncoding::Utf8, Some("c64"));
        assert_eq!(s.output_mode, OutputMode::Ansi);
        let s = on_settings_changed(&before, "en", CharacterEncoding::Petscii, Some("standard"));
        assert_eq!(s.output_mode, OutputMode::PetsciiCtrl);
        // Connecting with a C64 default profile: the wire is still ShiftJIS
        // until a connection type is chosen (Q8), so ANSI is used.
        assert_eq!(connected("c64").output_mode, OutputMode::Ansi);
    }

    /// Settings screen (golden c_settings__*).
    #[test]
    fn test_on_settings_changed() {
        let before = on_login(
            &selected(ConnectionType::JapaneseShiftJis),
            "ja",
            "standard",
        );
        let s = on_settings_changed(&before, "en", CharacterEncoding::Petscii, Some("c64"));
        assert_eq!(s.profile, TerminalProfile::c64());
        assert_eq!(s.encoding, CharacterEncoding::Petscii);
        assert_eq!(s.language, "en");
        assert_eq!(s.output_mode, OutputMode::PetsciiCtrl);

        let kept = on_settings_changed(&s, "ja", CharacterEncoding::Utf8, None);
        assert_eq!(kept.profile, s.profile);
        // UTF-8 on the wire: ANSI, not PETSCII control codes.
        assert_eq!(kept.output_mode, OutputMode::Ansi);
        assert_eq!(kept.encoding, CharacterEncoding::Utf8);
        assert_eq!(kept.language, "ja");
    }
}
