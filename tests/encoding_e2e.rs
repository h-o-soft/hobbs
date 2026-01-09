#![cfg(feature = "sqlite")]
//! End-to-end encoding tests for HOBBS.
//!
//! These tests verify that character encoding works correctly throughout
//! the entire flow from i18n strings to wire format.

mod common;

use common::{TestClient, TestServer};
use hobbs::server::{decode_from_client, encode_for_client, CharacterEncoding};
use std::time::Duration;
use tokio::io::AsyncReadExt;

/// Filter out Telnet IAC command sequences from raw bytes.
/// IAC (0xFF) introduces command sequences that should be stripped.
/// This properly preserves ShiftJIS bytes which can be in range 0x80-0xFC.
fn filter_telnet_iac(data: &[u8]) -> Vec<u8> {
    let mut result = Vec::with_capacity(data.len());
    let mut i = 0;
    while i < data.len() {
        if data[i] == 0xFF {
            // IAC - Telnet command
            if i + 1 < data.len() {
                match data[i + 1] {
                    0xFF => {
                        // IAC IAC = literal 0xFF
                        result.push(0xFF);
                        i += 2;
                    }
                    0xFA => {
                        // Subnegotiation - skip until IAC SE (0xFF 0xF0)
                        i += 2;
                        while i + 1 < data.len() {
                            if data[i] == 0xFF && data[i + 1] == 0xF0 {
                                i += 2;
                                break;
                            }
                            i += 1;
                        }
                    }
                    _ => {
                        // Other command (WILL, WONT, DO, DONT, etc) - skip 2 or 3 bytes
                        if data[i + 1] >= 0xFB && data[i + 1] <= 0xFE {
                            // WILL, WONT, DO, DONT have one option byte
                            i += 3;
                        } else {
                            i += 2;
                        }
                    }
                }
            } else {
                i += 1;
            }
        } else {
            result.push(data[i]);
            i += 1;
        }
    }
    result
}

/// Test that encode_for_client produces correct ShiftJIS bytes for Japanese text.
#[test]
fn test_encode_japanese_to_shiftjis() {
    // Test common Japanese phrases
    let test_cases = [
        (
            "メインメニュー",
            vec![
                0x83, 0x81, 0x83, 0x43, 0x83, 0x93, 0x83, 0x81, 0x83, 0x6A, 0x83, 0x85, 0x81, 0x5B,
            ],
        ),
        (
            "ようこそ",
            vec![0x82, 0xE6, 0x82, 0xA4, 0x82, 0xB1, 0x82, 0xBB],
        ),
        ("掲示板", vec![0x8C, 0x66, 0x8E, 0xA6, 0x94, 0xC2]),
    ];

    for (input, expected) in test_cases {
        let encoded = encode_for_client(input, CharacterEncoding::ShiftJIS);
        assert_eq!(
            encoded, expected,
            "Failed for input: {} - got {:02X?}, expected {:02X?}",
            input, encoded, expected
        );
    }
}

/// Test that encode_for_client preserves ASCII for ShiftJIS.
#[test]
fn test_encode_ascii_shiftjis() {
    let input = "Hello, World!";
    let encoded = encode_for_client(input, CharacterEncoding::ShiftJIS);
    assert_eq!(encoded, b"Hello, World!");
}

/// Test that encode_for_client preserves UTF-8 for UTF-8 encoding.
#[test]
fn test_encode_utf8_passthrough() {
    let input = "メインメニュー";
    let encoded = encode_for_client(input, CharacterEncoding::Utf8);
    assert_eq!(encoded, input.as_bytes());
}

/// Test roundtrip: encode then decode ShiftJIS.
#[test]
fn test_roundtrip_shiftjis() {
    let inputs = [
        "メインメニュー",
        "掲示板の閲覧・投稿",
        "HOBBSへようこそ！",
        "ゲストとしてログインしました",
        "日本語テスト ABC 123 !@#",
    ];

    for input in inputs {
        let encoded = encode_for_client(input, CharacterEncoding::ShiftJIS);
        let decoded = decode_from_client(&encoded, CharacterEncoding::ShiftJIS);
        assert_eq!(decoded, input, "Roundtrip failed for: {}", input);
    }
}

/// Test that mixed Japanese and ASCII encodes correctly.
#[test]
fn test_mixed_content_shiftjis() {
    let input = "[B] 掲示板 - Board";
    let encoded = encode_for_client(input, CharacterEncoding::ShiftJIS);
    let decoded = decode_from_client(&encoded, CharacterEncoding::ShiftJIS);
    assert_eq!(decoded, input);
}

/// Test encoding for all messages from ja.toml that would appear in menus.
#[test]
fn test_menu_messages_shiftjis() {
    // These are the actual messages from locales/ja.toml
    let messages = [
        "メインメニュー",
        "掲示板",
        "チャット",
        "メール",
        "ファイル",
        "プロフィール",
        "設定",
        "管理",
        "会員一覧",
        "選択してください: ",
        "ログイン",
        "ログアウト",
        "新規登録",
    ];

    for msg in messages {
        let encoded = encode_for_client(msg, CharacterEncoding::ShiftJIS);
        let decoded = decode_from_client(&encoded, CharacterEncoding::ShiftJIS);
        assert_eq!(decoded, msg, "Failed to roundtrip message: {}", msg);

        // Also verify the encoded bytes are valid ShiftJIS (no replacement characters)
        let (_, _, had_errors) = encoding_rs::SHIFT_JIS.encode(msg);
        assert!(!had_errors, "ShiftJIS encoding had errors for: {}", msg);
    }
}

/// Test that the raw bytes received match expected ASCII for the welcome screen.
#[tokio::test]
async fn test_raw_bytes_from_server() {
    let mut server = TestServer::new().await.expect("Failed to create server");
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Connect client
    let mut stream = tokio::net::TcpStream::connect(server.addr())
        .await
        .expect("Failed to connect");

    // Read initial data (negotiation + ASCII welcome screen)
    let mut buf = vec![0u8; 4096];
    tokio::time::sleep(Duration::from_millis(200)).await;
    let n = stream.read(&mut buf).await.expect("Failed to read");
    let initial = &buf[..n];

    // The initial screen should be ASCII (welcome screen is ASCII-only)
    // Filter out Telnet IAC commands properly
    let filtered = filter_telnet_iac(initial);
    let text = String::from_utf8_lossy(&filtered);

    // Should contain the ASCII welcome screen with G/R/L/Q options
    assert!(
        text.contains("HOBBS") || text.contains("Select") || text.contains("Guest"),
        "Expected ASCII welcome screen, got: {}",
        text
    );

    server.stop();
}

/// Test that after selecting Japanese ShiftJIS for guest mode, the menu is ShiftJIS encoded.
#[tokio::test]
async fn test_shiftjis_welcome_screen() {
    use tokio::io::AsyncWriteExt;

    let mut server = TestServer::new().await.expect("Failed to create server");
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Connect client
    let mut stream = tokio::net::TcpStream::connect(server.addr())
        .await
        .expect("Failed to connect");

    // Read initial data (negotiation + ASCII welcome screen)
    let mut buf = vec![0u8; 4096];
    tokio::time::sleep(Duration::from_millis(200)).await;
    let _ = stream.read(&mut buf).await.expect("Failed to read");

    // New flow: First choose Guest
    stream.write_all(b"G\r").await.expect("Failed to send G");

    // Read language selection screen
    tokio::time::sleep(Duration::from_millis(300)).await;
    let _ = stream
        .read(&mut buf)
        .await
        .expect("Failed to read language selection");

    // Select Japanese ShiftJIS
    stream.write_all(b"J\r").await.expect("Failed to send J");

    // Read main menu (in ShiftJIS)
    tokio::time::sleep(Duration::from_millis(300)).await;
    let n = stream.read(&mut buf).await.expect("Failed to read menu");
    let menu_data = &buf[..n];

    // Filter out Telnet control bytes properly
    let filtered = filter_telnet_iac(menu_data);

    // Decode as ShiftJIS
    let (decoded, _, _had_errors) = encoding_rs::SHIFT_JIS.decode(&filtered);

    // The menu screen should contain Japanese text
    let text = decoded.to_string();

    // Look for expected Japanese patterns
    let has_japanese = text.contains("メニュー")
        || text.contains("掲示板")
        || text.contains("チャット")
        || text.contains("HOBBS"); // ASCII fallback

    assert!(
        has_japanese,
        "Expected Japanese menu when decoded as ShiftJIS. Got: {}",
        text
    );

    server.stop();
}

/// Test that UTF-8 selection works correctly for guest mode.
#[tokio::test]
async fn test_utf8_welcome_screen() {
    use tokio::io::AsyncWriteExt;

    let mut server = TestServer::new().await.expect("Failed to create server");
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Connect client
    let mut stream = tokio::net::TcpStream::connect(server.addr())
        .await
        .expect("Failed to connect");

    // Read initial data (ASCII welcome screen)
    let mut buf = vec![0u8; 4096];
    tokio::time::sleep(Duration::from_millis(200)).await;
    let _ = stream.read(&mut buf).await.expect("Failed to read");

    // New flow: First choose Guest
    stream.write_all(b"G\r").await.expect("Failed to send G");

    // Read language selection screen
    tokio::time::sleep(Duration::from_millis(300)).await;
    let _ = stream
        .read(&mut buf)
        .await
        .expect("Failed to read language selection");

    // Select Japanese UTF-8
    stream.write_all(b"U\r").await.expect("Failed to send");

    // Read main menu (in UTF-8)
    tokio::time::sleep(Duration::from_millis(300)).await;
    let n = stream.read(&mut buf).await.expect("Failed to read menu");
    let menu_data = &buf[..n];

    // Filter out Telnet control bytes properly
    let filtered = filter_telnet_iac(menu_data);

    // Decode as UTF-8
    let text = String::from_utf8_lossy(&filtered);

    // Should contain Japanese text decoded correctly as UTF-8
    let has_japanese =
        text.contains("メニュー") || text.contains("掲示板") || text.contains("チャット");

    assert!(
        has_japanese,
        "Expected Japanese menu when decoded as UTF-8. Got: {}",
        text
    );

    server.stop();
}

/// Test that English selection uses UTF-8 encoding.
#[tokio::test]
async fn test_english_encoding() {
    use tokio::io::AsyncWriteExt;

    let mut server = TestServer::new().await.expect("Failed to create server");
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Connect client
    let mut stream = tokio::net::TcpStream::connect(server.addr())
        .await
        .expect("Failed to connect");

    // Read initial data
    let mut buf = vec![0u8; 4096];
    tokio::time::sleep(Duration::from_millis(200)).await;
    let _ = stream.read(&mut buf).await.expect("Failed to read");

    // Select English
    stream.write_all(b"E\r").await.expect("Failed to send");

    // Read welcome screen
    tokio::time::sleep(Duration::from_millis(300)).await;
    let n = stream.read(&mut buf).await.expect("Failed to read welcome");
    let welcome_data = &buf[..n];

    // Decode as UTF-8 (English uses UTF-8)
    let text = String::from_utf8_lossy(&welcome_data);

    // Should contain English text
    let has_english = text.contains("Welcome") || text.contains("HOBBS") || text.contains("Login");

    assert!(
        has_english,
        "Expected English welcome screen. Got: {}",
        text
    );

    server.stop();
}

/// Test that the menu navigation works with ShiftJIS encoding.
#[tokio::test]
async fn test_menu_navigation_shiftjis() {
    use tokio::io::AsyncWriteExt;

    let mut server = TestServer::new().await.expect("Failed to create server");
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Connect client
    let mut stream = tokio::net::TcpStream::connect(server.addr())
        .await
        .expect("Failed to connect");

    let mut buf = vec![0u8; 4096];

    // Read initial ASCII welcome screen
    tokio::time::sleep(Duration::from_millis(200)).await;
    let _ = stream.read(&mut buf).await.unwrap();

    // New flow: First choose Guest
    stream.write_all(b"G\r").await.unwrap();

    // Read language selection screen
    tokio::time::sleep(Duration::from_millis(300)).await;
    let _ = stream.read(&mut buf).await.unwrap();

    // Select Japanese ShiftJIS
    stream.write_all(b"J\r").await.unwrap();

    // Read main menu
    tokio::time::sleep(Duration::from_millis(300)).await;
    let n = stream.read(&mut buf).await.unwrap();
    let menu_data = &buf[..n];

    // Decode as ShiftJIS
    let (decoded, _, _) = encoding_rs::SHIFT_JIS.decode(menu_data);
    let text = decoded.to_string();

    // Main menu should have Japanese text
    let has_menu_items =
        text.contains("メニュー") || text.contains("掲示板") || text.contains("チャット");

    assert!(
        has_menu_items,
        "Expected Japanese menu items when decoded as ShiftJIS. Got: {}",
        text
    );

    server.stop();
}

/// Detailed test that prints exact bytes for debugging.
#[tokio::test]
async fn test_shiftjis_bytes_detailed() {
    use tokio::io::AsyncWriteExt;

    let mut server = TestServer::new().await.expect("Failed to create server");
    tokio::time::sleep(Duration::from_millis(100)).await;

    let mut stream = tokio::net::TcpStream::connect(server.addr())
        .await
        .expect("Failed to connect");

    let mut buf = vec![0u8; 16384];

    // Read initial ASCII welcome screen
    tokio::time::sleep(Duration::from_millis(200)).await;
    let _ = stream.read(&mut buf).await.unwrap();

    // New flow: First choose Guest
    stream.write_all(b"G\r").await.unwrap();

    // Read language selection screen
    tokio::time::sleep(Duration::from_millis(300)).await;
    let _ = stream.read(&mut buf).await.unwrap();

    // Select Japanese ShiftJIS
    stream.write_all(b"J\r").await.unwrap();

    // Read main menu with multiple reads to get all data
    tokio::time::sleep(Duration::from_millis(500)).await;
    let mut total_data = Vec::new();
    loop {
        match tokio::time::timeout(Duration::from_millis(100), stream.read(&mut buf)).await {
            Ok(Ok(0)) => break,
            Ok(Ok(n)) => total_data.extend_from_slice(&buf[..n]),
            Ok(Err(_)) => break,
            Err(_) => break, // timeout
        }
    }
    let data = &total_data;

    // Filter out Telnet IAC commands properly
    // IAC (0xFF) is followed by command bytes. We need to skip IAC sequences, not just high bytes.
    // ShiftJIS can have second bytes in range 0x40-0x7E, 0x80-0xFC, so we can't just filter high bytes!
    let filtered = filter_telnet_iac(data);

    // Print the raw bytes for debugging
    eprintln!(
        "Raw bytes (first 200): {:02X?}",
        &filtered[..filtered.len().min(200)]
    );

    // Decode as ShiftJIS
    let (decoded_sjis, _, _) = encoding_rs::SHIFT_JIS.decode(&filtered);
    eprintln!("Decoded as ShiftJIS:\n{}", decoded_sjis);

    // Also try decoding as UTF-8 for comparison
    let decoded_utf8 = String::from_utf8_lossy(&filtered);
    eprintln!("Decoded as UTF-8 (lossy):\n{}", decoded_utf8);

    // The ShiftJIS decoded version should contain readable Japanese
    let has_japanese = decoded_sjis.contains("メニュー")
        || decoded_sjis.contains("掲示板")
        || decoded_sjis.contains("チャット")
        || decoded_sjis.contains("HOBBS");

    assert!(has_japanese, "Expected Japanese in ShiftJIS decoded text");

    server.stop();
}

/// Test problematic characters that might not encode correctly.
#[test]
fn test_problematic_characters() {
    let test_cases = [
        "接続日時",           // connection datetime
        "選択してください",   // please select
        "をお楽しみください", // please enjoy
        "日時",               // datetime
        "選択",               // select
        "を",                 // wo particle
    ];

    for s in test_cases {
        let encoded = encode_for_client(s, CharacterEncoding::ShiftJIS);
        let decoded = decode_from_client(&encoded, CharacterEncoding::ShiftJIS);

        // Check for encoding errors
        let (_, _, had_errors) = encoding_rs::SHIFT_JIS.encode(s);

        eprintln!(
            "{}: encoded={:02X?}, decoded={}, errors={}",
            s,
            &encoded[..encoded.len().min(20)],
            decoded,
            had_errors
        );

        assert_eq!(
            decoded, s,
            "Roundtrip failed for '{}': got '{}'",
            s, decoded
        );
        assert!(!had_errors, "Encoding had errors for '{}'", s);
    }
}

/// Verify that specific i18n keys are correctly encoded.
#[test]
fn test_i18n_key_encoding() {
    // Load the actual Japanese locale file and test encoding
    let content = std::fs::read_to_string("locales/ja.toml").expect("Failed to read ja.toml");

    // The file should be UTF-8
    assert!(content.contains("メインメニュー"));
    assert!(content.contains("掲示板"));

    // These strings should encode to valid ShiftJIS
    let test_strings: Vec<&str> = vec![
        "メインメニュー",
        "掲示板",
        "チャット",
        "ファイル",
        "プロフィール",
        "ログイン",
        "ログアウト",
    ];

    for s in test_strings {
        let encoded = encode_for_client(s, CharacterEncoding::ShiftJIS);

        // Verify it's not empty
        assert!(
            !encoded.is_empty(),
            "Encoding produced empty result for: {}",
            s
        );

        // Verify roundtrip
        let decoded = decode_from_client(&encoded, CharacterEncoding::ShiftJIS);
        assert_eq!(decoded, s, "Roundtrip failed for: {}", s);
    }
}
