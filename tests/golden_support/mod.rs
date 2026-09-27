//! Golden (characterization) test helpers.
//!
//! ゴールデンテストは「正しい挙動」ではなく「現状の挙動」を固定するためのもの。
//! 文字コード・端末まわりのリファクタリング（Issue #325, plan.md）の各フェーズで、
//! サーバーが送るバイト列が1バイトも変わっていないことを確かめるために使う。
//!
//! - 比較対象は `tests/golden/<name>.txt`。
//! - `UPDATE_GOLDEN=1 cargo test --test golden_...` のときだけ上書き保存する。
//! - バイト列は `escape_bytes` で可逆なテキストにしてから保存する（PR の差分で読めるように）。

#![allow(dead_code)]

use std::path::PathBuf;

/// Escape a byte sequence into a lossless, diff-friendly text form.
///
/// - Printable ASCII (0x20-0x7E) except `\` is written as-is.
/// - `\` is written as `\\`.
/// - LF is written as `\n` followed by a real newline (for readable diffs).
/// - CR is `\r`, ESC is `\e`, everything else is `\xNN`.
pub fn escape_bytes(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        match b {
            b'\\' => out.push_str("\\\\"),
            b'\n' => out.push_str("\\n\n"),
            b'\r' => out.push_str("\\r"),
            0x1B => out.push_str("\\e"),
            0x20..=0x7E => out.push(b as char),
            _ => out.push_str(&format!("\\x{:02X}", b)),
        }
    }
    out
}

fn is_digit(b: Option<&u8>) -> bool {
    matches!(b, Some(c) if c.is_ascii_digit())
}

fn digits_at(bytes: &[u8], i: usize, n: usize) -> bool {
    i + n <= bytes.len() && bytes[i..i + n].iter().all(|b| b.is_ascii_digit())
}

/// Replace date/time-looking digit runs with fixed placeholders.
///
/// Digits, `/`, `-`, `:` and space are the same bytes in ShiftJIS, UTF-8,
/// CP437 and PETSCII, so this works on raw wire bytes of any encoding.
///
/// - `YYYY/MM/DD`, `YYYY-MM-DD` → `NNNN/NN/NN` (separator kept)
/// - `HH:MM` and `HH:MM:SS` → `NN:NN` / `NN:NN:NN`
/// - `MM/DD` directly followed by a space and `HH:MM` → `NN/NN`
pub fn normalize_datetimes(bytes: &[u8]) -> Vec<u8> {
    let mut out = bytes.to_vec();
    let len = out.len();
    let mut i = 0;
    while i < len {
        let prev_is_digit = i > 0 && out[i - 1].is_ascii_digit();
        if prev_is_digit {
            i += 1;
            continue;
        }
        // YYYY/MM/DD or YYYY-MM-DD
        if digits_at(&out, i, 4)
            && i + 10 <= len
            && (out[i + 4] == b'/' || out[i + 4] == b'-')
            && out[i + 7] == out[i + 4]
            && digits_at(&out, i + 5, 2)
            && digits_at(&out, i + 8, 2)
            && !is_digit(out.get(i + 10))
        {
            for j in [i, i + 1, i + 2, i + 3, i + 5, i + 6, i + 8, i + 9] {
                out[j] = b'N';
            }
            i += 10;
            continue;
        }
        // HH:MM(:SS)
        if digits_at(&out, i, 2)
            && i + 5 <= len
            && out[i + 2] == b':'
            && digits_at(&out, i + 3, 2)
            && !is_digit(out.get(i + 5))
        {
            for j in [i, i + 1, i + 3, i + 4] {
                out[j] = b'N';
            }
            let mut end = i + 5;
            if end + 3 <= len
                && out[end] == b':'
                && digits_at(&out, end + 1, 2)
                && !is_digit(out.get(end + 3))
            {
                out[end + 1] = b'N';
                out[end + 2] = b'N';
                end += 3;
            }
            i = end;
            continue;
        }
        // MM/DD HH:MM
        if digits_at(&out, i, 2)
            && i + 11 <= len
            && out[i + 2] == b'/'
            && digits_at(&out, i + 3, 2)
            && out[i + 5] == b' '
            && digits_at(&out, i + 6, 2)
            && out[i + 8] == b':'
        {
            for j in [i, i + 1, i + 3, i + 4] {
                out[j] = b'N';
            }
            i += 5;
            continue;
        }
        i += 1;
    }
    out
}

fn golden_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("golden")
        .join(format!("{name}.txt"))
}

/// Compare `actual` with the stored golden file, or rewrite it when
/// `UPDATE_GOLDEN=1` is set.
pub fn assert_golden(name: &str, actual: &str) {
    let path = golden_path(name);
    if std::env::var("UPDATE_GOLDEN").as_deref() == Ok("1") {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, actual).unwrap();
        return;
    }
    let expected = std::fs::read_to_string(&path).unwrap_or_else(|_| {
        panic!(
            "golden file missing: {} (run with UPDATE_GOLDEN=1 to create it)",
            path.display()
        )
    });
    if expected != actual {
        let exp_lines: Vec<&str> = expected.lines().collect();
        let act_lines: Vec<&str> = actual.lines().collect();
        let first_diff = exp_lines
            .iter()
            .zip(act_lines.iter())
            .position(|(a, b)| a != b)
            .unwrap_or(exp_lines.len().min(act_lines.len()));
        panic!(
            "golden mismatch: {}\nfirst difference at line {}\n  expected: {:?}\n  actual:   {:?}\n(expected {} lines, actual {} lines)",
            path.display(),
            first_diff + 1,
            exp_lines.get(first_diff),
            act_lines.get(first_diff),
            exp_lines.len(),
            act_lines.len()
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escape_bytes_roundtrip_markers() {
        assert_eq!(escape_bytes(b"a\\b"), "a\\\\b");
        assert_eq!(escape_bytes(b"\r\n"), "\\r\\n\n");
        assert_eq!(escape_bytes(&[0x1B, 0xFF]), "\\e\\xFF");
    }

    #[test]
    fn test_normalize_datetimes() {
        assert_eq!(
            normalize_datetimes(b"Connected: 2026/09/27 16:29:39\r\n"),
            b"Connected: NNNN/NN/NN NN:NN:NN\r\n".to_vec()
        );
        assert_eq!(
            normalize_datetimes(b"at 09/27 16:29 by x"),
            b"at NN/NN NN:NN by x".to_vec()
        );
        assert_eq!(normalize_datetimes(b"Page 1/10"), b"Page 1/10".to_vec());
        assert_eq!(normalize_datetimes(b"12345:67"), b"12345:67".to_vec());
    }
}
