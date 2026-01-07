//! Input handling for Telnet sessions.
//!
//! This module provides line buffering, special key handling, and
//! input processing for Telnet connections.

use super::encoding::{decode_from_client, CharacterEncoding};
use super::telnet::control;

/// Result of processing input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputResult {
    /// A complete line was entered.
    Line(String),
    /// Input is still being buffered.
    Buffering,
    /// User pressed Ctrl+C (cancel).
    Cancel,
    /// User pressed Ctrl+D (EOF/logout).
    Eof,
}

/// Echo mode for input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EchoMode {
    /// Normal echo - characters are echoed back.
    #[default]
    Normal,
    /// No echo - for password input.
    Password,
    /// Echo with masking character.
    Masked(char),
}

/// A line buffer for input processing.
#[derive(Debug)]
pub struct LineBuffer {
    /// The current buffer contents.
    buffer: Vec<u8>,
    /// Maximum buffer size.
    max_size: usize,
    /// Current echo mode.
    echo_mode: EchoMode,
    /// Character encoding for decoding input.
    encoding: CharacterEncoding,
    /// Pending bytes for multi-byte character echo.
    /// Used to buffer incomplete multi-byte characters before echoing.
    pending_echo: Vec<u8>,
}

impl LineBuffer {
    /// Create a new line buffer with the given maximum size.
    pub fn new(max_size: usize) -> Self {
        Self {
            buffer: Vec::with_capacity(max_size.min(1024)),
            max_size,
            echo_mode: EchoMode::Normal,
            encoding: CharacterEncoding::default(),
            pending_echo: Vec::with_capacity(4),
        }
    }

    /// Create a new line buffer with a specific encoding.
    pub fn with_encoding(max_size: usize, encoding: CharacterEncoding) -> Self {
        Self {
            buffer: Vec::with_capacity(max_size.min(1024)),
            max_size,
            echo_mode: EchoMode::Normal,
            encoding,
            pending_echo: Vec::with_capacity(4),
        }
    }

    /// Create a new line buffer with default settings.
    pub fn with_defaults() -> Self {
        Self::new(1024)
    }

    /// Get the current echo mode.
    pub fn echo_mode(&self) -> EchoMode {
        self.echo_mode
    }

    /// Set the echo mode.
    pub fn set_echo_mode(&mut self, mode: EchoMode) {
        self.echo_mode = mode;
    }

    /// Get the current character encoding.
    pub fn encoding(&self) -> CharacterEncoding {
        self.encoding
    }

    /// Set the character encoding.
    pub fn set_encoding(&mut self, encoding: CharacterEncoding) {
        self.encoding = encoding;
    }

    /// Get the current buffer contents.
    pub fn contents(&self) -> &[u8] {
        &self.buffer
    }

    /// Get the current buffer length.
    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    /// Check if the buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    /// Clear the buffer.
    pub fn clear(&mut self) {
        self.buffer.clear();
        self.pending_echo.clear();
    }

    /// Calculate the number of bytes to delete for a backspace operation.
    ///
    /// This handles multi-byte characters correctly based on the current encoding.
    fn bytes_to_delete(&self) -> usize {
        if self.buffer.is_empty() {
            return 0;
        }

        match self.encoding {
            CharacterEncoding::Utf8 => {
                // UTF-8: scan backwards for continuation bytes (0x80-0xBF)
                let mut len = 0;
                for &byte in self.buffer.iter().rev() {
                    len += 1;
                    if byte & 0xC0 != 0x80 {
                        // Not a continuation byte, this is the start of the character
                        break;
                    }
                }
                len
            }
            CharacterEncoding::ShiftJIS => {
                // ShiftJIS: check if last byte is part of a 2-byte character
                let last = *self.buffer.last().unwrap();

                // Check for 1-byte characters first
                // ASCII (0x00-0x7F) or half-width katakana (0xA1-0xDF)
                if last <= 0x7F || (0xA1..=0xDF).contains(&last) {
                    return 1;
                }

                // Check if this could be the second byte of a 2-byte character
                if self.buffer.len() >= 2 {
                    let prev = self.buffer[self.buffer.len() - 2];
                    // Is prev a lead byte? (0x81-0x9F or 0xE0-0xFC)
                    let is_lead = (0x81..=0x9F).contains(&prev) || (0xE0..=0xFC).contains(&prev);
                    // Is last a valid trail byte? (0x40-0x7E or 0x80-0xFC)
                    let is_trail = (0x40..=0x7E).contains(&last) || (0x80..=0xFC).contains(&last);
                    if is_lead && is_trail {
                        return 2;
                    }
                }

                1
            }
            CharacterEncoding::Cp437 | CharacterEncoding::Petscii => {
                // CP437 and PETSCII are single-byte encodings
                1
            }
        }
    }

    /// Calculate the display width of deleted bytes.
    ///
    /// For multi-byte characters (2+ bytes), assumes 2-column width (full-width).
    /// For single-byte characters, assumes 1-column width.
    fn display_width_of_deleted(&self, bytes_deleted: usize) -> usize {
        if bytes_deleted > 1 {
            2 // Full-width character
        } else {
            1 // Half-width character
        }
    }

    /// Check if pending_echo contains a complete character.
    /// Returns true if the bytes form a complete character that can be echoed.
    fn is_pending_echo_complete(&self) -> bool {
        if self.pending_echo.is_empty() {
            return false;
        }

        match self.encoding {
            CharacterEncoding::Utf8 => {
                let first = self.pending_echo[0];
                let expected_len = if first < 0x80 {
                    1
                } else if first & 0xE0 == 0xC0 {
                    2
                } else if first & 0xF0 == 0xE0 {
                    3
                } else if first & 0xF8 == 0xF0 {
                    4
                } else {
                    // Invalid UTF-8 lead byte, treat as single byte
                    1
                };
                self.pending_echo.len() >= expected_len
            }
            CharacterEncoding::ShiftJIS => {
                let first = self.pending_echo[0];
                // Check if it's a 2-byte character lead byte
                let is_lead = (0x81..=0x9F).contains(&first) || (0xE0..=0xFC).contains(&first);
                if is_lead {
                    self.pending_echo.len() >= 2
                } else {
                    // Single-byte character (ASCII or half-width katakana)
                    true
                }
            }
            CharacterEncoding::Cp437 | CharacterEncoding::Petscii => {
                // CP437 and PETSCII are single-byte encodings
                // Any byte is a complete character
                true
            }
        }
    }

    /// Process a single byte of input.
    ///
    /// Returns the input result and any bytes that should be echoed back.
    pub fn process_byte(&mut self, byte: u8) -> (InputResult, Vec<u8>) {
        match byte {
            control::CR | control::LF => {
                // End of line - flush any pending echo bytes first
                let mut echo = std::mem::take(&mut self.pending_echo);
                echo.push(control::CR);
                echo.push(control::LF);
                let line = self.take_line();
                (InputResult::Line(line), echo)
            }
            control::BS | control::DEL => {
                // Clear any pending echo bytes (incomplete multi-byte char)
                self.pending_echo.clear();

                // Backspace - delete the last character (which may be multi-byte)
                let bytes_to_del = self.bytes_to_delete();
                if bytes_to_del > 0 {
                    // Calculate display width before deleting
                    let width = self.display_width_of_deleted(bytes_to_del);

                    // Delete the bytes
                    let new_len = self.buffer.len() - bytes_to_del;
                    self.buffer.truncate(new_len);

                    // Echo: move cursor back, print spaces, move cursor back
                    // Repeat for each column the character occupied
                    let mut echo = Vec::with_capacity(width * 3);
                    for _ in 0..width {
                        echo.push(control::BS);
                    }
                    for _ in 0..width {
                        echo.push(b' ');
                    }
                    for _ in 0..width {
                        echo.push(control::BS);
                    }
                    (InputResult::Buffering, echo)
                } else {
                    // Buffer was empty, no echo
                    (InputResult::Buffering, vec![])
                }
            }
            control::ETX => {
                // Ctrl+C - cancel
                self.clear();
                (
                    InputResult::Cancel,
                    vec![b'^', b'C', control::CR, control::LF],
                )
            }
            control::EOT => {
                // Ctrl+D - EOF
                (InputResult::Eof, vec![])
            }
            control::ESC => {
                // Start of escape sequence - ignore for now
                // TODO: Handle ANSI escape sequences
                (InputResult::Buffering, vec![])
            }
            control::NUL => {
                // Ignore null bytes (often sent after CR)
                (InputResult::Buffering, vec![])
            }
            _ if byte < 32 => {
                // Other control characters - ignore
                (InputResult::Buffering, vec![])
            }
            _ => {
                // Regular character
                if self.buffer.len() < self.max_size {
                    self.buffer.push(byte);

                    let echo = match self.echo_mode {
                        EchoMode::Normal => {
                            // Add byte to pending echo buffer
                            self.pending_echo.push(byte);

                            // Check if we have a complete character
                            if self.is_pending_echo_complete() {
                                // Return all pending bytes as echo
                                std::mem::take(&mut self.pending_echo)
                            } else {
                                // Still waiting for more bytes
                                vec![]
                            }
                        }
                        EchoMode::Password => vec![],
                        EchoMode::Masked(c) => {
                            // For masked mode, also wait for complete character
                            self.pending_echo.push(byte);
                            if self.is_pending_echo_complete() {
                                self.pending_echo.clear();
                                c.to_string().into_bytes()
                            } else {
                                vec![]
                            }
                        }
                    };
                    (InputResult::Buffering, echo)
                } else {
                    // Buffer full - beep
                    (InputResult::Buffering, vec![0x07]) // BEL
                }
            }
        }
    }

    /// Process multiple bytes of input.
    ///
    /// Returns a list of results with their echo bytes.
    pub fn process_bytes(&mut self, bytes: &[u8]) -> Vec<(InputResult, Vec<u8>)> {
        let mut results = Vec::new();
        for &byte in bytes {
            let result = self.process_byte(byte);
            // Skip Buffering results with no echo for cleaner output
            if result.0 != InputResult::Buffering || !result.1.is_empty() {
                results.push(result);
            }
        }
        results
    }

    /// Take the current buffer contents as a string and clear the buffer.
    ///
    /// Uses the configured encoding to decode the bytes.
    fn take_line(&mut self) -> String {
        let bytes = std::mem::take(&mut self.buffer);
        decode_from_client(&bytes, self.encoding)
    }
}

impl Default for LineBuffer {
    fn default() -> Self {
        Self::with_defaults()
    }
}

/// A more advanced input handler that handles multi-line input.
#[derive(Debug)]
pub struct MultiLineBuffer {
    /// Individual lines collected.
    lines: Vec<String>,
    /// Current line buffer.
    current: LineBuffer,
    /// Terminator pattern (e.g., "." for single period on a line).
    terminator: String,
}

impl MultiLineBuffer {
    /// Create a new multi-line buffer with the given terminator.
    pub fn new(terminator: impl Into<String>) -> Self {
        Self {
            lines: Vec::new(),
            current: LineBuffer::with_defaults(),
            terminator: terminator.into(),
        }
    }

    /// Process a single byte.
    ///
    /// Returns Some(collected lines) when the terminator is reached.
    pub fn process_byte(&mut self, byte: u8) -> (Option<Vec<String>>, Vec<u8>) {
        let (result, echo) = self.current.process_byte(byte);
        match result {
            InputResult::Line(line) => {
                if line == self.terminator {
                    // Terminator reached - return all collected lines
                    let result = std::mem::take(&mut self.lines);
                    (Some(result), echo)
                } else {
                    self.lines.push(line);
                    (None, echo)
                }
            }
            InputResult::Cancel => {
                // Cancel - clear everything
                self.lines.clear();
                self.current.clear();
                (Some(vec![]), echo) // Return empty to indicate cancellation
            }
            _ => (None, echo),
        }
    }

    /// Get the number of lines collected so far.
    pub fn line_count(&self) -> usize {
        self.lines.len()
    }

    /// Clear all collected data.
    pub fn clear(&mut self) {
        self.lines.clear();
        self.current.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_line_buffer_new() {
        let buffer = LineBuffer::new(100);
        assert!(buffer.is_empty());
        assert_eq!(buffer.len(), 0);
        assert_eq!(buffer.echo_mode(), EchoMode::Normal);
    }

    #[test]
    fn test_line_buffer_process_regular_chars() {
        let mut buffer = LineBuffer::new(100);

        let (result, echo) = buffer.process_byte(b'H');
        assert_eq!(result, InputResult::Buffering);
        assert_eq!(echo, vec![b'H']);

        let (result, echo) = buffer.process_byte(b'i');
        assert_eq!(result, InputResult::Buffering);
        assert_eq!(echo, vec![b'i']);

        assert_eq!(buffer.contents(), b"Hi");
    }

    #[test]
    fn test_line_buffer_process_enter() {
        let mut buffer = LineBuffer::new(100);

        buffer.process_byte(b'H');
        buffer.process_byte(b'i');

        let (result, echo) = buffer.process_byte(control::CR);
        assert_eq!(result, InputResult::Line("Hi".to_string()));
        assert_eq!(echo, vec![control::CR, control::LF]);
        assert!(buffer.is_empty());
    }

    #[test]
    fn test_line_buffer_process_lf() {
        let mut buffer = LineBuffer::new(100);

        buffer.process_byte(b'H');
        buffer.process_byte(b'i');

        let (result, _) = buffer.process_byte(control::LF);
        assert_eq!(result, InputResult::Line("Hi".to_string()));
    }

    #[test]
    fn test_line_buffer_backspace() {
        let mut buffer = LineBuffer::new(100);

        buffer.process_byte(b'H');
        buffer.process_byte(b'i');
        buffer.process_byte(b'!');

        let (result, echo) = buffer.process_byte(control::BS);
        assert_eq!(result, InputResult::Buffering);
        assert_eq!(echo, vec![control::BS, b' ', control::BS]);
        assert_eq!(buffer.contents(), b"Hi");
    }

    #[test]
    fn test_line_buffer_del() {
        let mut buffer = LineBuffer::new(100);

        buffer.process_byte(b'H');
        buffer.process_byte(b'i');

        let (result, echo) = buffer.process_byte(control::DEL);
        assert_eq!(result, InputResult::Buffering);
        assert_eq!(echo, vec![control::BS, b' ', control::BS]);
        assert_eq!(buffer.contents(), b"H");
    }

    #[test]
    fn test_line_buffer_backspace_empty() {
        let mut buffer = LineBuffer::new(100);

        let (result, echo) = buffer.process_byte(control::BS);
        assert_eq!(result, InputResult::Buffering);
        assert!(echo.is_empty());
    }

    #[test]
    fn test_line_buffer_ctrl_c() {
        let mut buffer = LineBuffer::new(100);

        buffer.process_byte(b'H');
        buffer.process_byte(b'i');

        let (result, echo) = buffer.process_byte(control::ETX);
        assert_eq!(result, InputResult::Cancel);
        assert_eq!(echo, vec![b'^', b'C', control::CR, control::LF]);
        assert!(buffer.is_empty());
    }

    #[test]
    fn test_line_buffer_ctrl_d() {
        let mut buffer = LineBuffer::new(100);

        let (result, echo) = buffer.process_byte(control::EOT);
        assert_eq!(result, InputResult::Eof);
        assert!(echo.is_empty());
    }

    #[test]
    fn test_line_buffer_password_mode() {
        let mut buffer = LineBuffer::new(100);
        buffer.set_echo_mode(EchoMode::Password);

        let (result, echo) = buffer.process_byte(b'p');
        assert_eq!(result, InputResult::Buffering);
        assert!(echo.is_empty()); // No echo in password mode

        let (result, echo) = buffer.process_byte(b'a');
        assert_eq!(result, InputResult::Buffering);
        assert!(echo.is_empty());

        assert_eq!(buffer.contents(), b"pa");
    }

    #[test]
    fn test_line_buffer_masked_mode() {
        let mut buffer = LineBuffer::new(100);
        buffer.set_echo_mode(EchoMode::Masked('*'));

        let (result, echo) = buffer.process_byte(b'p');
        assert_eq!(result, InputResult::Buffering);
        assert_eq!(echo, b"*");

        let (result, echo) = buffer.process_byte(b'a');
        assert_eq!(result, InputResult::Buffering);
        assert_eq!(echo, b"*");

        assert_eq!(buffer.contents(), b"pa");
    }

    #[test]
    fn test_line_buffer_max_size() {
        let mut buffer = LineBuffer::new(3);

        buffer.process_byte(b'a');
        buffer.process_byte(b'b');
        buffer.process_byte(b'c');

        // Buffer is full
        let (result, echo) = buffer.process_byte(b'd');
        assert_eq!(result, InputResult::Buffering);
        assert_eq!(echo, vec![0x07]); // BEL
        assert_eq!(buffer.contents(), b"abc"); // d was not added
    }

    #[test]
    fn test_line_buffer_process_bytes() {
        let mut buffer = LineBuffer::new(100);

        let results = buffer.process_bytes(b"Hi\r");

        // Should have 3 results: 'H' echo, 'i' echo, line complete
        assert_eq!(results.len(), 3);
        assert_eq!(results[0], (InputResult::Buffering, vec![b'H']));
        assert_eq!(results[1], (InputResult::Buffering, vec![b'i']));
        assert_eq!(
            results[2],
            (
                InputResult::Line("Hi".to_string()),
                vec![control::CR, control::LF]
            )
        );
    }

    #[test]
    fn test_line_buffer_ignore_null() {
        let mut buffer = LineBuffer::new(100);

        buffer.process_byte(b'H');
        let (result, echo) = buffer.process_byte(control::NUL);
        assert_eq!(result, InputResult::Buffering);
        assert!(echo.is_empty());
        assert_eq!(buffer.contents(), b"H");
    }

    #[test]
    fn test_multi_line_buffer() {
        let mut buffer = MultiLineBuffer::new(".");

        // First line
        buffer.process_byte(b'H');
        buffer.process_byte(b'i');
        let (result, _) = buffer.process_byte(control::CR);
        assert!(result.is_none());
        assert_eq!(buffer.line_count(), 1);

        // Second line
        buffer.process_byte(b'W');
        buffer.process_byte(b'o');
        buffer.process_byte(b'r');
        buffer.process_byte(b'l');
        buffer.process_byte(b'd');
        let (result, _) = buffer.process_byte(control::CR);
        assert!(result.is_none());
        assert_eq!(buffer.line_count(), 2);

        // Terminator
        buffer.process_byte(b'.');
        let (result, _) = buffer.process_byte(control::CR);
        assert!(result.is_some());
        let lines = result.unwrap();
        assert_eq!(lines, vec!["Hi", "World"]);
    }

    #[test]
    fn test_multi_line_buffer_cancel() {
        let mut buffer = MultiLineBuffer::new(".");

        buffer.process_byte(b'H');
        buffer.process_byte(b'i');
        buffer.process_byte(control::CR);
        assert_eq!(buffer.line_count(), 1);

        // Cancel
        let (result, _) = buffer.process_byte(control::ETX);
        assert!(result.is_some());
        let lines = result.unwrap();
        assert!(lines.is_empty()); // Cancelled
        assert_eq!(buffer.line_count(), 0);
    }

    #[test]
    fn test_line_buffer_clear() {
        let mut buffer = LineBuffer::new(100);
        buffer.process_byte(b'H');
        buffer.process_byte(b'i');
        assert_eq!(buffer.len(), 2);

        buffer.clear();
        assert!(buffer.is_empty());
    }

    #[test]
    fn test_line_buffer_encoding_default() {
        let buffer = LineBuffer::new(100);
        assert_eq!(buffer.encoding(), CharacterEncoding::ShiftJIS);
    }

    #[test]
    fn test_line_buffer_with_encoding() {
        let buffer = LineBuffer::with_encoding(100, CharacterEncoding::Utf8);
        assert_eq!(buffer.encoding(), CharacterEncoding::Utf8);
        assert_eq!(buffer.echo_mode(), EchoMode::Normal);
    }

    #[test]
    fn test_line_buffer_set_encoding() {
        let mut buffer = LineBuffer::new(100);
        assert_eq!(buffer.encoding(), CharacterEncoding::ShiftJIS);

        buffer.set_encoding(CharacterEncoding::Utf8);
        assert_eq!(buffer.encoding(), CharacterEncoding::Utf8);
    }

    #[test]
    fn test_line_buffer_decode_utf8() {
        let mut buffer = LineBuffer::with_encoding(100, CharacterEncoding::Utf8);

        // UTF-8 encoded "こんにちは" (hello in Japanese)
        let bytes: &[u8] = "こんにちは".as_bytes();
        for &b in bytes {
            buffer.process_byte(b);
        }

        let (result, _) = buffer.process_byte(control::CR);
        match result {
            InputResult::Line(line) => assert_eq!(line, "こんにちは"),
            _ => panic!("Expected Line result"),
        }
    }

    #[test]
    fn test_line_buffer_decode_shiftjis() {
        let mut buffer = LineBuffer::with_encoding(100, CharacterEncoding::ShiftJIS);

        // ShiftJIS encoded "こんにちは"
        // こ=0x82 0xB1, ん=0x82 0xF1, に=0x82 0xC9, ち=0x82 0xBF, は=0x82 0xCD
        let shiftjis_bytes: &[u8] = &[
            0x82, 0xB1, // こ
            0x82, 0xF1, // ん
            0x82, 0xC9, // に
            0x82, 0xBF, // ち
            0x82, 0xCD, // は
        ];
        for &b in shiftjis_bytes {
            buffer.process_byte(b);
        }

        let (result, _) = buffer.process_byte(control::CR);
        match result {
            InputResult::Line(line) => assert_eq!(line, "こんにちは"),
            _ => panic!("Expected Line result"),
        }
    }

    #[test]
    fn test_line_buffer_decode_ascii_same_for_both() {
        // ASCII should decode the same for both encodings
        let mut buffer_shiftjis = LineBuffer::with_encoding(100, CharacterEncoding::ShiftJIS);
        let mut buffer_utf8 = LineBuffer::with_encoding(100, CharacterEncoding::Utf8);

        for &b in b"Hello" {
            buffer_shiftjis.process_byte(b);
            buffer_utf8.process_byte(b);
        }

        let (result_sj, _) = buffer_shiftjis.process_byte(control::CR);
        let (result_utf8, _) = buffer_utf8.process_byte(control::CR);

        match (result_sj, result_utf8) {
            (InputResult::Line(line_sj), InputResult::Line(line_utf8)) => {
                assert_eq!(line_sj, "Hello");
                assert_eq!(line_utf8, "Hello");
                assert_eq!(line_sj, line_utf8);
            }
            _ => panic!("Expected Line results"),
        }
    }

    #[test]
    fn test_line_buffer_backspace_utf8_multibyte() {
        // Test backspace with UTF-8 multi-byte character "あ" (3 bytes: 0xE3 0x81 0x82)
        let mut buffer = LineBuffer::with_encoding(100, CharacterEncoding::Utf8);

        // Input "Aあ" (1 ASCII + 3 UTF-8 bytes)
        buffer.process_byte(b'A');
        for &b in "あ".as_bytes() {
            buffer.process_byte(b);
        }
        assert_eq!(buffer.len(), 4); // 1 + 3 bytes

        // Backspace should delete the entire "あ" character (3 bytes)
        let (result, echo) = buffer.process_byte(control::BS);
        assert_eq!(result, InputResult::Buffering);
        // Echo should be 2-column width (full-width character)
        assert_eq!(
            echo,
            vec![
                control::BS,
                control::BS,
                b' ',
                b' ',
                control::BS,
                control::BS
            ]
        );
        assert_eq!(buffer.contents(), b"A");
        assert_eq!(buffer.len(), 1);
    }

    #[test]
    fn test_line_buffer_backspace_utf8_4byte() {
        // Test backspace with UTF-8 4-byte character "𠀋" (U+2000B)
        let mut buffer = LineBuffer::with_encoding(100, CharacterEncoding::Utf8);

        // Input the 4-byte character
        let char_bytes = "𠀋".as_bytes();
        assert_eq!(char_bytes.len(), 4);
        for &b in char_bytes {
            buffer.process_byte(b);
        }
        assert_eq!(buffer.len(), 4);

        // Backspace should delete all 4 bytes
        let (result, _) = buffer.process_byte(control::BS);
        assert_eq!(result, InputResult::Buffering);
        assert!(buffer.is_empty());
    }

    #[test]
    fn test_line_buffer_backspace_shiftjis_multibyte() {
        // Test backspace with ShiftJIS 2-byte character "あ" (0x82 0xA0)
        let mut buffer = LineBuffer::with_encoding(100, CharacterEncoding::ShiftJIS);

        // Input "Aあ" in ShiftJIS
        buffer.process_byte(b'A');
        buffer.process_byte(0x82); // Lead byte of "あ"
        buffer.process_byte(0xA0); // Trail byte of "あ"
        assert_eq!(buffer.len(), 3); // 1 + 2 bytes

        // Backspace should delete the entire "あ" character (2 bytes)
        let (result, echo) = buffer.process_byte(control::BS);
        assert_eq!(result, InputResult::Buffering);
        // Echo should be 2-column width
        assert_eq!(
            echo,
            vec![
                control::BS,
                control::BS,
                b' ',
                b' ',
                control::BS,
                control::BS
            ]
        );
        assert_eq!(buffer.contents(), b"A");
        assert_eq!(buffer.len(), 1);
    }

    #[test]
    fn test_line_buffer_backspace_shiftjis_halfwidth_katakana() {
        // Test backspace with ShiftJIS half-width katakana "ｱ" (0xB1, single byte)
        let mut buffer = LineBuffer::with_encoding(100, CharacterEncoding::ShiftJIS);

        buffer.process_byte(b'A');
        buffer.process_byte(0xB1); // Half-width "ｱ"
        assert_eq!(buffer.len(), 2);

        // Backspace should delete only 1 byte (half-width character)
        let (result, echo) = buffer.process_byte(control::BS);
        assert_eq!(result, InputResult::Buffering);
        // Echo should be 1-column width
        assert_eq!(echo, vec![control::BS, b' ', control::BS]);
        assert_eq!(buffer.contents(), b"A");
    }

    #[test]
    fn test_line_buffer_backspace_mixed_ascii_and_multibyte() {
        // Test multiple backspaces with mixed ASCII and multi-byte characters
        let mut buffer = LineBuffer::with_encoding(100, CharacterEncoding::Utf8);

        // Input "ABあい" (2 ASCII + 2 Japanese characters)
        buffer.process_byte(b'A');
        buffer.process_byte(b'B');
        for &b in "あ".as_bytes() {
            buffer.process_byte(b);
        }
        for &b in "い".as_bytes() {
            buffer.process_byte(b);
        }
        assert_eq!(buffer.len(), 8); // 2 + 3 + 3 bytes

        // First backspace: delete "い" (3 bytes)
        buffer.process_byte(control::BS);
        assert_eq!(buffer.len(), 5); // 2 + 3 bytes

        // Second backspace: delete "あ" (3 bytes)
        buffer.process_byte(control::BS);
        assert_eq!(buffer.len(), 2); // Just "AB"

        // Third backspace: delete "B" (1 byte)
        let (_, echo) = buffer.process_byte(control::BS);
        assert_eq!(buffer.len(), 1);
        // ASCII should have 1-column echo
        assert_eq!(echo, vec![control::BS, b' ', control::BS]);
        assert_eq!(buffer.contents(), b"A");
    }

    #[test]
    fn test_bytes_to_delete_utf8() {
        let mut buffer = LineBuffer::with_encoding(100, CharacterEncoding::Utf8);

        // Empty buffer
        assert_eq!(buffer.bytes_to_delete(), 0);

        // ASCII character
        buffer.process_byte(b'A');
        assert_eq!(buffer.bytes_to_delete(), 1);

        buffer.clear();

        // 2-byte UTF-8 character (e.g., "é" = 0xC3 0xA9)
        buffer.process_byte(0xC3);
        buffer.process_byte(0xA9);
        assert_eq!(buffer.bytes_to_delete(), 2);

        buffer.clear();

        // 3-byte UTF-8 character (e.g., "あ" = 0xE3 0x81 0x82)
        buffer.process_byte(0xE3);
        buffer.process_byte(0x81);
        buffer.process_byte(0x82);
        assert_eq!(buffer.bytes_to_delete(), 3);
    }

    #[test]
    fn test_bytes_to_delete_shiftjis() {
        let mut buffer = LineBuffer::with_encoding(100, CharacterEncoding::ShiftJIS);

        // Empty buffer
        assert_eq!(buffer.bytes_to_delete(), 0);

        // ASCII character
        buffer.process_byte(b'A');
        assert_eq!(buffer.bytes_to_delete(), 1);

        buffer.clear();

        // Half-width katakana (single byte)
        buffer.process_byte(0xB1); // "ｱ"
        assert_eq!(buffer.bytes_to_delete(), 1);

        buffer.clear();

        // Full-width character "あ" (0x82 0xA0)
        buffer.process_byte(0x82);
        buffer.process_byte(0xA0);
        assert_eq!(buffer.bytes_to_delete(), 2);
    }
}
