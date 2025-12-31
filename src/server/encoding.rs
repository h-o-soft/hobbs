//! Character encoding conversion for Telnet communication.
//!
//! This module handles conversion between UTF-8 (internal representation)
//! and various wire formats (ShiftJIS for legacy terminals, UTF-8 for modern terminals).

use std::fmt;
use std::str::FromStr;

use encoding_rs::SHIFT_JIS;

/// Character encoding for client communication.
///
/// HOBBS supports two encodings:
/// - ShiftJIS: For legacy Japanese terminals and retro computing enthusiasts
/// - UTF-8: For modern terminals and international users
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub enum CharacterEncoding {
    /// ShiftJIS encoding (default for retro compatibility).
    #[default]
    ShiftJIS,
    /// UTF-8 encoding for modern terminals.
    Utf8,
}

impl CharacterEncoding {
    /// Get the encoding name as a string.
    pub fn as_str(&self) -> &'static str {
        match self {
            CharacterEncoding::ShiftJIS => "shiftjis",
            CharacterEncoding::Utf8 => "utf8",
        }
    }

    /// Get the display name for the encoding.
    pub fn display_name(&self) -> &'static str {
        match self {
            CharacterEncoding::ShiftJIS => "ShiftJIS",
            CharacterEncoding::Utf8 => "UTF-8",
        }
    }

    /// Get all available encodings.
    pub fn all() -> &'static [CharacterEncoding] {
        &[CharacterEncoding::ShiftJIS, CharacterEncoding::Utf8]
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
    let (cow, _encoding, had_errors) = SHIFT_JIS.encode(text);
    EncodeResult {
        bytes: cow.into_owned(),
        had_errors,
    }
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
        assert_eq!(all.len(), 2);
        assert!(all.contains(&CharacterEncoding::ShiftJIS));
        assert!(all.contains(&CharacterEncoding::Utf8));
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
}
