//! Wire output: turning text into the bytes sent to a client.
//!
//! This is the single place where outgoing text is converted for the wire:
//! newline normalization → output mode processing (ANSI pass-through / strip /
//! PETSCII control conversion) → character encoding.
//!
//! It also holds the echo filtering used by `ScreenContext` input reading.

use tokio::io::AsyncWriteExt;

use super::encoding::{encode_for_client, process_output_mode, CharacterEncoding, OutputMode};
use super::input::EchoMode;
use super::session::TelnetSession;

/// How newlines in outgoing text are treated by [`to_wire`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NewlinePolicy {
    /// Convert every LF (and existing CRLF) to CRLF for Telnet.
    Normalize,
    /// Leave newlines as they are (the caller already normalized them,
    /// or the text is sent raw).
    AsIs,
}

/// Convert LF to CRLF, without doubling existing CRLF sequences.
pub fn normalize_newlines(text: &str) -> String {
    text.replace("\r\n", "\n").replace('\n', "\r\n")
}

/// Convert text to the bytes sent on the wire.
pub fn to_wire(
    text: &str,
    encoding: CharacterEncoding,
    output_mode: OutputMode,
    newline: NewlinePolicy,
) -> Vec<u8> {
    let processed = match newline {
        NewlinePolicy::Normalize => process_output_mode(&normalize_newlines(text), output_mode),
        NewlinePolicy::AsIs => process_output_mode(text, output_mode),
    };
    encode_for_client(&processed, encoding)
}

/// Decide which bytes `ScreenContext` echoes back for the `LineBuffer` echo
/// output, according to the echo mode.
///
/// - `Normal`: the echo as-is.
/// - `Password` / `Masked(c)`: a single non-BS byte becomes `*` / `c`;
///   a backspace sequence (starting with BS, longer than 1) is echoed as-is;
///   anything else (e.g. the CRLF at end of line) is not echoed.
///
/// Note: `SessionHandler` does not use this filter; it writes the
/// `LineBuffer` echo verbatim (plan.md Q11).
pub fn screen_echo_bytes(echo: &[u8], mode: EchoMode) -> Vec<u8> {
    match mode {
        EchoMode::Normal => echo.to_vec(),
        EchoMode::Password | EchoMode::Masked(_) => {
            if echo.len() == 1 && echo[0] != b'\x08' {
                match mode {
                    EchoMode::Masked(c) => vec![c as u8],
                    _ => b"*".to_vec(),
                }
            } else if echo.len() > 1 && echo[0] == b'\x08' {
                echo.to_vec()
            } else {
                Vec::new()
            }
        }
    }
}

/// Write the `ScreenContext` echo for `echo` to the client.
///
/// Write errors are ignored, as input reading continues regardless.
pub async fn write_screen_echo(session: &mut TelnetSession, echo: &[u8], mode: EchoMode) {
    if echo.is_empty() {
        return;
    }
    let bytes = screen_echo_bytes(echo, mode);
    if !bytes.is_empty() {
        let _ = session.stream_mut().write_all(&bytes).await;
        let _ = session.stream_mut().flush().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_newlines() {
        assert_eq!(normalize_newlines("a\nb"), "a\r\nb");
        assert_eq!(normalize_newlines("a\r\nb"), "a\r\nb");
        assert_eq!(normalize_newlines("a\rb"), "a\rb");
    }

    #[test]
    fn test_to_wire_newline_policy() {
        let enc = CharacterEncoding::Utf8;
        let mode = OutputMode::Ansi;
        assert_eq!(
            to_wire("a\nb", enc, mode, NewlinePolicy::Normalize),
            b"a\r\nb"
        );
        assert_eq!(to_wire("a\nb", enc, mode, NewlinePolicy::AsIs), b"a\nb");
    }

    #[test]
    fn test_to_wire_output_mode_and_encoding() {
        assert_eq!(
            to_wire(
                "\x1b[31mA\x1b[0m",
                CharacterEncoding::Utf8,
                OutputMode::Plain,
                NewlinePolicy::AsIs
            ),
            b"A"
        );
        assert_eq!(
            to_wire(
                "あ",
                CharacterEncoding::ShiftJIS,
                OutputMode::Ansi,
                NewlinePolicy::AsIs
            ),
            vec![0x82, 0xA0]
        );
    }

    #[test]
    fn test_screen_echo_bytes() {
        assert_eq!(screen_echo_bytes(b"a", EchoMode::Normal), b"a");
        assert_eq!(screen_echo_bytes(b"\r\n", EchoMode::Normal), b"\r\n");
        assert_eq!(screen_echo_bytes(b"a", EchoMode::Password), b"*");
        assert_eq!(screen_echo_bytes(b"a", EchoMode::Masked('#')), b"#");
        assert_eq!(
            screen_echo_bytes(b"\x08 \x08", EchoMode::Password),
            b"\x08 \x08"
        );
        assert!(screen_echo_bytes(b"\r\n", EchoMode::Password).is_empty());
        assert!(screen_echo_bytes(b"\x08", EchoMode::Masked('#')).is_empty());
    }
}
