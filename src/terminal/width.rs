//! Display width calculation.
//!
//! This is the single implementation of "how many columns does this text take".
//! `TerminalProfile`, the template renderer and the screen word-wrapper all
//! delegate here.
//!
//! The rule (unchanged from the previous per-module copies):
//! - `cjk_width == 1`: every character is 1 column.
//! - otherwise: ASCII is 1 column, every non-ASCII character is 2 columns.
//!
//! Note that this does not consult Unicode East Asian Width, so half-width
//! katakana, Latin-1 letters, box drawing and emoji are all counted as 2 when
//! `cjk_width != 1`.

/// Display width of a single character.
pub fn char_width(c: char, cjk_width: usize) -> usize {
    if cjk_width == 1 || c.is_ascii() {
        1
    } else {
        2
    }
}

/// Display width of a string.
pub fn display_width(s: &str, cjk_width: usize) -> usize {
    s.chars().map(|c| char_width(c, cjk_width)).sum()
}

/// Truncate a string so that its display width does not exceed `max_width`.
pub fn truncate_to_width(s: &str, max_width: usize, cjk_width: usize) -> String {
    let mut result = String::new();
    let mut current_width = 0;

    for c in s.chars() {
        let w = char_width(c, cjk_width);
        if current_width + w > max_width {
            break;
        }
        result.push(c);
        current_width += w;
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_char_width() {
        assert_eq!(char_width('A', 2), 1);
        assert_eq!(char_width('あ', 2), 2);
        assert_eq!(char_width('ｶ', 2), 2);
        assert_eq!(char_width('é', 2), 2);
        assert_eq!(char_width('あ', 1), 1);
        assert_eq!(char_width('A', 1), 1);
    }

    #[test]
    fn test_display_width() {
        assert_eq!(display_width("", 2), 0);
        assert_eq!(display_width("Hello", 2), 5);
        assert_eq!(display_width("こんにちは", 2), 10);
        assert_eq!(display_width("こんにちは", 1), 5);
        assert_eq!(display_width("ABCあいう", 2), 9);
    }

    #[test]
    fn test_truncate_to_width() {
        assert_eq!(truncate_to_width("Hello, World!", 5, 2), "Hello");
        assert_eq!(truncate_to_width("こんにちは", 6, 2), "こんに");
        assert_eq!(truncate_to_width("こんにちは", 5, 2), "こん");
        assert_eq!(truncate_to_width("こんにちは", 3, 1), "こんに");
        assert_eq!(truncate_to_width("abc", 0, 2), "");
    }
}
