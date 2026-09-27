//! Golden unit-level characterization tests (Issue #325, plan.md フェーズ0).
//!
//! E2E (golden_terminal.rs) では到達しにくい経路を、関数を直接呼んで固定する。
//! - 入力エコー: `LineBuffer`（SessionHandler はこのエコーをそのまま書く）と、
//!   `ScreenContext` の `read_line` / `read_line_nonblocking`（→ `finish_line_reading`）
//!   を、エコーモード（Normal / Password / Masked）× 文字コードごとに。
//! - 幅の計算: 4か所の実装（TerminalProfile / template / word_wrap / char_wrap）。
//! - 出力の変換: `convert_caret_escape` → `process_output_mode` → `encode_for_client`。
//!
//! 記録しているのは**現状の挙動**。更新: `UPDATE_GOLDEN=1 cargo test --test golden_units`

#![cfg(feature = "sqlite")]

mod common;
mod golden_support;

use std::fmt::Write as _;
use std::sync::Arc;
use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::time::timeout;

use common::test_config;
use golden_support::{assert_golden, escape_bytes};
use hobbs::app::screens::ScreenContext;
use hobbs::chat::ChatRoomManager;
use hobbs::server::{
    convert_caret_escape, encode_for_client, process_output_mode, CharacterEncoding, EchoMode,
    LineBuffer, OutputMode, SessionManager,
};
use hobbs::template::{display_width, truncate_to_width};
use hobbs::terminal::TerminalProfile;
use hobbs::{Database, I18nManager, RateLimiters, TelnetSession, TemplateLoader};

const ENCODINGS: [CharacterEncoding; 4] = [
    CharacterEncoding::ShiftJIS,
    CharacterEncoding::Utf8,
    CharacterEncoding::Cp437,
    CharacterEncoding::Petscii,
];

const ECHO_MODES: [EchoMode; 3] = [EchoMode::Normal, EchoMode::Password, EchoMode::Masked('*')];

fn mode_name(mode: EchoMode) -> String {
    match mode {
        EchoMode::Normal => "normal".to_string(),
        EchoMode::Password => "password".to_string(),
        EchoMode::Masked(c) => format!("masked({c})"),
    }
}

fn enc_name(enc: CharacterEncoding) -> &'static str {
    match enc {
        CharacterEncoding::ShiftJIS => "shiftjis",
        CharacterEncoding::Utf8 => "utf8",
        CharacterEncoding::Cp437 => "cp437",
        CharacterEncoding::Petscii => "petscii",
    }
}

/// Input byte sequences, each ending with a line terminator or a cancel.
fn input_cases(enc: CharacterEncoding) -> Vec<(&'static str, Vec<u8>)> {
    let wide: Vec<u8> = match enc {
        CharacterEncoding::ShiftJIS => vec![0x82, 0xA0, 0x82, 0xA2], // あい
        CharacterEncoding::Utf8 => "あい".as_bytes().to_vec(),
        CharacterEncoding::Cp437 => vec![0x82, 0x94], // é ö
        CharacterEncoding::Petscii => vec![0xC1, 0x61],
    };
    let half: Vec<u8> = match enc {
        CharacterEncoding::ShiftJIS => vec![0xB6, 0xC5], // ｶﾅ
        CharacterEncoding::Utf8 => "ｶﾅ".as_bytes().to_vec(),
        _ => b"ab".to_vec(),
    };
    let with = |base: &[u8], tail: &[u8]| {
        let mut v = base.to_vec();
        v.extend_from_slice(tail);
        v
    };
    vec![
        ("ascii", b"abc\r".to_vec()),
        ("wide", with(&wide, b"\r")),
        ("wide + BS", with(&wide, &[0x08, b'\r'])),
        ("wide + DEL", with(&wide, &[0x7F, b'\r'])),
        ("wide + BS BS BS", with(&wide, &[0x08, 0x08, 0x08, b'\r'])),
        ("half + BS", with(&half, &[0x08, b'\r'])),
        ("petscii DEL 0x14", vec![b'a', 0x14, b'\r']),
        ("ESC [ A", vec![b'a', 0x1B, b'[', b'A', b'\r']),
        ("IAC WILL SGA", vec![0xFF, 0xFB, 0x03, b'x', b'\r']),
        ("IAC DO ECHO", vec![0xFF, 0xFD, 0x01, b'x', b'\r']),
        ("Ctrl+C", vec![b'a', 0x03]),
        ("LF only", vec![b'z', b'\n']),
        ("CR LF", vec![b'z', b'\r', b'\n']),
        ("CR NUL", vec![b'z', b'\r', 0x00]),
        ("incomplete lead byte + CR", with(&wide[..1], b"\r")),
    ]
}

// ---------------------------------------------------------------------------
// LineBuffer (SessionHandler writes these echo bytes verbatim)
// ---------------------------------------------------------------------------

#[test]
fn golden_line_buffer_echo() {
    let mut out = String::new();
    for enc in ENCODINGS {
        for mode in ECHO_MODES {
            for (label, input) in input_cases(enc) {
                let mut lb = LineBuffer::with_encoding(1024, enc);
                lb.set_echo_mode(mode);
                let mut echo = Vec::new();
                let mut results = Vec::new();
                for &b in &input {
                    let (res, e) = lb.process_byte(b);
                    echo.extend_from_slice(&e);
                    results.push(format!("{res:?}"));
                }
                let _ = writeln!(
                    out,
                    "[{}][{}] {}\n  in:   {}\n  echo: {}\n  res:  {}",
                    enc_name(enc),
                    mode_name(mode),
                    label,
                    escape_bytes(&input),
                    escape_bytes(&echo).replace('\n', ""),
                    results.join(" ")
                );
            }
        }
    }
    assert_golden("u_line_buffer_echo", &out);
}

// ---------------------------------------------------------------------------
// ScreenContext read paths (read_line / read_line_nonblocking)
// ---------------------------------------------------------------------------

struct Shared {
    db: Arc<Database>,
    config: Arc<hobbs::Config>,
    templates: Arc<TemplateLoader>,
    i18n: Arc<hobbs::I18n>,
    chat: Arc<ChatRoomManager>,
    sessions: Arc<SessionManager>,
    limiters: Arc<RateLimiters>,
}

impl Shared {
    async fn new() -> Self {
        let i18n_manager = I18nManager::load_all("locales").expect("i18n");
        Self {
            db: Arc::new(Database::open_in_memory().await.expect("db")),
            config: Arc::new(test_config()),
            templates: Arc::new(TemplateLoader::new("templates")),
            i18n: Arc::new(i18n_manager.get("ja").expect("ja").clone()),
            chat: Arc::new(ChatRoomManager::with_defaults().await),
            sessions: Arc::new(SessionManager::new(300)),
            limiters: Arc::new(RateLimiters::new()),
        }
    }

    fn ctx(&self, profile: TerminalProfile, enc: CharacterEncoding) -> ScreenContext {
        ScreenContext::new(
            Arc::clone(&self.db),
            Arc::clone(&self.config),
            Arc::clone(&self.templates),
            profile,
            Arc::clone(&self.i18n),
            enc,
            Arc::clone(&self.chat),
            Arc::clone(&self.sessions),
            Arc::clone(&self.limiters),
        )
    }
}

/// Create a connected (server session, client stream) pair over loopback.
async fn session_pair() -> (TelnetSession, TcpStream) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let client = TcpStream::connect(addr).await.unwrap();
    let (server, peer) = listener.accept().await.unwrap();
    (TelnetSession::new(server, peer), client)
}

/// Read everything the server has written so far.
async fn drain(client: &mut TcpStream) -> Vec<u8> {
    let mut out = Vec::new();
    let mut buf = [0u8; 1024];
    while let Ok(Ok(n)) = timeout(Duration::from_millis(100), client.read(&mut buf)).await {
        if n == 0 {
            break;
        }
        out.extend_from_slice(&buf[..n]);
    }
    out
}

#[derive(Clone, Copy)]
enum ReadPath {
    ReadLine,
    NonBlocking,
}

async fn screen_read_golden(path: ReadPath, name: &str) {
    let shared = Shared::new().await;
    let mut out = String::new();
    for enc in ENCODINGS {
        for mode in ECHO_MODES {
            for (label, input) in input_cases(enc) {
                let (mut session, mut client) = session_pair().await;
                session.set_encoding(enc);
                let mut ctx = shared.ctx(TerminalProfile::standard(), enc);
                ctx.set_echo_mode(mode);
                client.write_all(&input).await.unwrap();
                client.flush().await.unwrap();
                let result = match path {
                    ReadPath::ReadLine => format!("{:?}", ctx.read_line(&mut session).await),
                    ReadPath::NonBlocking => {
                        format!("{:?}", ctx.read_line_nonblocking(&mut session, 1000).await)
                    }
                };
                let echo = drain(&mut client).await;
                let _ = writeln!(
                    out,
                    "[{}][{}] {}\n  in:   {}\n  echo: {}\n  ret:  {}",
                    enc_name(enc),
                    mode_name(mode),
                    label,
                    escape_bytes(&input),
                    escape_bytes(&echo).replace('\n', ""),
                    result
                );
                if matches!(mode, EchoMode::Password | EchoMode::Masked(_)) {
                    // Secret bytes (anything but controls) must never be echoed.
                    let printable: Vec<u8> = input
                        .iter()
                        .copied()
                        .filter(|b| b.is_ascii_alphanumeric())
                        .collect();
                    for b in printable {
                        assert!(
                            !echo.contains(&b),
                            "{name}: [{}][{}] {label}: secret byte {b:#04x} echoed: {}",
                            enc_name(enc),
                            mode_name(mode),
                            escape_bytes(&echo)
                        );
                    }
                }
            }
        }
    }
    assert_golden(name, &out);
}

#[tokio::test]
async fn golden_screen_read_line_echo() {
    screen_read_golden(ReadPath::ReadLine, "u_screen_read_line_echo").await;
}

#[tokio::test]
async fn golden_screen_read_line_nonblocking_echo() {
    screen_read_golden(ReadPath::NonBlocking, "u_screen_read_line_nonblocking_echo").await;
}

// ---------------------------------------------------------------------------
// Width calculation (4 implementations must agree today)
// ---------------------------------------------------------------------------

const WIDTH_CORPUS: [&str; 12] = [
    "",
    "Hello",
    "こんにちは",
    "ｶﾀｶﾅ",
    "café",
    "─┼─",
    "😀👍",
    "e\u{0301}",
    "ABCあいうDEF",
    "  spaces  ",
    "\u{1b}[31m赤\u{1b}[0m",
    "タブ\tあり",
];

#[tokio::test]
async fn golden_width_functions() {
    let shared = Shared::new().await;
    let mut out = String::new();
    for cjk in [1u8, 2u8] {
        let mut profile = TerminalProfile::standard();
        profile.cjk_width = cjk;
        for s in WIDTH_CORPUS {
            let w_profile = profile.display_width(s);
            let w_template = display_width(s, cjk as usize);
            assert_eq!(
                w_profile, w_template,
                "display_width differs (cjk={cjk}) for {s:?}"
            );
            let mut truncs = Vec::new();
            for max in [0usize, 1, 2, 3, 5, 8] {
                let t_profile = profile.truncate_to_width(s, max);
                let t_template = truncate_to_width(s, max, cjk as usize);
                assert_eq!(
                    t_profile, t_template,
                    "truncate_to_width differs (cjk={cjk}, max={max}) for {s:?}"
                );
                truncs.push(format!("{max}:{t_profile:?}"));
            }
            let pad = profile.pad_to_width(s, 8);
            let _ = writeln!(
                out,
                "cjk={cjk} {s:?} width={w_profile} pad8={pad:?} trunc=[{}]",
                truncs.join(" ")
            );
        }
    }

    // word_wrap / char_wrap (ScreenContext) at 40 and 80 columns, cjk 1 and 2.
    let wrap_corpus = [
        "short line",
        "The quick brown fox jumps over the lazy dog and keeps running far away",
        "日本語の長い文章を折り返すテストです。全角文字は一文字ずつ折り返せる。まだ続きますよ。",
        "混在 mixed テキスト with ASCII words と 日本語 が 交互に 並ぶ 文章 です よ",
        "Averyveryveryverylongwordwithoutanyspacesthatexceedsthewidthofthescreenforsure",
        "ｶﾀｶﾅﾊﾝｶｸﾓｼﾞﾚﾂﾃﾞｽ ｶﾀｶﾅﾊﾝｶｸﾓｼﾞﾚﾂﾃﾞｽ ｶﾀｶﾅﾊﾝｶｸﾓｼﾞﾚﾂﾃﾞｽ",
        "line1\nline2 は 改行を含む\n\nline4",
    ];
    for (base, width) in [
        (TerminalProfile::standard(), 80u16),
        (TerminalProfile::col40_sjis(), 40),
        (TerminalProfile::jterm40(), 40),
        (TerminalProfile::c64(), 40),
    ] {
        let mut profile = base.clone();
        profile.width = width;
        let ctx = shared.ctx(profile.clone(), profile.encoding);
        for text in wrap_corpus {
            let _ = writeln!(
                out,
                "wrap[{} w={} cjk={}] {:?}\n  => {:?}",
                profile.name,
                profile.width,
                profile.cjk_width,
                text,
                ctx.word_wrap(text)
            );
        }
    }
    assert_golden("u_width_functions", &out);
}

// ---------------------------------------------------------------------------
// Output conversion chain (caret escape → output mode → encoding)
// ---------------------------------------------------------------------------

#[test]
fn golden_output_conversion() {
    let corpus = [
        "ASCII only\r\n",
        "日本語テキスト\r\n",
        "^[[31m赤^[[0m ^[[1;33m黄^[[0m ^[[5;46mblink^[[0m\r\n",
        "^[[2J^[[H^[[3A^[[10C^[[7mRVS^[[0m\r\n",
        "é ö ü ─ │ ┼ ░ ▒ █ ½\r\n",
        "ｶﾀｶﾅ 〜 − — ～ ① ㈱ 髙\r\n",
        "😀 emoji\r\n",
        "lower UPPER 0123 @[]\\^_`{|}~\r\n",
        "\u{00A0}nbsp \u{00FF}ÿ\r\n",
        "\t tab \u{7}bell \u{8}bs\r\n",
        "bare LF\nand CR\ronly\r\n",
    ];
    let modes = [OutputMode::Ansi, OutputMode::Plain, OutputMode::PetsciiCtrl];
    let mut out = String::new();
    for text in corpus {
        let caret = convert_caret_escape(text);
        let _ = writeln!(out, "input: {text:?}");
        for mode in modes {
            let processed = process_output_mode(&caret, mode);
            for enc in ENCODINGS {
                let bytes = encode_for_client(&processed, enc);
                let _ = writeln!(
                    out,
                    "  [{mode:?}][{}] {}",
                    enc_name(enc),
                    escape_bytes(&bytes).replace('\n', "")
                );
            }
        }
    }
    assert_golden("u_output_conversion", &out);
}
