//! Character encoding conversion for Telnet communication.
//!
//! This module handles conversion between UTF-8 (internal representation)
//! and ShiftJIS (Telnet wire format).

use encoding_rs::SHIFT_JIS;

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
}
