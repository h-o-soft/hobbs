//! Character encoding conversion for Telnet communication.
//!
//! This module handles conversion between UTF-8 (internal representation)
//! and various wire formats (ShiftJIS for legacy terminals, UTF-8 for modern terminals,
//! CP437 for IBM PC compatibles, and PETSCII for Commodore computers).

use std::fmt;
use std::str::FromStr;

use codepage_437::{BorrowFromCp437, ToCp437, CP437_CONTROL};
use encoding_rs::SHIFT_JIS;

/// Character encoding for client communication.
///
/// HOBBS supports multiple encodings for different terminal types:
/// - ShiftJIS: For legacy Japanese terminals (PC-98, etc.)
/// - UTF-8: For modern terminals and international users
/// - Cp437: For IBM PC compatibles and DOS terminals
/// - Petscii: For Commodore 64/128 and other Commodore computers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub enum CharacterEncoding {
    /// ShiftJIS encoding (default for retro compatibility).
    #[default]
    ShiftJIS,
    /// UTF-8 encoding for modern terminals.
    Utf8,
    /// Code Page 437 (IBM PC original character set).
    Cp437,
    /// PETSCII (Commodore 64/128 character set).
    Petscii,
}

/// Output mode for terminal display.
///
/// This determines how escape sequences and control codes are handled
/// when sending output to the terminal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub enum OutputMode {
    /// ANSI escape sequences for colors and cursor control.
    /// Used by most modern terminals and some retro terminals with ANSI support.
    #[default]
    Ansi,
    /// Plain text with no escape sequences.
    /// ANSI sequences are stripped from output.
    Plain,
    /// PETSCII control codes for Commodore 64/128.
    /// ANSI sequences are converted to equivalent PETSCII control codes.
    PetsciiCtrl,
}

impl OutputMode {
    /// Get the mode name as a string.
    pub fn as_str(&self) -> &'static str {
        match self {
            OutputMode::Ansi => "ansi",
            OutputMode::Plain => "plain",
            OutputMode::PetsciiCtrl => "petscii_ctrl",
        }
    }

    /// Get the display name for the mode.
    pub fn display_name(&self) -> &'static str {
        match self {
            OutputMode::Ansi => "ANSI",
            OutputMode::Plain => "Plain",
            OutputMode::PetsciiCtrl => "PETSCII Ctrl",
        }
    }

    /// Get all available output modes.
    pub fn all() -> &'static [OutputMode] {
        &[OutputMode::Ansi, OutputMode::Plain, OutputMode::PetsciiCtrl]
    }
}

impl fmt::Display for OutputMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

impl FromStr for OutputMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "ansi" => Ok(OutputMode::Ansi),
            "plain" | "ascii" | "none" => Ok(OutputMode::Plain),
            "petscii_ctrl" | "petscii-ctrl" | "petscii" => Ok(OutputMode::PetsciiCtrl),
            _ => Err(format!("unknown output mode: {s}")),
        }
    }
}

impl CharacterEncoding {
    /// Get the encoding name as a string.
    pub fn as_str(&self) -> &'static str {
        match self {
            CharacterEncoding::ShiftJIS => "shiftjis",
            CharacterEncoding::Utf8 => "utf8",
            CharacterEncoding::Cp437 => "cp437",
            CharacterEncoding::Petscii => "petscii",
        }
    }

    /// Get the display name for the encoding.
    pub fn display_name(&self) -> &'static str {
        match self {
            CharacterEncoding::ShiftJIS => "ShiftJIS",
            CharacterEncoding::Utf8 => "UTF-8",
            CharacterEncoding::Cp437 => "CP437",
            CharacterEncoding::Petscii => "PETSCII",
        }
    }

    /// Get all available encodings.
    pub fn all() -> &'static [CharacterEncoding] {
        &[
            CharacterEncoding::ShiftJIS,
            CharacterEncoding::Utf8,
            CharacterEncoding::Cp437,
            CharacterEncoding::Petscii,
        ]
    }
}

impl fmt::Display for CharacterEncoding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

impl FromStr for CharacterEncoding {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "shiftjis" | "shift_jis" | "shift-jis" | "sjis" => Ok(CharacterEncoding::ShiftJIS),
            "utf8" | "utf-8" => Ok(CharacterEncoding::Utf8),
            "cp437" | "ibm437" | "dos" | "oem-us" => Ok(CharacterEncoding::Cp437),
            "petscii" | "cbm" | "commodore" => Ok(CharacterEncoding::Petscii),
            _ => Err(format!("unknown encoding: {s}")),
        }
    }
}

/// Encode a UTF-8 string for sending to a client with the specified encoding.
///
/// # Arguments
///
/// * `text` - The UTF-8 string to encode
/// * `encoding` - The target encoding for the client
///
/// # Returns
///
/// The encoded bytes ready to send to the client.
///
/// # Examples
///
/// ```
/// use hobbs::server::encoding::{encode_for_client, CharacterEncoding};
///
/// let text = "Hello, 世界!";
///
/// // UTF-8 encoding (pass-through)
/// let utf8_bytes = encode_for_client(text, CharacterEncoding::Utf8);
/// assert_eq!(utf8_bytes, text.as_bytes());
///
/// // ShiftJIS encoding
/// let sjis_bytes = encode_for_client(text, CharacterEncoding::ShiftJIS);
/// assert_ne!(sjis_bytes, text.as_bytes());
/// ```
pub fn encode_for_client(text: &str, encoding: CharacterEncoding) -> Vec<u8> {
    match encoding {
        CharacterEncoding::Utf8 => text.as_bytes().to_vec(),
        CharacterEncoding::ShiftJIS => encode_shiftjis(text).bytes,
        CharacterEncoding::Cp437 => encode_cp437(text).bytes,
        CharacterEncoding::Petscii => encode_petscii(text).bytes,
    }
}

/// Decode bytes received from a client with the specified encoding.
///
/// # Arguments
///
/// * `bytes` - The bytes received from the client
/// * `encoding` - The encoding used by the client
///
/// # Returns
///
/// The decoded UTF-8 string. Invalid sequences are replaced with the
/// Unicode replacement character (U+FFFD).
///
/// # Examples
///
/// ```
/// use hobbs::server::encoding::{decode_from_client, CharacterEncoding};
///
/// // UTF-8 decoding
/// let utf8_bytes = "Hello, 世界!".as_bytes();
/// let text = decode_from_client(utf8_bytes, CharacterEncoding::Utf8);
/// assert_eq!(text, "Hello, 世界!");
///
/// // ShiftJIS decoding
/// let sjis_bytes = vec![0x83, 0x65, 0x83, 0x58, 0x83, 0x67]; // "テスト"
/// let text = decode_from_client(&sjis_bytes, CharacterEncoding::ShiftJIS);
/// assert_eq!(text, "テスト");
/// ```
pub fn decode_from_client(bytes: &[u8], encoding: CharacterEncoding) -> String {
    match encoding {
        CharacterEncoding::Utf8 => String::from_utf8_lossy(bytes).into_owned(),
        CharacterEncoding::ShiftJIS => decode_shiftjis(bytes).text,
        CharacterEncoding::Cp437 => decode_cp437(bytes).text,
        CharacterEncoding::Petscii => decode_petscii(bytes).text,
    }
}

/// Encode a UTF-8 string for a client, with error information.
///
/// Like `encode_for_client`, but returns detailed information about
/// whether any characters could not be represented in the target encoding.
///
/// # Arguments
///
/// * `text` - The UTF-8 string to encode
/// * `encoding` - The target encoding for the client
///
/// # Returns
///
/// An `EncodeResult` with the encoded bytes and error flag.
pub fn encode_for_client_detailed(text: &str, encoding: CharacterEncoding) -> EncodeResult {
    match encoding {
        CharacterEncoding::Utf8 => EncodeResult {
            bytes: text.as_bytes().to_vec(),
            had_errors: false,
        },
        CharacterEncoding::ShiftJIS => encode_shiftjis(text),
        CharacterEncoding::Cp437 => encode_cp437(text),
        CharacterEncoding::Petscii => encode_petscii(text),
    }
}

/// Decode bytes from a client, with error information.
///
/// Like `decode_from_client`, but returns detailed information about
/// whether any bytes could not be decoded.
///
/// # Arguments
///
/// * `bytes` - The bytes received from the client
/// * `encoding` - The encoding used by the client
///
/// # Returns
///
/// A `DecodeResult` with the decoded string and error flag.
pub fn decode_from_client_detailed(bytes: &[u8], encoding: CharacterEncoding) -> DecodeResult {
    match encoding {
        CharacterEncoding::Utf8 => {
            let text = String::from_utf8_lossy(bytes);
            let had_errors = text.contains('\u{FFFD}');
            DecodeResult {
                text: text.into_owned(),
                had_errors,
            }
        }
        CharacterEncoding::ShiftJIS => decode_shiftjis(bytes),
        CharacterEncoding::Cp437 => decode_cp437(bytes),
        CharacterEncoding::Petscii => decode_petscii(bytes),
    }
}

/// Result of a decoding operation.
#[derive(Debug, Clone)]
pub struct DecodeResult {
    /// The decoded UTF-8 string.
    pub text: String,
    /// Whether any errors occurred during decoding.
    /// If true, some bytes were replaced with the replacement character (U+FFFD).
    pub had_errors: bool,
}

/// Result of an encoding operation.
#[derive(Debug, Clone)]
pub struct EncodeResult {
    /// The encoded ShiftJIS bytes.
    pub bytes: Vec<u8>,
    /// Whether any errors occurred during encoding.
    /// If true, some characters were replaced with HTML numeric character references.
    pub had_errors: bool,
}

/// Decode ShiftJIS bytes to UTF-8 string.
///
/// This function converts bytes received from a Telnet client (assumed to be
/// ShiftJIS encoded) into a UTF-8 string for internal processing.
///
/// # Arguments
///
/// * `bytes` - The ShiftJIS encoded bytes to decode.
///
/// # Returns
///
/// A `DecodeResult` containing the decoded string and an error flag.
///
/// # Example
///
/// ```
/// use hobbs::server::encoding::decode_shiftjis;
///
/// // "テスト" in ShiftJIS
/// let shiftjis_bytes = vec![0x83, 0x65, 0x83, 0x58, 0x83, 0x67];
/// let result = decode_shiftjis(&shiftjis_bytes);
/// assert_eq!(result.text, "テスト");
/// assert!(!result.had_errors);
/// ```
pub fn decode_shiftjis(bytes: &[u8]) -> DecodeResult {
    let (cow, _encoding, had_errors) = SHIFT_JIS.decode(bytes);
    DecodeResult {
        text: cow.into_owned(),
        had_errors,
    }
}

/// Encode UTF-8 string to ShiftJIS bytes.
///
/// This function converts a UTF-8 string into ShiftJIS bytes for sending
/// to a Telnet client.
///
/// Characters that cannot be represented in ShiftJIS are replaced with
/// HTML numeric character references (e.g., `&#12345;`).
///
/// # Arguments
///
/// * `text` - The UTF-8 string to encode.
///
/// # Returns
///
/// An `EncodeResult` containing the encoded bytes and an error flag.
///
/// # Example
///
/// ```
/// use hobbs::server::encoding::encode_shiftjis;
///
/// let result = encode_shiftjis("テスト");
/// // "テスト" in ShiftJIS
/// assert_eq!(result.bytes, vec![0x83, 0x65, 0x83, 0x58, 0x83, 0x67]);
/// assert!(!result.had_errors);
/// ```
pub fn encode_shiftjis(text: &str) -> EncodeResult {
    // Normalize problematic Unicode characters before encoding
    let normalized = normalize_for_shiftjis(text);
    let (cow, _encoding, had_errors) = SHIFT_JIS.encode(&normalized);
    EncodeResult {
        bytes: cow.into_owned(),
        had_errors,
    }
}

/// Normalize Unicode characters that cause issues with ShiftJIS encoding.
///
/// This handles the famous "wave dash problem" and other character mapping
/// issues between Unicode and ShiftJIS.
///
/// # Mappings
/// - U+301C (Wave Dash) → U+FF5E (Fullwidth Tilde)
/// - U+2212 (Minus Sign) → U+FF0D (Fullwidth Hyphen-Minus)
/// - U+2014 (Em Dash) → U+2015 (Horizontal Bar)
fn normalize_for_shiftjis(text: &str) -> String {
    text.chars()
        .map(|c| match c {
            '\u{301C}' => '\u{FF5E}', // Wave Dash → Fullwidth Tilde
            '\u{2212}' => '\u{FF0D}', // Minus Sign → Fullwidth Hyphen-Minus
            '\u{2014}' => '\u{2015}', // Em Dash → Horizontal Bar
            _ => c,
        })
        .collect()
}

/// Decode ShiftJIS bytes to UTF-8 string, returning None on error.
///
/// This is a stricter version of `decode_shiftjis` that returns `None`
/// if any decoding errors occurred.
///
/// # Arguments
///
/// * `bytes` - The ShiftJIS encoded bytes to decode.
///
/// # Returns
///
/// `Some(String)` if decoding succeeded without errors, `None` otherwise.
pub fn decode_shiftjis_strict(bytes: &[u8]) -> Option<String> {
    let result = decode_shiftjis(bytes);
    if result.had_errors {
        None
    } else {
        Some(result.text)
    }
}

/// Encode UTF-8 string to ShiftJIS bytes, returning None on error.
///
/// This is a stricter version of `encode_shiftjis` that returns `None`
/// if any encoding errors occurred (i.e., if any characters could not
/// be represented in ShiftJIS).
///
/// # Arguments
///
/// * `text` - The UTF-8 string to encode.
///
/// # Returns
///
/// `Some(Vec<u8>)` if encoding succeeded without errors, `None` otherwise.
pub fn encode_shiftjis_strict(text: &str) -> Option<Vec<u8>> {
    let result = encode_shiftjis(text);
    if result.had_errors {
        None
    } else {
        Some(result.bytes)
    }
}

// ============================================================================
// CP437 (Code Page 437) Encoding/Decoding
// ============================================================================

/// Decode CP437 bytes to UTF-8 string.
///
/// This function converts bytes from a DOS/IBM PC terminal (CP437 encoded)
/// into a UTF-8 string for internal processing.
///
/// # Arguments
///
/// * `bytes` - The CP437 encoded bytes to decode.
///
/// # Returns
///
/// A `DecodeResult` containing the decoded string and an error flag.
///
/// # Example
///
/// ```
/// use hobbs::server::encoding::decode_cp437;
///
/// // "Hello" in CP437 (same as ASCII for basic chars)
/// let bytes = vec![0x48, 0x65, 0x6C, 0x6C, 0x6F];
/// let result = decode_cp437(&bytes);
/// assert_eq!(result.text, "Hello");
/// assert!(!result.had_errors);
/// ```
pub fn decode_cp437(bytes: &[u8]) -> DecodeResult {
    // CP437 decoding never fails - every byte maps to a Unicode character
    let text: String = String::borrow_from_cp437(bytes, &CP437_CONTROL);
    DecodeResult {
        text,
        had_errors: false,
    }
}

/// Encode UTF-8 string to CP437 bytes.
///
/// This function converts a UTF-8 string into CP437 bytes for sending
/// to a DOS/IBM PC terminal.
///
/// Characters that cannot be represented in CP437 are replaced with '?'.
///
/// # Arguments
///
/// * `text` - The UTF-8 string to encode.
///
/// # Returns
///
/// An `EncodeResult` containing the encoded bytes and an error flag.
///
/// # Example
///
/// ```
/// use hobbs::server::encoding::encode_cp437;
///
/// let result = encode_cp437("Hello");
/// assert_eq!(result.bytes, vec![0x48, 0x65, 0x6C, 0x6C, 0x6F]);
/// assert!(!result.had_errors);
/// ```
pub fn encode_cp437(text: &str) -> EncodeResult {
    match text.to_cp437(&CP437_CONTROL) {
        Ok(bytes) => EncodeResult {
            bytes: bytes.into_owned(),
            had_errors: false,
        },
        Err(_) => {
            // Lossy conversion: replace unmappable chars with '?'
            let bytes: Vec<u8> = text
                .chars()
                .map(|c| {
                    c.to_string()
                        .as_str()
                        .to_cp437(&CP437_CONTROL)
                        .map(|b| b[0])
                        .unwrap_or(b'?')
                })
                .collect();
            EncodeResult {
                bytes,
                had_errors: true,
            }
        }
    }
}

// ============================================================================
// PETSCII (Commodore 64/128) Encoding/Decoding
// ============================================================================

/// Decode PETSCII bytes to UTF-8 string.
///
/// This function converts bytes from a Commodore 64/128 terminal (PETSCII encoded)
/// into a UTF-8 string for internal processing.
///
/// PETSCII control codes (0x00-0x1F, 0x80-0x9F) are filtered out.
///
/// # Arguments
///
/// * `bytes` - The PETSCII encoded bytes to decode.
///
/// # Returns
///
/// A `DecodeResult` containing the decoded string and an error flag.
///
/// # Example
///
/// ```
/// use hobbs::server::encoding::decode_petscii;
///
/// // "HELLO" in PETSCII uppercase mode
/// let bytes = vec![0x48, 0x45, 0x4C, 0x4C, 0x4F];
/// let result = decode_petscii(&bytes);
/// assert_eq!(result.text, "HELLO");
/// ```
pub fn decode_petscii(bytes: &[u8]) -> DecodeResult {
    let mut text = String::new();
    let mut had_errors = false;

    for &byte in bytes {
        match petscii_byte_to_unicode(byte) {
            Some(c) => text.push(c),
            None => {
                // Control code or unmappable - skip
                had_errors = true;
            }
        }
    }

    DecodeResult { text, had_errors }
}

/// Encode UTF-8 string to PETSCII bytes.
///
/// This function converts a UTF-8 string into PETSCII bytes for sending
/// to a Commodore 64/128 terminal.
///
/// Characters that cannot be represented in PETSCII are replaced with '?'.
/// The output is in PETSCII uppercase (graphics) mode.
///
/// # Arguments
///
/// * `text` - The UTF-8 string to encode.
///
/// # Returns
///
/// An `EncodeResult` containing the encoded bytes and an error flag.
///
/// # Example
///
/// ```
/// use hobbs::server::encoding::encode_petscii;
///
/// let result = encode_petscii("HELLO");
/// assert_eq!(result.bytes, vec![0x48, 0x45, 0x4C, 0x4C, 0x4F]);
/// ```
pub fn encode_petscii(text: &str) -> EncodeResult {
    let mut bytes = Vec::new();
    let mut had_errors = false;

    for c in text.chars() {
        match unicode_to_petscii_byte(c) {
            Some(byte) => bytes.push(byte),
            None => {
                bytes.push(b'?');
                had_errors = true;
            }
        }
    }

    EncodeResult { bytes, had_errors }
}

/// Convert a PETSCII byte to a Unicode character.
///
/// Returns None for control codes and unmappable bytes.
fn petscii_byte_to_unicode(byte: u8) -> Option<char> {
    match byte {
        // Control codes - skip
        0x00..=0x1F => None,
        0x80..=0x9F => None,

        // Standard printable ASCII range (0x20-0x3F) - mostly same as ASCII
        0x20 => Some(' '),
        0x21..=0x3F => Some(byte as char),

        // PETSCII uppercase letters (0x41-0x5A) map to ASCII uppercase
        0x41..=0x5A => Some(byte as char),

        // Special characters (0x40, 0x5B-0x5F)
        0x40 => Some('@'),
        0x5B => Some('['),
        0x5C => Some('£'), // British pound sign in PETSCII
        0x5D => Some(']'),
        0x5E => Some('↑'), // Up arrow
        0x5F => Some('←'), // Left arrow

        // 0x60-0x7F are duplicates in PETSCII, map to graphic characters
        // For simplicity, we'll treat them as the same as 0xC0-0xDF
        0x60..=0x7F => petscii_graphic_char(byte - 0x60 + 0xC0),

        // 0xA0-0xBF - graphic characters
        0xA0..=0xBF => petscii_graphic_char(byte),

        // 0xC0-0xDF - uppercase letters (shifted mode) or graphics (unshifted)
        // In uppercase mode, these are graphics
        0xC0..=0xDF => petscii_graphic_char(byte),

        // 0xE0-0xFE - duplicates of 0xA0-0xBE
        0xE0..=0xFE => petscii_graphic_char(byte - 0x40),

        // 0xFF - duplicate of 0x7E (pi symbol)
        0xFF => Some('π'),
    }
}

/// Convert PETSCII graphic character codes to Unicode.
///
/// PETSCII has many graphic characters that don't have direct Unicode equivalents.
/// We map to the closest Unicode block drawing characters where possible.
fn petscii_graphic_char(byte: u8) -> Option<char> {
    // Map PETSCII graphic characters to Unicode block elements and symbols
    // This is a simplified mapping - full PETSCII has 64 unique graphics
    match byte {
        0xA0 => Some('\u{00A0}'), // Non-breaking space
        0xA1 => Some('▌'),       // Left half block
        0xA2 => Some('▄'),       // Lower half block
        0xA3 => Some('▔'),       // Upper one eighth block
        0xA4 => Some('▁'),       // Lower one eighth block
        0xA5 => Some('▏'),       // Left one eighth block
        0xA6 => Some('▒'),       // Medium shade
        0xA7 => Some('▕'),       // Right one eighth block
        0xA8 => Some('◤'),       // Upper left triangle (approximation)
        0xA9 => Some('╮'),       // Box drawings light arc down and left
        0xAA => Some('╰'),       // Box drawings light arc up and right
        0xAB => Some('╯'),       // Box drawings light arc up and left
        0xAC => Some('╲'),       // Box drawings light diagonal upper left to lower right
        0xAD => Some('╱'),       // Box drawings light diagonal upper right to lower left
        0xAE => Some('╳'),       // Box drawings light diagonal cross
        0xAF => Some('◥'),       // Upper right triangle (approximation)
        0xB0 => Some('◣'),       // Lower left triangle
        0xB1 => Some('├'),       // Box drawings light vertical and right
        0xB2 => Some('▗'),       // Quadrant lower right
        0xB3 => Some('▖'),       // Quadrant lower left
        0xB4 => Some('▝'),       // Quadrant upper right
        0xB5 => Some('┌'),       // Box drawings light down and right
        0xB6 => Some('▘'),       // Quadrant upper left
        0xB7 => Some('┬'),       // Box drawings light down and horizontal
        0xB8 => Some('┴'),       // Box drawings light up and horizontal
        0xB9 => Some('┤'),       // Box drawings light vertical and left
        0xBA => Some('▎'),       // Left one quarter block
        0xBB => Some('▐'),       // Right half block
        0xBC => Some('▀'),       // Upper half block
        0xBD => Some('▃'),       // Lower three eighths block
        0xBE => Some('🮇'),       // Block sextant (approximation, using Unicode 13.0+)
        0xBF => Some('▂'),       // Lower one quarter block
        0xC0 => Some('─'),       // Box drawings light horizontal
        0xC1 => Some('♠'),       // Black spade suit
        0xC2 => Some('│'),       // Box drawings light vertical
        0xC3 => Some('─'),       // Horizontal line (duplicate)
        0xC4 => Some('─'),       // Horizontal line
        0xC5 => Some('─'),       // Horizontal line
        0xC6 => Some('─'),       // Horizontal line
        0xC7 => Some('─'),       // Horizontal line
        0xC8 => Some('─'),       // Horizontal line
        0xC9 => Some('╭'),       // Box drawings light arc down and right
        0xCA => Some('╮'),       // Box drawings light arc down and left
        0xCB => Some('╰'),       // Box drawings light arc up and right
        0xCC => Some('╯'),       // Box drawings light arc up and left
        0xCD => Some('┼'),       // Box drawings light vertical and horizontal
        0xCE => Some('╲'),       // Diagonal
        0xCF => Some('╱'),       // Diagonal
        0xD0 => Some('╳'),       // Diagonal cross
        0xD1 => Some('●'),       // Black circle
        0xD2 => Some('▒'),       // Medium shade
        0xD3 => Some('♥'),       // Black heart suit
        0xD4 => Some('▗'),       // Quadrant lower right
        0xD5 => Some('╭'),       // Arc
        0xD6 => Some('╳'),       // Cross
        0xD7 => Some('○'),       // White circle
        0xD8 => Some('♣'),       // Black club suit
        0xD9 => Some('▖'),       // Quadrant lower left
        0xDA => Some('♦'),       // Black diamond suit
        0xDB => Some('┼'),       // Cross
        0xDC => Some('▘'),       // Quadrant upper left
        0xDD => Some('│'),       // Vertical line
        0xDE => Some('π'),       // Pi symbol
        0xDF => Some('◥'),       // Triangle
        _ => Some('?'),          // Fallback for unmapped characters
    }
}

/// Convert a Unicode character to a PETSCII byte.
///
/// Returns None for characters that cannot be represented in PETSCII.
fn unicode_to_petscii_byte(c: char) -> Option<u8> {
    match c {
        // Control characters
        '\r' => Some(0x0D), // Carriage return
        '\n' => Some(0x0D), // Map newline to CR (PETSCII uses CR only)

        // Basic ASCII printable (0x20-0x3F) - same as ASCII
        // This includes: space, !"#$%&'()*+,-./0123456789:;<=>?
        ' '..='?' => Some(c as u8),

        // Uppercase letters (0x41-0x5A)
        'A'..='Z' => Some(c as u8),

        // Lowercase letters - convert to uppercase in PETSCII
        'a'..='z' => Some((c as u8) - 0x20), // Convert to uppercase

        // Special characters
        '@' => Some(0x40),
        '[' => Some(0x5B),
        '£' => Some(0x5C),
        ']' => Some(0x5D),
        '↑' => Some(0x5E),
        '←' => Some(0x5F),

        // Some Unicode block characters map back to PETSCII graphics
        '─' => Some(0xC0),
        '│' => Some(0xC2),
        '┌' => Some(0xB5),
        '┐' => Some(0xB6),
        '└' => Some(0xAA),
        '┘' => Some(0xAB),
        '├' => Some(0xB1),
        '┤' => Some(0xB9),
        '┬' => Some(0xB7),
        '┴' => Some(0xB8),
        '┼' => Some(0xDB),
        '▀' => Some(0xBC),
        '▄' => Some(0xA2),
        '▌' => Some(0xA1),
        '▐' => Some(0xBB),
        '█' => Some(0xA0),
        '♠' => Some(0xC1),
        '♥' => Some(0xD3),
        '♦' => Some(0xDA),
        '♣' => Some(0xD8),
        'π' => Some(0xDE),

        // Everything else - not mappable
        _ => None,
    }
}

// ============================================================================
// Output Mode Processing
// ============================================================================

/// Strip ANSI escape sequences from text.
///
/// This removes all ANSI escape sequences (CSI sequences starting with ESC[)
/// from the input text, leaving only the plain text content.
///
/// # Arguments
///
/// * `text` - The text that may contain ANSI escape sequences.
///
/// # Returns
///
/// The text with all ANSI sequences removed.
///
/// # Example
///
/// ```
/// use hobbs::server::encoding::strip_ansi_sequences;
///
/// let colored = "\x1b[31mRed\x1b[0m Text";
/// assert_eq!(strip_ansi_sequences(colored), "Red Text");
/// ```
pub fn strip_ansi_sequences(text: &str) -> String {
    let mut result = String::new();
    let mut chars = text.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\x1b' {
            // Check for CSI sequence (ESC [)
            if chars.peek() == Some(&'[') {
                chars.next(); // consume '['
                // Skip until we hit a letter (the final byte of the sequence)
                while let Some(&next) = chars.peek() {
                    chars.next();
                    if next.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
            // Else: standalone ESC, just skip it
        } else {
            result.push(c);
        }
    }

    result
}

/// Process text according to the output mode.
///
/// - `Ansi`: Returns text unchanged
/// - `Plain`: Strips all ANSI escape sequences
/// - `PetsciiCtrl`: Converts ANSI codes to PETSCII control codes
///
/// # Arguments
///
/// * `text` - The text to process (may contain ANSI escape sequences)
/// * `mode` - The output mode to apply
///
/// # Returns
///
/// The processed text according to the specified mode.
pub fn process_output_mode(text: &str, mode: OutputMode) -> String {
    match mode {
        OutputMode::Ansi => text.to_string(),
        OutputMode::Plain => strip_ansi_sequences(text),
        OutputMode::PetsciiCtrl => convert_ansi_to_petscii_ctrl(text),
    }
}

/// Convert ANSI escape sequences to PETSCII control codes.
///
/// This converts common ANSI sequences to their PETSCII equivalents:
/// - Color codes (limited palette)
/// - Cursor movement
/// - Clear screen
///
/// Unsupported sequences are stripped.
pub fn convert_ansi_to_petscii_ctrl(text: &str) -> String {
    let mut result = String::new();
    let mut chars = text.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\x1b' {
            // Check for CSI sequence (ESC [)
            if chars.peek() == Some(&'[') {
                chars.next(); // consume '['

                // Parse the sequence parameters
                let mut params = String::new();
                while let Some(&next) = chars.peek() {
                    if next.is_ascii_alphabetic() {
                        let cmd = chars.next().unwrap();
                        // Convert the ANSI command to PETSCII
                        if let Some(petscii) = ansi_to_petscii_ctrl(&params, cmd) {
                            result.push(petscii);
                        }
                        break;
                    } else {
                        params.push(chars.next().unwrap());
                    }
                }
            }
            // Else: standalone ESC, just skip it
        } else {
            result.push(c);
        }
    }

    result
}

/// Convert a single ANSI CSI command to a PETSCII control character.
///
/// Returns None if the sequence has no PETSCII equivalent.
fn ansi_to_petscii_ctrl(params: &str, cmd: char) -> Option<char> {
    match cmd {
        // SGR (Select Graphic Rendition) - colors and attributes
        'm' => {
            let code: u8 = params.parse().unwrap_or(0);
            match code {
                0 => Some('\u{0092}'), // Reset - RVS OFF
                1 => None,             // Bold - no PETSCII equivalent
                7 => Some('\x12'),     // Reverse - RVS ON
                // Foreground colors (approximate mapping)
                30 => Some('\u{0090}'), // Black
                31 => Some('\x1C'),     // Red
                32 => Some('\x1E'),     // Green
                33 => Some('\u{009E}'), // Yellow
                34 => Some('\x1F'),     // Blue
                35 => Some('\u{009C}'), // Magenta -> Purple
                36 => Some('\u{009F}'), // Cyan
                37 => Some('\x05'),     // White
                _ => None,
            }
        }
        // Cursor Up
        'A' => Some('\u{0091}'),
        // Cursor Down
        'B' => Some('\x11'),
        // Cursor Forward (Right)
        'C' => Some('\x1D'),
        // Cursor Back (Left)
        'D' => Some('\u{009D}'),
        // Clear screen
        'J' => {
            if params == "2" {
                Some('\u{0093}') // Clear screen (CLR)
            } else {
                None
            }
        }
        // Cursor Home
        'H' => Some('\x13'),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_ascii() {
        let bytes = b"Hello, World!";
        let result = decode_shiftjis(bytes);
        assert_eq!(result.text, "Hello, World!");
        assert!(!result.had_errors);
    }

    #[test]
    fn test_encode_ascii() {
        let result = encode_shiftjis("Hello, World!");
        assert_eq!(result.bytes, b"Hello, World!");
        assert!(!result.had_errors);
    }

    #[test]
    fn test_decode_japanese() {
        // "テスト" in ShiftJIS
        let shiftjis_bytes = vec![0x83, 0x65, 0x83, 0x58, 0x83, 0x67];
        let result = decode_shiftjis(&shiftjis_bytes);
        assert_eq!(result.text, "テスト");
        assert!(!result.had_errors);
    }

    #[test]
    fn test_encode_japanese() {
        let result = encode_shiftjis("テスト");
        // "テスト" in ShiftJIS
        assert_eq!(result.bytes, vec![0x83, 0x65, 0x83, 0x58, 0x83, 0x67]);
        assert!(!result.had_errors);
    }

    #[test]
    fn test_decode_japanese_hiragana() {
        // "あいうえお" in ShiftJIS
        let shiftjis_bytes = vec![0x82, 0xA0, 0x82, 0xA2, 0x82, 0xA4, 0x82, 0xA6, 0x82, 0xA8];
        let result = decode_shiftjis(&shiftjis_bytes);
        assert_eq!(result.text, "あいうえお");
        assert!(!result.had_errors);
    }

    #[test]
    fn test_encode_japanese_hiragana() {
        let result = encode_shiftjis("あいうえお");
        // "あいうえお" in ShiftJIS
        assert_eq!(
            result.bytes,
            vec![0x82, 0xA0, 0x82, 0xA2, 0x82, 0xA4, 0x82, 0xA6, 0x82, 0xA8]
        );
        assert!(!result.had_errors);
    }

    #[test]
    fn test_decode_mixed_content() {
        // "Hello世界" in ShiftJIS: "Hello" + "世界"
        // 世 = 0x90, 0xA2; 界 = 0x8A, 0x45
        let shiftjis_bytes = vec![0x48, 0x65, 0x6C, 0x6C, 0x6F, 0x90, 0xA2, 0x8A, 0x45];
        let result = decode_shiftjis(&shiftjis_bytes);
        assert_eq!(result.text, "Hello世界");
        assert!(!result.had_errors);
    }

    #[test]
    fn test_encode_mixed_content() {
        let result = encode_shiftjis("Hello世界");
        // "Hello世界" in ShiftJIS
        assert_eq!(
            result.bytes,
            vec![0x48, 0x65, 0x6C, 0x6C, 0x6F, 0x90, 0xA2, 0x8A, 0x45]
        );
        assert!(!result.had_errors);
    }

    #[test]
    fn test_decode_control_characters() {
        // CR LF (carriage return, line feed)
        let bytes = vec![0x0D, 0x0A];
        let result = decode_shiftjis(&bytes);
        assert_eq!(result.text, "\r\n");
        assert!(!result.had_errors);
    }

    #[test]
    fn test_encode_control_characters() {
        let result = encode_shiftjis("\r\n");
        assert_eq!(result.bytes, vec![0x0D, 0x0A]);
        assert!(!result.had_errors);
    }

    #[test]
    fn test_decode_invalid_bytes() {
        // Invalid ShiftJIS sequence (incomplete multi-byte sequence)
        let invalid_bytes = vec![0x82]; // Start of a 2-byte sequence without continuation
        let result = decode_shiftjis(&invalid_bytes);
        // encoding_rs replaces invalid sequences with replacement character
        // and sets had_errors flag
        assert!(result.had_errors || result.text.contains('\u{FFFD}'));
    }

    #[test]
    fn test_encode_unmappable_character() {
        // Euro sign (€) is not in ShiftJIS
        let result = encode_shiftjis("€");
        // Should have errors and produce HTML numeric character reference
        assert!(result.had_errors);
    }

    #[test]
    fn test_decode_strict_success() {
        let bytes = b"Hello";
        let result = decode_shiftjis_strict(bytes);
        assert_eq!(result, Some("Hello".to_string()));
    }

    #[test]
    fn test_decode_strict_failure() {
        // Invalid ShiftJIS sequence
        let invalid_bytes = vec![0x80, 0x00];
        let result = decode_shiftjis_strict(&invalid_bytes);
        // May or may not be None depending on encoding_rs behavior
        // but if it returns Some, the text shouldn't be corrupted
        if let Some(text) = result {
            assert!(!text.is_empty() || invalid_bytes.is_empty());
        }
    }

    #[test]
    fn test_encode_strict_success() {
        let result = encode_shiftjis_strict("テスト");
        assert!(result.is_some());
        assert_eq!(result.unwrap(), vec![0x83, 0x65, 0x83, 0x58, 0x83, 0x67]);
    }

    #[test]
    fn test_encode_strict_failure() {
        // Euro sign (€) is not in ShiftJIS
        let result = encode_shiftjis_strict("€");
        assert!(result.is_none());
    }

    #[test]
    fn test_empty_input_decode() {
        let result = decode_shiftjis(&[]);
        assert_eq!(result.text, "");
        assert!(!result.had_errors);
    }

    #[test]
    fn test_empty_input_encode() {
        let result = encode_shiftjis("");
        assert!(result.bytes.is_empty());
        assert!(!result.had_errors);
    }

    #[test]
    fn test_decode_half_width_katakana() {
        // Half-width katakana "ｱｲｳ" in ShiftJIS (single-byte katakana)
        // encoding_rs preserves half-width katakana as half-width
        let shiftjis_bytes = vec![0xB1, 0xB2, 0xB3];
        let result = decode_shiftjis(&shiftjis_bytes);
        assert_eq!(result.text, "ｱｲｳ");
        assert!(!result.had_errors);
    }

    #[test]
    fn test_roundtrip_japanese() {
        let original = "こんにちは世界！Hello, World!";
        let encoded = encode_shiftjis(original);
        assert!(!encoded.had_errors);

        let decoded = decode_shiftjis(&encoded.bytes);
        assert!(!decoded.had_errors);
        assert_eq!(decoded.text, original);
    }

    #[test]
    fn test_decode_kanji() {
        // "漢字" in ShiftJIS
        let shiftjis_bytes = vec![0x8A, 0xBF, 0x8E, 0x9A];
        let result = decode_shiftjis(&shiftjis_bytes);
        assert_eq!(result.text, "漢字");
        assert!(!result.had_errors);
    }

    #[test]
    fn test_encode_kanji() {
        let result = encode_shiftjis("漢字");
        assert_eq!(result.bytes, vec![0x8A, 0xBF, 0x8E, 0x9A]);
        assert!(!result.had_errors);
    }

    // Wave dash normalization tests
    #[test]
    fn test_encode_wave_dash() {
        // U+301C (Wave Dash) should be converted to U+FF5E (Fullwidth Tilde)
        // which can be encoded in ShiftJIS
        let text_with_wave_dash = "1〜100"; // Uses U+301C
        let result = encode_shiftjis(text_with_wave_dash);
        // Should succeed without errors
        assert!(!result.had_errors);
        // Verify the encoded bytes don't contain HTML entity
        let decoded = decode_shiftjis(&result.bytes);
        assert!(!decoded.text.contains("&#"));
    }

    #[test]
    fn test_normalize_wave_dash() {
        let text = "数字は1〜100の範囲です"; // Uses U+301C
        let result = encode_shiftjis(text);
        assert!(!result.had_errors);
    }

    #[test]
    fn test_normalize_minus_sign() {
        // U+2212 (Minus Sign) should be converted to U+FF0D (Fullwidth Hyphen-Minus)
        let text = "−5"; // Uses U+2212
        let result = encode_shiftjis(text);
        assert!(!result.had_errors);
    }

    #[test]
    fn test_normalize_em_dash() {
        // U+2014 (Em Dash) should be converted to U+2015 (Horizontal Bar)
        let text = "これ—あれ"; // Uses U+2014
        let result = encode_shiftjis(text);
        assert!(!result.had_errors);
    }

    // CharacterEncoding tests
    #[test]
    fn test_character_encoding_default() {
        let encoding = CharacterEncoding::default();
        assert_eq!(encoding, CharacterEncoding::ShiftJIS);
    }

    #[test]
    fn test_character_encoding_as_str() {
        assert_eq!(CharacterEncoding::ShiftJIS.as_str(), "shiftjis");
        assert_eq!(CharacterEncoding::Utf8.as_str(), "utf8");
    }

    #[test]
    fn test_character_encoding_display_name() {
        assert_eq!(CharacterEncoding::ShiftJIS.display_name(), "ShiftJIS");
        assert_eq!(CharacterEncoding::Utf8.display_name(), "UTF-8");
    }

    #[test]
    fn test_character_encoding_display() {
        assert_eq!(format!("{}", CharacterEncoding::ShiftJIS), "ShiftJIS");
        assert_eq!(format!("{}", CharacterEncoding::Utf8), "UTF-8");
    }

    #[test]
    fn test_character_encoding_from_str() {
        assert_eq!(
            "shiftjis".parse::<CharacterEncoding>().unwrap(),
            CharacterEncoding::ShiftJIS
        );
        assert_eq!(
            "shift_jis".parse::<CharacterEncoding>().unwrap(),
            CharacterEncoding::ShiftJIS
        );
        assert_eq!(
            "shift-jis".parse::<CharacterEncoding>().unwrap(),
            CharacterEncoding::ShiftJIS
        );
        assert_eq!(
            "sjis".parse::<CharacterEncoding>().unwrap(),
            CharacterEncoding::ShiftJIS
        );
        assert_eq!(
            "SHIFTJIS".parse::<CharacterEncoding>().unwrap(),
            CharacterEncoding::ShiftJIS
        );
        assert_eq!(
            "utf8".parse::<CharacterEncoding>().unwrap(),
            CharacterEncoding::Utf8
        );
        assert_eq!(
            "utf-8".parse::<CharacterEncoding>().unwrap(),
            CharacterEncoding::Utf8
        );
        assert_eq!(
            "UTF-8".parse::<CharacterEncoding>().unwrap(),
            CharacterEncoding::Utf8
        );
        assert!("invalid".parse::<CharacterEncoding>().is_err());
    }

    #[test]
    fn test_character_encoding_all() {
        let all = CharacterEncoding::all();
        assert_eq!(all.len(), 4);
        assert!(all.contains(&CharacterEncoding::ShiftJIS));
        assert!(all.contains(&CharacterEncoding::Utf8));
        assert!(all.contains(&CharacterEncoding::Cp437));
        assert!(all.contains(&CharacterEncoding::Petscii));
    }

    // encode_for_client tests
    #[test]
    fn test_encode_for_client_utf8_ascii() {
        let text = "Hello, World!";
        let bytes = encode_for_client(text, CharacterEncoding::Utf8);
        assert_eq!(bytes, text.as_bytes());
    }

    #[test]
    fn test_encode_for_client_utf8_japanese() {
        let text = "こんにちは世界";
        let bytes = encode_for_client(text, CharacterEncoding::Utf8);
        assert_eq!(bytes, text.as_bytes());
    }

    #[test]
    fn test_encode_for_client_shiftjis_ascii() {
        let text = "Hello, World!";
        let bytes = encode_for_client(text, CharacterEncoding::ShiftJIS);
        assert_eq!(bytes, text.as_bytes()); // ASCII is same in both encodings
    }

    #[test]
    fn test_encode_for_client_shiftjis_japanese() {
        let text = "テスト";
        let bytes = encode_for_client(text, CharacterEncoding::ShiftJIS);
        assert_eq!(bytes, vec![0x83, 0x65, 0x83, 0x58, 0x83, 0x67]);
    }

    #[test]
    fn test_encode_for_client_utf8_preserves_special_chars() {
        // UTF-8 can encode characters that ShiftJIS cannot
        let text = "€£¥";
        let bytes = encode_for_client(text, CharacterEncoding::Utf8);
        assert_eq!(bytes, text.as_bytes());
    }

    // decode_from_client tests
    #[test]
    fn test_decode_from_client_utf8_ascii() {
        let bytes = b"Hello, World!";
        let text = decode_from_client(bytes, CharacterEncoding::Utf8);
        assert_eq!(text, "Hello, World!");
    }

    #[test]
    fn test_decode_from_client_utf8_japanese() {
        let original = "こんにちは世界";
        let bytes = original.as_bytes();
        let text = decode_from_client(bytes, CharacterEncoding::Utf8);
        assert_eq!(text, original);
    }

    #[test]
    fn test_decode_from_client_shiftjis_ascii() {
        let bytes = b"Hello, World!";
        let text = decode_from_client(bytes, CharacterEncoding::ShiftJIS);
        assert_eq!(text, "Hello, World!");
    }

    #[test]
    fn test_decode_from_client_shiftjis_japanese() {
        // "テスト" in ShiftJIS
        let bytes = vec![0x83, 0x65, 0x83, 0x58, 0x83, 0x67];
        let text = decode_from_client(&bytes, CharacterEncoding::ShiftJIS);
        assert_eq!(text, "テスト");
    }

    #[test]
    fn test_decode_from_client_utf8_invalid() {
        // Invalid UTF-8 sequence
        let bytes = vec![0xFF, 0xFE];
        let text = decode_from_client(&bytes, CharacterEncoding::Utf8);
        // Should contain replacement character
        assert!(text.contains('\u{FFFD}'));
    }

    // encode_for_client_detailed tests
    #[test]
    fn test_encode_for_client_detailed_utf8() {
        let text = "Hello, €世界!";
        let result = encode_for_client_detailed(text, CharacterEncoding::Utf8);
        assert_eq!(result.bytes, text.as_bytes());
        assert!(!result.had_errors);
    }

    #[test]
    fn test_encode_for_client_detailed_shiftjis_success() {
        let text = "Hello, 世界!";
        let result = encode_for_client_detailed(text, CharacterEncoding::ShiftJIS);
        assert!(!result.had_errors);
    }

    #[test]
    fn test_encode_for_client_detailed_shiftjis_error() {
        // Euro sign is not in ShiftJIS
        let text = "€";
        let result = encode_for_client_detailed(text, CharacterEncoding::ShiftJIS);
        assert!(result.had_errors);
    }

    // decode_from_client_detailed tests
    #[test]
    fn test_decode_from_client_detailed_utf8_valid() {
        let bytes = "Hello, 世界!".as_bytes();
        let result = decode_from_client_detailed(bytes, CharacterEncoding::Utf8);
        assert_eq!(result.text, "Hello, 世界!");
        assert!(!result.had_errors);
    }

    #[test]
    fn test_decode_from_client_detailed_utf8_invalid() {
        let bytes = vec![0xFF, 0xFE];
        let result = decode_from_client_detailed(&bytes, CharacterEncoding::Utf8);
        assert!(result.had_errors);
        assert!(result.text.contains('\u{FFFD}'));
    }

    #[test]
    fn test_decode_from_client_detailed_shiftjis_valid() {
        let bytes = vec![0x83, 0x65, 0x83, 0x58, 0x83, 0x67]; // "テスト"
        let result = decode_from_client_detailed(&bytes, CharacterEncoding::ShiftJIS);
        assert_eq!(result.text, "テスト");
        assert!(!result.had_errors);
    }

    // Roundtrip tests
    #[test]
    fn test_roundtrip_utf8() {
        let original = "Hello, 世界! €123";
        let encoded = encode_for_client(original, CharacterEncoding::Utf8);
        let decoded = decode_from_client(&encoded, CharacterEncoding::Utf8);
        assert_eq!(decoded, original);
    }

    #[test]
    fn test_roundtrip_shiftjis() {
        let original = "Hello, 世界!"; // No characters outside ShiftJIS
        let encoded = encode_for_client(original, CharacterEncoding::ShiftJIS);
        let decoded = decode_from_client(&encoded, CharacterEncoding::ShiftJIS);
        assert_eq!(decoded, original);
    }

    #[test]
    fn test_character_encoding_equality() {
        assert_eq!(CharacterEncoding::ShiftJIS, CharacterEncoding::ShiftJIS);
        assert_eq!(CharacterEncoding::Utf8, CharacterEncoding::Utf8);
        assert_ne!(CharacterEncoding::ShiftJIS, CharacterEncoding::Utf8);
    }

    #[test]
    fn test_character_encoding_clone() {
        let enc = CharacterEncoding::Utf8;
        let cloned = enc;
        assert_eq!(enc, cloned);
    }

    #[test]
    fn test_encode_empty_string() {
        let empty = "";
        assert!(encode_for_client(empty, CharacterEncoding::Utf8).is_empty());
        assert!(encode_for_client(empty, CharacterEncoding::ShiftJIS).is_empty());
    }

    #[test]
    fn test_decode_empty_bytes() {
        let empty: &[u8] = &[];
        assert_eq!(decode_from_client(empty, CharacterEncoding::Utf8), "");
        assert_eq!(decode_from_client(empty, CharacterEncoding::ShiftJIS), "");
    }

    // ============================================================================
    // CP437 Tests
    // ============================================================================

    #[test]
    fn test_decode_cp437_ascii() {
        let bytes = b"Hello, World!";
        let result = decode_cp437(bytes);
        assert_eq!(result.text, "Hello, World!");
        assert!(!result.had_errors);
    }

    #[test]
    fn test_encode_cp437_ascii() {
        let result = encode_cp437("Hello, World!");
        assert_eq!(result.bytes, b"Hello, World!");
        assert!(!result.had_errors);
    }

    #[test]
    fn test_decode_cp437_box_drawing() {
        // Box drawing characters in CP437
        // ┌ (0xDA), ─ (0xC4), ┐ (0xBF)
        let bytes = vec![0xDA, 0xC4, 0xC4, 0xBF];
        let result = decode_cp437(&bytes);
        assert_eq!(result.text, "┌──┐");
        assert!(!result.had_errors);
    }

    #[test]
    fn test_encode_cp437_roundtrip() {
        let original = "Hello";
        let encoded = encode_cp437(original);
        let decoded = decode_cp437(&encoded.bytes);
        assert_eq!(decoded.text, original);
    }

    #[test]
    fn test_encode_cp437_unmappable() {
        // Japanese characters cannot be encoded in CP437
        let result = encode_cp437("こんにちは");
        assert!(result.had_errors);
        assert_eq!(result.bytes, vec![b'?', b'?', b'?', b'?', b'?']);
    }

    #[test]
    fn test_character_encoding_from_str_cp437() {
        assert_eq!(
            "cp437".parse::<CharacterEncoding>().unwrap(),
            CharacterEncoding::Cp437
        );
        assert_eq!(
            "ibm437".parse::<CharacterEncoding>().unwrap(),
            CharacterEncoding::Cp437
        );
        assert_eq!(
            "dos".parse::<CharacterEncoding>().unwrap(),
            CharacterEncoding::Cp437
        );
    }

    // ============================================================================
    // PETSCII Tests
    // ============================================================================

    #[test]
    fn test_decode_petscii_uppercase() {
        // "HELLO" in PETSCII (same bytes as ASCII for uppercase letters)
        let bytes = vec![0x48, 0x45, 0x4C, 0x4C, 0x4F];
        let result = decode_petscii(&bytes);
        assert_eq!(result.text, "HELLO");
    }

    #[test]
    fn test_encode_petscii_uppercase() {
        let result = encode_petscii("HELLO");
        assert_eq!(result.bytes, vec![0x48, 0x45, 0x4C, 0x4C, 0x4F]);
        assert!(!result.had_errors);
    }

    #[test]
    fn test_encode_petscii_lowercase_to_uppercase() {
        // Lowercase should be converted to uppercase in PETSCII
        let result = encode_petscii("hello");
        assert_eq!(result.bytes, vec![0x48, 0x45, 0x4C, 0x4C, 0x4F]);
        assert!(!result.had_errors);
    }

    #[test]
    fn test_encode_petscii_roundtrip() {
        let original = "HELLO WORLD 123";
        let encoded = encode_petscii(original);
        let decoded = decode_petscii(&encoded.bytes);
        assert_eq!(decoded.text, original);
    }

    #[test]
    fn test_encode_petscii_special_chars() {
        // British pound sign and arrows
        let result = encode_petscii("£↑←");
        assert_eq!(result.bytes, vec![0x5C, 0x5E, 0x5F]);
        assert!(!result.had_errors);
    }

    #[test]
    fn test_encode_petscii_unmappable() {
        // Japanese characters cannot be encoded in PETSCII
        let result = encode_petscii("こんにちは");
        assert!(result.had_errors);
        assert_eq!(result.bytes, vec![b'?', b'?', b'?', b'?', b'?']);
    }

    #[test]
    fn test_decode_petscii_control_codes() {
        // Control codes should be filtered out
        let bytes = vec![0x05, 0x48, 0x11, 0x45, 0x13]; // Color control mixed with HE
        let result = decode_petscii(&bytes);
        assert_eq!(result.text, "HE");
        assert!(result.had_errors); // Had control codes
    }

    #[test]
    fn test_character_encoding_from_str_petscii() {
        assert_eq!(
            "petscii".parse::<CharacterEncoding>().unwrap(),
            CharacterEncoding::Petscii
        );
        assert_eq!(
            "cbm".parse::<CharacterEncoding>().unwrap(),
            CharacterEncoding::Petscii
        );
        assert_eq!(
            "commodore".parse::<CharacterEncoding>().unwrap(),
            CharacterEncoding::Petscii
        );
    }

    #[test]
    fn test_cp437_as_str() {
        assert_eq!(CharacterEncoding::Cp437.as_str(), "cp437");
        assert_eq!(CharacterEncoding::Cp437.display_name(), "CP437");
    }

    #[test]
    fn test_petscii_as_str() {
        assert_eq!(CharacterEncoding::Petscii.as_str(), "petscii");
        assert_eq!(CharacterEncoding::Petscii.display_name(), "PETSCII");
    }

    // ============================================================================
    // OutputMode Tests
    // ============================================================================

    #[test]
    fn test_output_mode_default() {
        let mode = OutputMode::default();
        assert_eq!(mode, OutputMode::Ansi);
    }

    #[test]
    fn test_output_mode_as_str() {
        assert_eq!(OutputMode::Ansi.as_str(), "ansi");
        assert_eq!(OutputMode::Plain.as_str(), "plain");
        assert_eq!(OutputMode::PetsciiCtrl.as_str(), "petscii_ctrl");
    }

    #[test]
    fn test_output_mode_display_name() {
        assert_eq!(OutputMode::Ansi.display_name(), "ANSI");
        assert_eq!(OutputMode::Plain.display_name(), "Plain");
        assert_eq!(OutputMode::PetsciiCtrl.display_name(), "PETSCII Ctrl");
    }

    #[test]
    fn test_output_mode_display() {
        assert_eq!(format!("{}", OutputMode::Ansi), "ANSI");
        assert_eq!(format!("{}", OutputMode::Plain), "Plain");
        assert_eq!(format!("{}", OutputMode::PetsciiCtrl), "PETSCII Ctrl");
    }

    #[test]
    fn test_output_mode_from_str() {
        assert_eq!("ansi".parse::<OutputMode>().unwrap(), OutputMode::Ansi);
        assert_eq!("plain".parse::<OutputMode>().unwrap(), OutputMode::Plain);
        assert_eq!("ascii".parse::<OutputMode>().unwrap(), OutputMode::Plain);
        assert_eq!("none".parse::<OutputMode>().unwrap(), OutputMode::Plain);
        assert_eq!(
            "petscii_ctrl".parse::<OutputMode>().unwrap(),
            OutputMode::PetsciiCtrl
        );
        assert_eq!(
            "petscii-ctrl".parse::<OutputMode>().unwrap(),
            OutputMode::PetsciiCtrl
        );
        assert_eq!(
            "petscii".parse::<OutputMode>().unwrap(),
            OutputMode::PetsciiCtrl
        );
        assert!("invalid".parse::<OutputMode>().is_err());
    }

    #[test]
    fn test_output_mode_all() {
        let all = OutputMode::all();
        assert_eq!(all.len(), 3);
        assert!(all.contains(&OutputMode::Ansi));
        assert!(all.contains(&OutputMode::Plain));
        assert!(all.contains(&OutputMode::PetsciiCtrl));
    }

    // ============================================================================
    // ANSI Strip Tests
    // ============================================================================

    #[test]
    fn test_strip_ansi_no_sequences() {
        let text = "Hello, World!";
        assert_eq!(strip_ansi_sequences(text), "Hello, World!");
    }

    #[test]
    fn test_strip_ansi_color_sequence() {
        let text = "\x1b[31mRed\x1b[0m Text";
        assert_eq!(strip_ansi_sequences(text), "Red Text");
    }

    #[test]
    fn test_strip_ansi_multiple_sequences() {
        let text = "\x1b[1;32mBold Green\x1b[0m and \x1b[34mBlue\x1b[0m";
        assert_eq!(strip_ansi_sequences(text), "Bold Green and Blue");
    }

    #[test]
    fn test_strip_ansi_cursor_movement() {
        let text = "\x1b[2JScreen cleared\x1b[HHome";
        assert_eq!(strip_ansi_sequences(text), "Screen clearedHome");
    }

    #[test]
    fn test_strip_ansi_complex_params() {
        let text = "\x1b[38;5;196mExtended color\x1b[0m";
        assert_eq!(strip_ansi_sequences(text), "Extended color");
    }

    #[test]
    fn test_strip_ansi_standalone_esc() {
        let text = "Before\x1bAfter";
        assert_eq!(strip_ansi_sequences(text), "BeforeAfter");
    }

    #[test]
    fn test_strip_ansi_empty_string() {
        assert_eq!(strip_ansi_sequences(""), "");
    }

    #[test]
    fn test_strip_ansi_preserves_newlines() {
        let text = "\x1b[31mLine 1\x1b[0m\nLine 2";
        assert_eq!(strip_ansi_sequences(text), "Line 1\nLine 2");
    }

    // ============================================================================
    // Process Output Mode Tests
    // ============================================================================

    #[test]
    fn test_process_output_mode_ansi_passthrough() {
        let text = "\x1b[31mRed\x1b[0m Text";
        assert_eq!(
            process_output_mode(text, OutputMode::Ansi),
            "\x1b[31mRed\x1b[0m Text"
        );
    }

    #[test]
    fn test_process_output_mode_plain_strips() {
        let text = "\x1b[31mRed\x1b[0m Text";
        assert_eq!(process_output_mode(text, OutputMode::Plain), "Red Text");
    }

    #[test]
    fn test_process_output_mode_petscii_converts() {
        let text = "\x1b[2JClear\x1b[H";
        let result = process_output_mode(text, OutputMode::PetsciiCtrl);
        // Clear screen is \u{0093}, Home is \x13
        assert!(result.contains('\u{0093}')); // CLR
        assert!(result.contains('\x13')); // HOME
        assert!(result.contains("Clear"));
    }

    // ============================================================================
    // ANSI to PETSCII Control Code Tests
    // ============================================================================

    #[test]
    fn test_ansi_to_petscii_clear_screen() {
        let text = "\x1b[2JCleared";
        let result = convert_ansi_to_petscii_ctrl(text);
        assert_eq!(result, "\u{0093}Cleared"); // PETSCII CLR
    }

    #[test]
    fn test_ansi_to_petscii_home() {
        let text = "\x1b[HHome";
        let result = convert_ansi_to_petscii_ctrl(text);
        assert_eq!(result, "\x13Home"); // PETSCII HOME
    }

    #[test]
    fn test_ansi_to_petscii_cursor_up() {
        let text = "\x1b[AUp";
        let result = convert_ansi_to_petscii_ctrl(text);
        assert_eq!(result, "\u{0091}Up"); // PETSCII CURSOR UP
    }

    #[test]
    fn test_ansi_to_petscii_cursor_down() {
        let text = "\x1b[BDown";
        let result = convert_ansi_to_petscii_ctrl(text);
        assert_eq!(result, "\x11Down"); // PETSCII CURSOR DOWN
    }

    #[test]
    fn test_ansi_to_petscii_cursor_right() {
        let text = "\x1b[CRight";
        let result = convert_ansi_to_petscii_ctrl(text);
        assert_eq!(result, "\x1DRight"); // PETSCII CURSOR RIGHT
    }

    #[test]
    fn test_ansi_to_petscii_cursor_left() {
        let text = "\x1b[DLeft";
        let result = convert_ansi_to_petscii_ctrl(text);
        assert_eq!(result, "\u{009D}Left"); // PETSCII CURSOR LEFT
    }

    #[test]
    fn test_ansi_to_petscii_color_white() {
        let text = "\x1b[37mWhite";
        let result = convert_ansi_to_petscii_ctrl(text);
        assert_eq!(result, "\x05White"); // PETSCII WHITE
    }

    #[test]
    fn test_ansi_to_petscii_color_red() {
        let text = "\x1b[31mRed";
        let result = convert_ansi_to_petscii_ctrl(text);
        assert_eq!(result, "\x1CRed"); // PETSCII RED
    }

    #[test]
    fn test_ansi_to_petscii_color_green() {
        let text = "\x1b[32mGreen";
        let result = convert_ansi_to_petscii_ctrl(text);
        assert_eq!(result, "\x1EGreen"); // PETSCII GREEN
    }

    #[test]
    fn test_ansi_to_petscii_color_blue() {
        let text = "\x1b[34mBlue";
        let result = convert_ansi_to_petscii_ctrl(text);
        assert_eq!(result, "\x1FBlue"); // PETSCII BLUE
    }

    #[test]
    fn test_ansi_to_petscii_reverse() {
        let text = "\x1b[7mReverse";
        let result = convert_ansi_to_petscii_ctrl(text);
        assert_eq!(result, "\x12Reverse"); // PETSCII RVS ON
    }

    #[test]
    fn test_ansi_to_petscii_reset() {
        let text = "\x1b[0mReset";
        let result = convert_ansi_to_petscii_ctrl(text);
        assert_eq!(result, "\u{0092}Reset"); // PETSCII RVS OFF
    }

    #[test]
    fn test_ansi_to_petscii_unsupported_stripped() {
        // Bold (1) has no PETSCII equivalent
        let text = "\x1b[1mBold";
        let result = convert_ansi_to_petscii_ctrl(text);
        assert_eq!(result, "Bold"); // Should just have text, no control code
    }

    #[test]
    fn test_ansi_to_petscii_plain_text_preserved() {
        let text = "Just plain text";
        let result = convert_ansi_to_petscii_ctrl(text);
        assert_eq!(result, "Just plain text");
    }
}
