//! Input handling for Telnet sessions.
//!
//! This module provides line buffering, special key handling, and
//! input processing for Telnet connections.

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
}

impl LineBuffer {
    /// Create a new line buffer with the given maximum size.
    pub fn new(max_size: usize) -> Self {
        Self {
            buffer: Vec::with_capacity(max_size.min(1024)),
            max_size,
            echo_mode: EchoMode::Normal,
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
    }

    /// Process a single byte of input.
    ///
    /// Returns the input result and any bytes that should be echoed back.
    pub fn process_byte(&mut self, byte: u8) -> (InputResult, Vec<u8>) {
        match byte {
            control::CR | control::LF => {
                // End of line
                let line = self.take_line();
                let echo = vec![control::CR, control::LF];
                (InputResult::Line(line), echo)
            }
            control::BS | control::DEL => {
                // Backspace
                if self.buffer.pop().is_some() {
                    // Echo: move cursor back, print space, move cursor back
                    let echo = vec![control::BS, b' ', control::BS];
                    (InputResult::Buffering, echo)
                } else {
                    // Buffer was empty, no echo
                    (InputResult::Buffering, vec![])
                }
            }
            control::ETX => {
                // Ctrl+C - cancel
                self.clear();
                (InputResult::Cancel, vec![b'^', b'C', control::CR, control::LF])
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
                        EchoMode::Normal => vec![byte],
                        EchoMode::Password => vec![],
                        EchoMode::Masked(c) => c.to_string().into_bytes(),
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
    fn take_line(&mut self) -> String {
        let bytes = std::mem::take(&mut self.buffer);
        String::from_utf8_lossy(&bytes).into_owned()
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
            (InputResult::Line("Hi".to_string()), vec![control::CR, control::LF])
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
}
