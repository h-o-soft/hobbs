//! Golden E2E tests for terminal/encoding behavior (Issue #325, plan.md フェーズ0).
//!
//! サーバーが送る「生のバイト列」を記録して比較する。
//! 記録しているのは**現状の挙動**（既知の癖 Q1〜Q10 を含む）であり、正しい挙動ではない。
//! リファクタリング中は `tests/golden/` に差分が出てはならない。
//!
//! 更新: `UPDATE_GOLDEN=1 cargo test --test golden_terminal`

#![cfg(feature = "sqlite")]

mod common;
mod golden_support;

use std::net::SocketAddr;
use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::{timeout, Instant};

use common::{test_config, TestServer};
use golden_support::{assert_golden, escape_bytes, normalize_datetimes};
use hobbs::board::{
    BoardRepository, BoardType, NewBoard, NewThread, NewThreadPost, PostRepository,
    ThreadRepository,
};
use hobbs::db::{NewUser, Role, UserRepository};
use hobbs::server::CharacterEncoding;
use hobbs::Config;

/// Built-in profile names, plus the former c64 variants (now aliases of c64)
/// so that users who saved those names stay covered.
const PROFILES: [&str; 9] = [
    "standard",
    "standard_utf8",
    "dos",
    "c64",
    "c64_petscii",
    "c64_ansi",
    "40col_sjis",
    "jterm40",
    "40col_utf8",
];

/// How long the server must stay silent before a step is considered finished.
const QUIET: Duration = Duration::from_millis(300);
/// For `Until::Prompt`: if output has started and then stays silent this long,
/// the step is finished even without a known prompt suffix (e.g. the English
/// welcome prompt has no trailing separator).
const IDLE_FALLBACK: Duration = Duration::from_millis(1500);
/// Upper bound for one step (Argon2 in debug builds is slow).
const STEP_TIMEOUT: Duration = Duration::from_secs(60);

/// What to wait for after sending a step's input.
#[derive(Clone, Copy)]
enum Until {
    /// Output (since the step started) ends with a typical prompt suffix,
    /// or output has started and then stayed silent for `IDLE_FALLBACK`.
    Prompt,
    /// Output ends with a typical prompt suffix (no idle fallback).
    /// Use for steps that include slow work such as password hashing.
    StrictPrompt,
    /// Output (since the step started) ends with the given bytes.
    EndsWith(&'static [u8]),
    /// The server closes the connection.
    Closed,
}

const PROMPT_SUFFIXES: [&[u8]; 3] = [b": ", b"> ", b"? "];

/// A scripted Telnet session that records everything the server sends.
struct Conn {
    stream: TcpStream,
    transcript: String,
    closed: bool,
}

impl Conn {
    async fn open(addr: SocketAddr) -> Self {
        let stream = TcpStream::connect(addr).await.expect("connect");
        let mut conn = Self {
            stream,
            transcript: String::new(),
            closed: false,
        };
        conn.run("connect", b"", Until::Prompt).await;
        conn
    }

    fn matches(buf: &[u8], until: Until) -> bool {
        match until {
            Until::Prompt | Until::StrictPrompt => PROMPT_SUFFIXES.iter().any(|s| buf.ends_with(s)),
            Until::EndsWith(s) => buf.ends_with(s),
            Until::Closed => false,
        }
    }

    /// Send `input` and record the server's response.
    async fn run(&mut self, label: &str, input: &[u8], until: Until) -> Vec<u8> {
        assert!(!self.closed, "step {label}: connection already closed");
        if !input.is_empty() {
            self.stream.write_all(input).await.expect("write");
            self.stream.flush().await.expect("flush");
        }

        let deadline = Instant::now() + STEP_TIMEOUT;
        let mut received = Vec::new();
        let mut buf = [0u8; 4096];
        loop {
            let satisfied = Self::matches(&received, until);
            let idle_ok = matches!(until, Until::Prompt) && !received.is_empty();
            let remaining = deadline.saturating_duration_since(Instant::now());
            let wait = if satisfied {
                QUIET
            } else if idle_ok {
                IDLE_FALLBACK.min(remaining)
            } else {
                remaining
            };
            if wait.is_zero() {
                panic!(
                    "step {label}: timed out\n--- transcript so far ---\n{}{}",
                    self.transcript,
                    escape_bytes(&received)
                );
            }
            match timeout(wait, self.stream.read(&mut buf)).await {
                Ok(Ok(0)) | Ok(Err(_)) => {
                    self.closed = true;
                    if !matches!(until, Until::Closed) && !satisfied {
                        panic!(
                            "step {label}: connection closed unexpectedly\n--- transcript ---\n{}{}",
                            self.transcript,
                            escape_bytes(&received)
                        );
                    }
                    break;
                }
                Ok(Ok(n)) => received.extend_from_slice(&buf[..n]),
                Err(_) => {
                    if satisfied || (idle_ok && !remaining.is_zero()) {
                        // Quiet period elapsed after the condition was met.
                        break;
                    }
                    panic!(
                        "step {label}: timed out\n--- transcript so far ---\n{}{}",
                        self.transcript,
                        escape_bytes(&received)
                    );
                }
            }
        }

        let normalized = normalize_datetimes(&received);
        self.transcript.push_str(&format!("=== {label}\n"));
        if !input.is_empty() {
            self.transcript
                .push_str(&format!(">>> {}\n", escape_bytes(input).replace('\n', "")));
        }
        self.transcript.push_str("<<<\n");
        self.transcript.push_str(&escape_bytes(&normalized));
        self.transcript.push_str("\n\n");
        received
    }

    /// Like `run`, but asserts that `secret` never appears in the response
    /// (used for password prompts).
    async fn run_secret(&mut self, label: &str, secret: &[u8], until: Until) -> Vec<u8> {
        let mut input = secret.to_vec();
        input.push(b'\r');
        let received = self.run(label, &input, until).await;
        assert!(
            !received.windows(secret.len()).any(|w| w == secret),
            "step {label}: secret input was echoed back\n--- transcript ---\n{}",
            self.transcript
        );
        received
    }
}

/// Test server with fixture data.
struct Fixture {
    server: TestServer,
}

impl Fixture {
    async fn new(config: Config) -> Self {
        let server = TestServer::with_config(config).await.expect("server");
        tokio::time::sleep(Duration::from_millis(100)).await;
        let fixture = Self { server };
        fixture.seed_board().await;
        fixture
    }

    async fn standard() -> Self {
        Self::new(test_config()).await
    }

    fn addr(&self) -> SocketAddr {
        self.server.addr()
    }

    /// Seed one thread board with mixed-width content.
    ///
    /// The content deliberately mixes ASCII, kanji, half-width katakana,
    /// Latin-1 (é), box drawing (─), emoji and a caret-escape color sequence.
    async fn seed_board(&self) {
        let db = self.server.db();
        let author = self
            .create_user("seed", "member", "ja", "standard", "shiftjis")
            .await;
        let board = BoardRepository::new(db.pool())
            .create(
                &NewBoard::new("日本語ボード")
                    .with_description("説明 ─ é ｶﾅ")
                    .with_board_type(BoardType::Thread),
            )
            .await
            .expect("board");
        let threads = ThreadRepository::new(db.pool());
        let posts = PostRepository::new(db.pool());
        // Long thread (for the paging scenario). Created first.
        let long = threads
            .create(&NewThread::new(board.id, "長いスレッド", author))
            .await
            .expect("thread");
        for i in 1..=30 {
            posts
                .create_thread_post(&NewThreadPost::new(
                    board.id,
                    long.id,
                    author,
                    format!("レス{i} 行1\n行2 ─ line {i}"),
                ))
                .await
                .expect("post");
        }
        // Short thread with mixed-width content. Created last.
        let short = threads
            .create(&NewThread::new(board.id, "テスト ─ é ｶﾅ 😀", author))
            .await
            .expect("thread");
        posts
            .create_thread_post(&NewThreadPost::new(
                board.id,
                short.id,
                author,
                "こんにちは ASCII ─ é ｶﾅ 😀\n^[[31m赤^[[0m と ^[[1;33m黄^[[0m\n末尾",
            ))
            .await
            .expect("post");
    }

    async fn create_user(
        &self,
        username: &str,
        role: &str,
        language: &str,
        terminal: &str,
        encoding: &str,
    ) -> i64 {
        let hash = hobbs::hash_password("pass1234").expect("hash");
        let role: Role = role.parse().unwrap_or(Role::Member);
        let encoding: CharacterEncoding = encoding.parse().expect("encoding");
        UserRepository::new(self.server.db().pool())
            .create(
                &NewUser::new(username, &hash, username)
                    .with_role(role)
                    .with_language(language)
                    .with_terminal(terminal)
                    .with_encoding(encoding),
            )
            .await
            .expect("user")
            .id
    }
}

/// Log in from the welcome screen. Returns after the main menu prompt.
async fn login(conn: &mut Conn, username: &str) {
    conn.run("welcome: L", b"L\r", Until::Prompt).await;
    let mut name = username.as_bytes().to_vec();
    name.push(b'\r');
    conn.run("username", &name, Until::Prompt).await;
    conn.run_secret("password", b"pass1234", Until::StrictPrompt)
        .await;
}

// ---------------------------------------------------------------------------
// 条件軸 A: 接続時（default_profile ごとのウェルカム画面）
// ---------------------------------------------------------------------------

async fn scenario_connect(profile: &str) {
    let mut config = test_config();
    config.terminal.default_profile = profile.to_string();
    let fx = Fixture::new(config).await;
    let mut conn = Conn::open(fx.addr()).await;
    conn.run("invalid choice", b"X\r", Until::Prompt).await;
    conn.run("quit", b"Q\r", Until::Closed).await;
    assert_golden(&format!("a_connect__{profile}"), &conn.transcript);
}

#[tokio::test]
async fn golden_a_connect_all_profiles() {
    for profile in PROFILES {
        scenario_connect(profile).await;
    }
}

// ---------------------------------------------------------------------------
// 条件軸 B: ログイン後（users.terminal × users.encoding）
// ---------------------------------------------------------------------------

async fn scenario_login(case: &str, terminal: &str, encoding: &str, language: &str) {
    let fx = Fixture::standard().await;
    fx.create_user("tester", "member", language, terminal, encoding)
        .await;
    let mut conn = Conn::open(fx.addr()).await;
    login(&mut conn, "tester").await;
    conn.run("menu: B (board list)", b"B\r", Until::Prompt)
        .await;
    conn.run("board 1 (thread list)", b"1\r", Until::Prompt)
        .await;
    conn.run("thread 1 (view)", b"1\r", Until::Prompt).await;
    conn.run("back to thread list", b"Q\r", Until::Prompt).await;
    conn.run("back to board list", b"Q\r", Until::Prompt).await;
    conn.run("back to menu", b"Q\r", Until::Prompt).await;
    conn.run("menu: H (help)", b"H\r", Until::Prompt).await;
    conn.run("help: 2 (navigation)", b"2\r", Until::Prompt)
        .await;
    conn.run("help page: Enter", b"\r", Until::Prompt).await;
    conn.run("help: Q (back to menu)", b"Q\r", Until::Prompt)
        .await;
    conn.run("menu: Q (logout)", b"Q\r", Until::Prompt).await;
    assert_golden(&format!("b_login__{case}"), &conn.transcript);
}

#[tokio::test]
async fn golden_b_login_profile_defaults() {
    // Each profile with its own default encoding.
    let cases = [
        ("standard", "shiftjis"),
        ("standard_utf8", "utf8"),
        ("dos", "cp437"),
        ("c64", "petscii"),
        ("c64_petscii", "petscii"),
        ("c64_ansi", "petscii"),
        ("40col_sjis", "shiftjis"),
        ("jterm40", "shiftjis"),
        ("40col_utf8", "utf8"),
    ];
    for (terminal, encoding) in cases {
        scenario_login(terminal, terminal, encoding, "ja").await;
    }
}

#[tokio::test]
async fn golden_b_login_mismatched_and_english() {
    // DB 上で terminal と encoding が食い違っているユーザー（E1）
    scenario_login("standard_with_utf8", "standard", "utf8", "ja").await;
    scenario_login("c64_with_shiftjis", "c64", "shiftjis", "ja").await;
    // 英語ユーザー
    scenario_login("standard_utf8_en", "standard_utf8", "utf8", "en").await;
    scenario_login("dos_en", "dos", "cp437", "en").await;
    // 未知のプロファイル名（Q2: standard 扱い）
    scenario_login("unknown_profile", "no_such_profile", "shiftjis", "ja").await;
}

// ---------------------------------------------------------------------------
// 条件軸 C: 設定画面でプロファイルを変更 → メインメニュー
// ---------------------------------------------------------------------------

async fn scenario_settings(index: usize, profile: &str) {
    let fx = Fixture::standard().await;
    fx.create_user("tester", "member", "ja", "standard", "shiftjis")
        .await;
    let mut conn = Conn::open(fx.addr()).await;
    login(&mut conn, "tester").await;
    conn.run("menu: P (profile)", b"P\r", Until::Prompt).await;
    conn.run("profile: S (settings)", b"S\r", Until::Prompt)
        .await;
    conn.run("language: keep", b"\r", Until::Prompt).await;
    let choice = format!("{}\r", index + 1);
    conn.run("terminal profile", choice.as_bytes(), Until::Prompt)
        .await;
    conn.run(
        "auto paging: keep (saved, back to menu)",
        b"\r",
        Until::Prompt,
    )
    .await;
    conn.run("menu: B (board list)", b"B\r", Until::Prompt)
        .await;
    conn.run("back to menu", b"Q\r", Until::Prompt).await;
    conn.run("logout", b"Q\r", Until::Prompt).await;
    assert_golden(&format!("c_settings__{profile}"), &conn.transcript);
}

/// Order in which the settings screen lists the built-in profiles
/// (`TerminalProfile::available_profiles`).
const SETTINGS_ORDER: [&str; 7] = [
    "standard",
    "standard_utf8",
    "40col_sjis",
    "jterm40",
    "40col_utf8",
    "dos",
    "c64",
];

#[tokio::test]
async fn golden_c_settings_change_profile() {
    for (i, profile) in SETTINGS_ORDER.iter().enumerate() {
        scenario_settings(i, profile).await;
    }
}

// ---------------------------------------------------------------------------
// 条件軸 D: ゲストと登録（言語選択 E/J/U/不正値）
// ---------------------------------------------------------------------------

async fn scenario_guest(choice: &str) {
    let fx = Fixture::standard().await;
    let mut conn = Conn::open(fx.addr()).await;
    conn.run("welcome: G", b"G\r", Until::Prompt).await;
    let input = format!("{choice}\r");
    conn.run("language selection", input.as_bytes(), Until::Prompt)
        .await;
    conn.run("menu: B (board list)", b"B\r", Until::Prompt)
        .await;
    conn.run("board 1 (thread list)", b"1\r", Until::Prompt)
        .await;
    conn.run("back to board list", b"Q\r", Until::Prompt).await;
    conn.run("back to menu", b"Q\r", Until::Prompt).await;
    conn.run("quit", b"Q\r", Until::Closed).await;
    assert_golden(&format!("d_guest__{choice}"), &conn.transcript);
}

#[tokio::test]
async fn golden_d_guest_language_choices() {
    for choice in ["E", "J", "U", "X"] {
        scenario_guest(choice).await;
    }
}

#[tokio::test]
async fn golden_d_register_japanese_sjis() {
    let fx = Fixture::standard().await;
    let mut conn = Conn::open(fx.addr()).await;
    conn.run("welcome: R", b"R\r", Until::Prompt).await;
    conn.run("language selection: J", b"J\r", Until::Prompt)
        .await;
    conn.run("username", b"newuser\r", Until::Prompt).await;
    conn.run_secret("password", b"pass1234", Until::Prompt)
        .await;
    conn.run_secret("password confirm", b"pass1234", Until::Prompt)
        .await;
    conn.run("nickname", b"\r", Until::StrictPrompt).await;
    conn.run("logout", b"Q\r", Until::Prompt).await;
    assert_golden("d_register__J", &conn.transcript);
}

// ---------------------------------------------------------------------------
// 条件軸 E / E2: 入力エコー（SessionHandler 側と ScreenContext 側）
// ---------------------------------------------------------------------------

/// Input sequences sent at a prompt, per wire encoding.
fn echo_inputs(encoding: &str) -> Vec<(&'static str, Vec<u8>)> {
    let multibyte: Vec<u8> = match encoding {
        "shiftjis" => vec![0x82, 0xA0, 0x82, 0xA2], // あい
        "utf8" => "あい".as_bytes().to_vec(),
        "cp437" => vec![0x82, 0x94],   // é ö
        "petscii" => vec![0xC1, 0x61], // shifted A, lowercase a
        _ => unreachable!(),
    };
    let mut bs = multibyte.clone();
    bs.push(0x08);
    bs.push(b'\r');
    let mut del = multibyte.clone();
    del.push(0x7F);
    del.push(b'\r');
    let halfwidth: Vec<u8> = match encoding {
        "shiftjis" => vec![0xB6, 0xC5, 0x08, b'\r'], // ｶﾅ + BS
        "utf8" => {
            let mut v = "ｶﾅ".as_bytes().to_vec();
            v.extend_from_slice(&[0x08, b'\r']);
            v
        }
        _ => vec![b'a', b'b', 0x08, b'\r'],
    };
    vec![
        ("multibyte + BS", bs),
        ("multibyte + DEL", del),
        ("halfwidth + BS", halfwidth),
        // On PETSCII, 0x14 (DEL key) erases a character, so type two
        // characters to keep the line non-empty (an empty line means "back").
        (
            "petscii DEL 0x14",
            if encoding == "petscii" {
                vec![b'a', b'b', 0x14, b'\r']
            } else {
                vec![b'a', 0x14, b'\r']
            },
        ),
        ("ESC [ A (arrow up)", vec![0x1B, b'[', b'A', b'\r']),
    ]
}

/// Inputs that may end the current screen (an empty/cancelled line means
/// "back" on most ScreenContext prompts). Only sent where that is harmless
/// (the main menu); ScreenContext coverage is in `golden_units`.
fn echo_inputs_line_endings() -> Vec<(&'static str, Vec<u8>)> {
    vec![
        // ScreenContext does not strip IAC (Q7); the 0x03 in "WILL SGA" acts as Ctrl+C there.
        (
            "IAC DO/WILL + text",
            vec![0xFF, 0xFD, 0x01, 0xFF, 0xFB, 0x03, b'x', b'\r'],
        ),
        ("Ctrl+C", vec![b'a', 0x03]),
        ("CR LF", vec![b'z', b'\r', b'\n']),
        ("CR NUL", vec![b'z', b'\r', 0x00]),
        ("LF only", vec![b'z', b'\n']),
    ]
}

async fn scenario_echo(terminal: &str, encoding: &str) {
    let fx = Fixture::standard().await;
    fx.create_user("tester", "member", "ja", terminal, encoding)
        .await;
    let mut conn = Conn::open(fx.addr()).await;
    // SessionHandler path (pre-login, always ShiftJIS on the wire).
    conn.run(
        "welcome: multibyte + BS",
        &[0x82, 0xA0, 0x08, b'\r'],
        Until::Prompt,
    )
    .await;
    login(&mut conn, "tester").await;
    // SessionHandler path (main menu, user's encoding).
    for (label, input) in echo_inputs(encoding)
        .into_iter()
        .chain(echo_inputs_line_endings())
    {
        conn.run(&format!("main menu: {label}"), &input, Until::Prompt)
            .await;
    }
    // ScreenContext path (board list prompt).
    conn.run("menu: B (board list)", b"B\r", Until::Prompt)
        .await;
    for (label, input) in echo_inputs(encoding) {
        // A board-list input that is not a number just redraws the list.
        conn.run(&format!("board list: {label}"), &input, Until::Prompt)
            .await;
    }
    conn.run("back to menu", b"Q\r", Until::Prompt).await;
    // ScreenContext path, Password mode (change password: wrong current password).
    conn.run("menu: P (profile)", b"P\r", Until::Prompt).await;
    conn.run("profile: P (change password)", b"P\r", Until::Prompt)
        .await;
    conn.run_secret("current password", b"wrongpw9", Until::Prompt)
        .await;
    conn.run_secret("new password", b"newpass99", Until::Prompt)
        .await;
    conn.run_secret("confirm password", b"newpass99", Until::Prompt)
        .await;
    assert_golden(&format!("e_echo__{terminal}__{encoding}"), &conn.transcript);
}

#[tokio::test]
async fn golden_e_echo_shiftjis() {
    scenario_echo("standard", "shiftjis").await;
    scenario_echo("jterm40", "shiftjis").await;
}

#[tokio::test]
async fn golden_e_echo_utf8() {
    scenario_echo("standard_utf8", "utf8").await;
}

#[tokio::test]
async fn golden_e_echo_cp437() {
    scenario_echo("dos", "cp437").await;
}

#[tokio::test]
async fn golden_e_echo_petscii() {
    scenario_echo("c64", "petscii").await;
}

// ---------------------------------------------------------------------------
// 条件軸 F: 自動ページング
// ---------------------------------------------------------------------------

async fn scenario_paging(terminal: &str, encoding: &str, more_suffix: &'static [u8]) {
    let fx = Fixture::standard().await;
    fx.create_user("tester", "member", "ja", terminal, encoding)
        .await;
    let mut conn = Conn::open(fx.addr()).await;
    login(&mut conn, "tester").await;
    conn.run("menu: B (board list)", b"B\r", Until::Prompt)
        .await;
    conn.run("board 1 (thread list)", b"1\r", Until::Prompt)
        .await;
    conn.run(
        "thread 2 (long, pauses)",
        b"2\r",
        Until::EndsWith(more_suffix),
    )
    .await;
    // Later pauses depend on the page layout; wait for any prompt or silence.
    conn.run("more: Enter", b"\r", Until::Prompt).await;
    conn.run("more: Enter", b"\r", Until::Prompt).await;
    conn.run("more: Enter", b"\r", Until::Prompt).await;
    assert_golden(&format!("f_paging__{terminal}"), &conn.transcript);
}

#[tokio::test]
async fn golden_f_paging() {
    scenario_paging("standard", "shiftjis", b" --").await;
    scenario_paging("40col_utf8", "utf8", b" --").await;
}
