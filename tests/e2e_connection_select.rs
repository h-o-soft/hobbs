#![cfg(feature = "sqlite")]
//! E2E tests for the connection type selection shown right after connecting
//! (plan.md D2/D3, Issue #269).
//!
//! Waits for expected text instead of fixed delays (see #327).

mod common;

use std::time::Duration;

use common::{create_test_user, TestClient, TestServer};
use hobbs::db::{NewUser, UserRepository};
use hobbs::server::CharacterEncoding;

const WAIT: Duration = Duration::from_secs(30);

async fn server() -> TestServer {
    let server = TestServer::new().await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;
    server
}

#[tokio::test]
async fn test_connection_menu_comes_first_and_reprompts() {
    let server = server().await;
    let mut client = TestClient::connect_raw(server.addr()).await.unwrap();

    let menu = client
        .recv_until_timeout("NUMBER [ENTER=1]: ", WAIT)
        .await
        .unwrap();
    assert!(menu.contains("SELECT YOUR TERMINAL"), "{menu}");
    assert!(menu.contains("5) COMMODORE 64 (PETSCII)"), "{menu}");
    // The screen is uppercase ASCII only, readable on a C64 before the
    // encoding is known.
    let text: String = menu.chars().filter(|c| !c.is_control()).collect();
    assert!(
        text.chars()
            .all(|c| c.is_ascii() && !c.is_ascii_lowercase()),
        "{text:?}"
    );

    // Invalid input shows the menu again.
    client.send_line("9").await.unwrap();
    client
        .recv_until_timeout("NUMBER [ENTER=1]: ", WAIT)
        .await
        .unwrap();

    // Enter selects the default (1: Japanese, ShiftJIS) and shows the welcome.
    client.send_line("").await.unwrap();
    client.set_encoding(CharacterEncoding::ShiftJIS);
    let welcome = client.recv_until_timeout("Select:", WAIT).await.unwrap();
    assert!(welcome.contains("[L]"), "{welcome}");
}

/// Issue #269: a user whose saved encoding is PETSCII can log in from a PC.
#[tokio::test]
async fn test_petscii_user_can_log_in_from_pc() {
    let server = server().await;
    let hash = hobbs::hash_password("password123").unwrap();
    UserRepository::new(server.db().pool())
        .create(
            &NewUser::new("c64user", &hash, "c64user")
                .with_language("ja")
                .with_terminal("c64")
                .with_encoding(CharacterEncoding::Petscii),
        )
        .await
        .unwrap();

    // Connect as "2: Japanese, UTF-8".
    let mut client = TestClient::connect_raw(server.addr()).await.unwrap();
    client.select_connection("2").await.unwrap();
    client.recv_until_timeout("Select:", WAIT).await.unwrap();
    client.send_line("L").await.unwrap();
    client.recv_until_timeout("Username:", WAIT).await.unwrap();
    client.send_line("c64user").await.unwrap();
    client.recv_until_timeout("Password:", WAIT).await.unwrap();
    client.send_line("password123").await.unwrap();

    // The main menu arrives in readable UTF-8 Japanese (not PETSCII).
    let menu = client.recv_until_timeout("> ", WAIT).await.unwrap();
    assert!(
        menu.contains("掲示板"),
        "menu should be readable UTF-8: {menu}"
    );
}

#[tokio::test]
async fn test_c64_connection_uses_petscii() {
    let server = server().await;
    create_test_user(server.db(), "someone", "password123", "member")
        .await
        .unwrap();
    let mut client = TestClient::connect_raw(server.addr()).await.unwrap();
    client.select_connection("5").await.unwrap();

    // The welcome screen is sent in PETSCII: CR line ends, no ANSI escapes,
    // and PETSCII color codes instead.
    let raw = client.recv_raw_until(b"> ", WAIT).await.unwrap();
    // Skip the rest of the selection prompt and the echo of "5" + CR LF
    // (echoed before the encoding switched to PETSCII).
    let start = raw.iter().position(|&b| b == b'\n').unwrap() + 1;
    let raw = raw[start..].to_vec();
    assert!(!raw.contains(&0x1B), "no ANSI escapes on PETSCII: {raw:?}");
    assert!(!raw.contains(&b'\n'), "PETSCII uses CR only: {raw:?}");
    assert!(
        raw.windows(2).any(|w| w == b"[L"),
        "welcome prompt present: {raw:?}"
    );
}

#[tokio::test]
async fn test_registration_saves_connection_settings() {
    let server = server().await;
    // First user becomes SysOp; create one so the new user is a member.
    create_test_user(server.db(), "sysop", "password123", "sysop")
        .await
        .unwrap();

    let mut client = TestClient::connect_raw(server.addr()).await.unwrap();
    client.select_connection("5").await.unwrap();
    client.recv_raw_until(b"> ", WAIT).await.unwrap();
    client.send_line("R").await.unwrap();
    client.recv_raw_until(b": ", WAIT).await.unwrap();
    client.send_line("NEWC64").await.unwrap();
    client.recv_raw_until(b": ", WAIT).await.unwrap();
    client.send_line("password123").await.unwrap();
    client.recv_raw_until(b": ", WAIT).await.unwrap();
    client.send_line("password123").await.unwrap();
    client.recv_raw_until(b": ", WAIT).await.unwrap();
    client.send_line("").await.unwrap();
    client.recv_raw_until(b"> ", WAIT).await.unwrap();

    let user = UserRepository::new(server.db().pool())
        .get_by_username("NEWC64")
        .await
        .unwrap()
        .expect("user registered");
    assert_eq!(user.terminal, "c64");
    assert_eq!(user.encoding, CharacterEncoding::Petscii);
    assert_eq!(user.language, "en");
}

/// A settings change that does not select a terminal keeps the connection's
/// encoding (codex review R1-F1): a PETSCII-saved user on a UTF-8 connection
/// who only toggles auto-paging stays on UTF-8.
#[tokio::test]
async fn test_settings_change_keeps_connection_encoding() {
    let server = server().await;
    let hash = hobbs::hash_password("password123").unwrap();
    UserRepository::new(server.db().pool())
        .create(
            &NewUser::new("c64user", &hash, "c64user")
                .with_language("ja")
                .with_terminal("c64")
                .with_encoding(CharacterEncoding::Petscii),
        )
        .await
        .unwrap();

    let mut client = TestClient::connect_raw(server.addr()).await.unwrap();
    client.select_connection("2").await.unwrap();
    client.recv_until_timeout("Select:", WAIT).await.unwrap();
    client.send_line("L").await.unwrap();
    client.recv_until_timeout("Username:", WAIT).await.unwrap();
    client.send_line("c64user").await.unwrap();
    client.recv_until_timeout("Password:", WAIT).await.unwrap();
    client.send_line("password123").await.unwrap();
    client.recv_until_timeout("> ", WAIT).await.unwrap();

    // Profile → Settings: keep language, keep terminal, turn paging off.
    client.send_line("P").await.unwrap();
    client.recv_until_timeout("]=", WAIT).await.unwrap();
    client.recv_until_timeout(": ", WAIT).await.unwrap();
    client.send_line("S").await.unwrap();
    client.recv_until_timeout("]: ", WAIT).await.unwrap(); // language
    client.send_line("").await.unwrap();
    client.recv_until_timeout("]: ", WAIT).await.unwrap(); // terminal / screen
    client.send_line("").await.unwrap();
    client.recv_until_timeout("]: ", WAIT).await.unwrap(); // auto paging
    client.send_line("2").await.unwrap();

    // Back at the main menu, still readable UTF-8 Japanese.
    let menu = client.recv_until_timeout("> ", WAIT).await.unwrap();
    assert!(menu.contains("掲示板"), "menu should stay UTF-8: {menu}");
}

async fn login_raw(client: &mut TestClient, username: &str) {
    client.recv_raw_until(b"> ", WAIT).await.unwrap();
    client.send_line("L").await.unwrap();
    client.recv_raw_until(b": ", WAIT).await.unwrap();
    client.send_line(username).await.unwrap();
    client.recv_raw_until(b": ", WAIT).await.unwrap();
    client.send_line("password123").await.unwrap();
    client.recv_raw_until(b"> ", WAIT).await.unwrap();
}

/// Profile → Settings, answering each prompt in order.
async fn change_settings(client: &mut TestClient, language: &str, terminal: &str, paging: &str) {
    client.send_line("P").await.unwrap();
    client.recv_raw_until(b": ", WAIT).await.unwrap();
    client.send_line("S").await.unwrap();
    client.recv_raw_until(b"]: ", WAIT).await.unwrap();
    client.send_line(language).await.unwrap();
    client.recv_raw_until(b"]: ", WAIT).await.unwrap();
    client.send_line(terminal).await.unwrap();
    client.recv_raw_until(b"]: ", WAIT).await.unwrap();
    client.send_line(paging).await.unwrap();
}

/// A Japanese account on a CP437 connection stays in English after an
/// unrelated settings change (codex review R2-F1).
#[tokio::test]
async fn test_settings_change_keeps_language_compatible() {
    let server = server().await;
    let hash = hobbs::hash_password("password123").unwrap();
    UserRepository::new(server.db().pool())
        .create(
            &NewUser::new("jauser", &hash, "jauser")
                .with_language("ja")
                .with_encoding(CharacterEncoding::ShiftJIS),
        )
        .await
        .unwrap();

    let mut client = TestClient::connect_raw(server.addr()).await.unwrap();
    client.select_connection("4").await.unwrap();
    login_raw(&mut client, "jauser").await;
    change_settings(&mut client, "", "", "2").await;

    let menu = client.recv_raw_until(b"> ", WAIT).await.unwrap();
    let text = String::from_utf8_lossy(&menu);
    assert!(text.contains("Boards"), "menu should stay English: {text}");
}

/// Selecting a terminal that equals the saved name still applies it when
/// login had substituted another profile (codex review R2-F2): a C64-saved
/// user on a UTF-8 connection who picks C64 gets PETSCII output.
#[tokio::test]
async fn test_settings_selecting_saved_terminal_applies_it() {
    let server = server().await;
    let hash = hobbs::hash_password("password123").unwrap();
    UserRepository::new(server.db().pool())
        .create(
            &NewUser::new("c64user", &hash, "c64user")
                .with_language("en")
                .with_terminal("c64")
                .with_encoding(CharacterEncoding::Petscii),
        )
        .await
        .unwrap();

    let mut client = TestClient::connect_raw(server.addr()).await.unwrap();
    client.select_connection("3").await.unwrap();
    login_raw(&mut client, "c64user").await;
    // Built-in list order: standard, standard_utf8, 40col_sjis, jterm40,
    // 40col_utf8, dos, c64.
    change_settings(&mut client, "", "7", "").await;

    let raw = client.recv_raw_until(b"> ", WAIT).await.unwrap();
    // Skip everything up to "Settings saved" (sent before the switch).
    let marker = b"Settings saved\r\n";
    let start = raw
        .windows(marker.len())
        .position(|w| w == marker)
        .expect("settings saved message")
        + marker.len();
    let after = &raw[start..];
    assert!(!after.is_empty());
    assert!(
        !after.contains(&0x1B),
        "no ANSI escapes after switching to C64"
    );
    assert!(!after.contains(&b'\n'), "PETSCII uses CR only");
    // The C64 layout (40 columns) is applied, not the 80-column PC one that
    // login had substituted: the menu's "=" separator is 40 wide.
    let longest_rule = after
        .split(|&b| b != b'=')
        .map(|run| run.len())
        .max()
        .unwrap_or(0);
    assert_eq!(longest_rule, 40, "40-column menu expected");
}
