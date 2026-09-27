//! Effective terminal settings of a session, and the rules that decide them.
//!
//! A session's character encoding, terminal profile, output mode and language
//! used to be set in several places at different times. They now live together
//! in [`TerminalSettings`] (held by `TelnetSession`), and every change goes
//! through one of the pure functions in [`resolve`].
//!
//! The rules in [`resolve`] reproduce the existing behavior exactly, including
//! known quirks (see plan.md, Issue #325). Quirks are marked `QUIRK(Qn)`; they
//! are intentionally kept here and are to be fixed in a later,
//! behavior-changing phase.

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

/// Rules deciding the settings at each point of a session.
pub mod resolve {
    use super::*;

    /// Settings when a client connects.
    ///
    /// - profile: the given connect profile (`terminal.default_profile`,
    ///   built-in profiles only).
    /// - output_mode: the connect profile's output mode.
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
            output_mode: profile.output_mode,
            profile,
            encoding: current.encoding,
            language: language.to_string(),
        }
    }

    /// Settings after the pre-registration / guest language selection.
    ///
    /// `input` is the raw line typed by the user.
    /// - `E`/`1`: English, UTF-8
    /// - `J`/`2`: Japanese, ShiftJIS
    /// - `U`/`3`: Japanese, UTF-8
    /// - anything else: English, UTF-8
    ///
    /// Profile and output mode are unchanged.
    pub fn on_language_selected(current: &TerminalSettings, input: &str) -> TerminalSettings {
        let (language, encoding) = match input.trim().to_uppercase().as_str() {
            "E" | "1" => ("en", CharacterEncoding::Utf8),
            "J" | "2" => ("ja", CharacterEncoding::ShiftJIS),
            "U" | "3" => ("ja", CharacterEncoding::Utf8),
            _ => ("en", CharacterEncoding::Utf8),
        };
        TerminalSettings {
            encoding,
            language: language.to_string(),
            ..current.clone()
        }
    }

    /// Settings after a successful login, from the user's saved preferences.
    ///
    /// - encoding / language: the user's saved values.
    /// - profile: `TerminalProfile::from_name(terminal)`.
    ///   QUIRK(Q2): custom profiles from config are not consulted, and an
    ///   unknown name falls back to "standard".
    /// - output_mode: unchanged.
    ///   QUIRK(Q1): the profile's output mode is not applied.
    pub fn on_login(
        current: &TerminalSettings,
        language: &str,
        terminal: &str,
        encoding: CharacterEncoding,
    ) -> TerminalSettings {
        TerminalSettings {
            profile: TerminalProfile::from_name(terminal),
            encoding,
            output_mode: current.output_mode,
            language: language.to_string(),
        }
    }

    /// Settings after the user saves the settings screen.
    ///
    /// - encoding / language: the new values.
    /// - profile: `TerminalProfile::from_name(terminal)` if a profile was
    ///   selected, otherwise unchanged. QUIRK(Q2) as in [`on_login`].
    /// - output_mode: unchanged. QUIRK(Q1) as in [`on_login`].
    pub fn on_settings_changed(
        current: &TerminalSettings,
        language: &str,
        encoding: CharacterEncoding,
        terminal: Option<&str>,
    ) -> TerminalSettings {
        TerminalSettings {
            profile: match terminal {
                Some(name) => TerminalProfile::from_name(name),
                None => current.profile.clone(),
            },
            encoding,
            output_mode: current.output_mode,
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
            ("c64", OutputMode::Plain),
            ("c64_petscii", OutputMode::PetsciiCtrl),
            ("c64_ansi", OutputMode::Ansi),
            ("40col_sjis", OutputMode::Ansi),
            ("jterm40", OutputMode::Ansi),
            ("40col_utf8", OutputMode::Ansi),
        ];
        for (name, mode) in table {
            let s = connected(name);
            assert_eq!(s.profile.name, name);
            assert_eq!(s.output_mode, mode, "{name}");
            // Q8: always the session's encoding (ShiftJIS), not the profile's.
            assert_eq!(s.encoding, CharacterEncoding::ShiftJIS, "{name}");
            assert_eq!(s.language, "en");
        }
    }

    #[test]
    fn test_on_connect_keeps_session_encoding() {
        let current = TerminalSettings {
            encoding: CharacterEncoding::Utf8,
            ..TerminalSettings::default()
        };
        let s = on_connect(&current, TerminalProfile::c64(), "ja");
        assert_eq!(s.encoding, CharacterEncoding::Utf8);
    }

    /// Language selection (golden d_guest__*, d_register__J).
    #[test]
    fn test_on_language_selected_table() {
        let table = [
            ("E", "en", CharacterEncoding::Utf8),
            ("1", "en", CharacterEncoding::Utf8),
            ("e", "en", CharacterEncoding::Utf8),
            ("J", "ja", CharacterEncoding::ShiftJIS),
            (" 2 ", "ja", CharacterEncoding::ShiftJIS),
            ("U", "ja", CharacterEncoding::Utf8),
            ("3", "ja", CharacterEncoding::Utf8),
            ("X", "en", CharacterEncoding::Utf8),
            ("", "en", CharacterEncoding::Utf8),
        ];
        let before = connected("c64_petscii");
        for (input, lang, enc) in table {
            let s = on_language_selected(&before, input);
            assert_eq!(s.language, lang, "{input:?}");
            assert_eq!(s.encoding, enc, "{input:?}");
            assert_eq!(s.profile, before.profile);
            assert_eq!(s.output_mode, before.output_mode);
        }
    }

    /// Login (golden b_login__*).
    #[test]
    fn test_on_login_table() {
        let before = connected("standard");
        let table = [
            ("standard", CharacterEncoding::ShiftJIS),
            ("standard_utf8", CharacterEncoding::Utf8),
            ("dos", CharacterEncoding::Cp437),
            ("c64", CharacterEncoding::Petscii),
            ("c64_petscii", CharacterEncoding::Petscii),
            ("c64_ansi", CharacterEncoding::Petscii),
            ("40col_sjis", CharacterEncoding::ShiftJIS),
            ("jterm40", CharacterEncoding::ShiftJIS),
            ("40col_utf8", CharacterEncoding::Utf8),
            // Saved terminal and encoding disagree (E1).
            ("standard", CharacterEncoding::Utf8),
            ("c64", CharacterEncoding::ShiftJIS),
        ];
        for (terminal, enc) in table {
            let s = on_login(&before, "ja", terminal, enc);
            assert_eq!(s.profile, TerminalProfile::from_name(terminal));
            assert_eq!(s.encoding, enc, "{terminal}");
            assert_eq!(s.language, "ja");
            // Q1: output mode stays as it was at connect.
            assert_eq!(s.output_mode, OutputMode::Ansi, "{terminal}");
        }
    }

    #[test]
    fn test_on_login_unknown_profile_falls_back_to_standard() {
        // Q2
        let s = on_login(
            &connected("c64"),
            "en",
            "no_such_profile",
            CharacterEncoding::ShiftJIS,
        );
        assert_eq!(s.profile, TerminalProfile::standard());
        assert_eq!(s.output_mode, OutputMode::Plain);
    }

    /// Settings screen (golden c_settings__*).
    #[test]
    fn test_on_settings_changed() {
        let before = on_login(
            &connected("standard"),
            "ja",
            "standard",
            CharacterEncoding::ShiftJIS,
        );
        let s = on_settings_changed(
            &before,
            "en",
            CharacterEncoding::Petscii,
            Some("c64_petscii"),
        );
        assert_eq!(s.profile, TerminalProfile::c64_petscii());
        assert_eq!(s.encoding, CharacterEncoding::Petscii);
        assert_eq!(s.language, "en");
        // Q1
        assert_eq!(s.output_mode, OutputMode::Ansi);

        let kept = on_settings_changed(&s, "ja", CharacterEncoding::Utf8, None);
        assert_eq!(kept.profile, s.profile);
        assert_eq!(kept.encoding, CharacterEncoding::Utf8);
        assert_eq!(kept.language, "ja");
    }
}
