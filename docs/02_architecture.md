# HOBBS - システムアーキテクチャ

## 1. 全体構成図

```
┌─────────────────────────────────────────────────────────────────┐
│                        HOBBS Server                              │
│                                                                  │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │                    Telnet Layer                           │  │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────┐  │  │
│  │  │   Listener  │  │  Session    │  │  ShiftJIS       │  │  │
│  │  │   (tokio)   │  │  Manager    │  │  Encoder/Decoder│  │  │
│  │  └─────────────┘  └─────────────┘  └─────────────────┘  │  │
│  └──────────────────────────────────────────────────────────┘  │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │              SSH Tunnel Layer (russh)                      │  │
│  │  direct-tcpip → 内部Telnetポートへ双方向TCPリレー          │  │
│  └──────────────────────────────────────────────────────────┘  │
│                              │                                   │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │                  Application Layer                        │  │
│  │  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐   │  │
│  │  │   Auth   │ │  Board   │ │   Chat   │ │   Mail   │   │  │
│  │  │  Module  │ │  Module  │ │  Module  │ │  Module  │   │  │
│  │  └──────────┘ └──────────┘ └──────────┘ └──────────┘   │  │
│  │  ┌──────────┐ ┌──────────┐ ┌──────────────────────┐   │  │
│  │  │   File   │ │  Admin   │ │     Menu / Screen    │   │  │
│  │  │  Module  │ │  Module  │ │       Renderer       │   │  │
│  │  └──────────┘ └──────────┘ └──────────────────────┘   │  │
│  └──────────────────────────────────────────────────────────┘  │
│                              │                                   │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │                    Data Layer                             │  │
│  │  ┌─────────────────────┐  ┌─────────────────────────┐   │  │
│  │  │ SQLite / PostgreSQL │  │    File Storage         │   │  │
│  │  │       (sqlx)        │  │    (filesystem)         │   │  │
│  │  └─────────────────────┘  └─────────────────────────┘   │  │
│  └──────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
          │
          │ Telnet (TCP)
          ▼
    ┌───────────┐
    │  Client   │ (TeraTermなど)
    └───────────┘
```

## 2. 技術スタック

### 言語・ランタイム
| 項目 | 選定 | 理由 |
|------|------|------|
| 言語 | Rust 2021 Edition | メモリ安全性、高パフォーマンス、非同期サポート |
| 非同期ランタイム | tokio | Rustの標準的な非同期ランタイム |

### 主要クレート（依存ライブラリ）

```toml
[dependencies]
# 非同期・ネットワーク
tokio = { version = "1", features = ["full"] }

# 文字コード変換
encoding_rs = "0.8"

# データベース（SQLite または PostgreSQL）
sqlx = { version = "0.8", features = ["runtime-tokio", "chrono", "migrate"] }
# ビルド時に --features sqlite または --features postgres で選択

# パスワードハッシュ
argon2 = "0.5"

# 設定ファイル
toml = "0.8"
serde = { version = "1", features = ["derive"] }

# ログ
tracing = "0.1"
tracing-subscriber = "0.3"

# 日時
chrono = { version = "0.4", features = ["serde"] }

# UUID生成
uuid = { version = "1", features = ["v4"] }
```

## 3. ディレクトリ構成

```
hobbs/
├── Cargo.toml
├── Cargo.lock
├── config.toml              # サーバ設定ファイル
├── docs/                    # ドキュメント
│   ├── 01_overview.md
│   ├── 02_architecture.md
│   ├── 03_features/
│   ├── 04_database.md
│   ├── 05_protocol.md
│   ├── 06_screens.md
│   └── 07_security.md
├── templates/               # 画面テンプレート
│   ├── 80/                  # 80カラム用
│   │   ├── welcome.txt      # ウェルカム画面
│   │   ├── main_menu.txt    # メインメニュー
│   │   └── help.txt         # ヘルプ画面
│   └── 40/                  # 40カラム用（C64等）
│       ├── welcome.txt
│       ├── main_menu.txt
│       └── help.txt
├── locales/                 # 言語リソース
│   ├── ja.toml              # 日本語
│   └── en.toml              # 英語
├── data/                    # ランタイムデータ
│   ├── hobbs.db             # SQLiteデータベース
│   └── files/               # アップロードファイル保存
├── src/
│   ├── main.rs              # エントリポイント
│   ├── lib.rs               # ライブラリルート
│   ├── config.rs            # 設定読み込み
│   ├── server/              # Telnetサーバ層 + SSHトンネル
│   │   ├── mod.rs
│   │   ├── listener.rs      # TCP接続受付
│   │   ├── session.rs       # セッション管理
│   │   ├── encoding.rs      # ShiftJIS変換
│   │   └── ssh.rs           # SSHトンネルサーバー
│   ├── terminal/            # 端末プロファイル
│   │   ├── mod.rs
│   │   └── profile.rs       # TerminalProfile定義
│   ├── screen/              # 画面表示
│   │   ├── mod.rs
│   │   ├── ansi.rs          # ANSIエスケープシーケンス
│   │   ├── menu.rs          # メニュー表示
│   │   ├── renderer.rs      # 画面レンダリング
│   │   └── plain.rs         # プレーンテキスト描画
│   ├── auth/                # 認証・会員管理
│   │   ├── mod.rs
│   │   ├── user.rs          # ユーザー管理
│   │   ├── session.rs       # ログインセッション
│   │   └── permission.rs    # 権限管理
│   ├── board/               # 掲示板
│   │   ├── mod.rs
│   │   ├── board.rs         # 掲示板管理
│   │   ├── thread.rs        # スレッド管理
│   │   └── post.rs          # 投稿管理
│   ├── chat/                # チャット
│   │   ├── mod.rs
│   │   └── room.rs          # チャットルーム
│   ├── mail/                # メール
│   │   ├── mod.rs
│   │   └── message.rs       # メッセージ管理
│   ├── file/                # ファイル管理
│   │   ├── mod.rs
│   │   ├── folder.rs        # フォルダ管理
│   │   └── transfer.rs      # アップロード/ダウンロード
│   ├── admin/               # 管理機能
│   │   ├── mod.rs
│   │   └── management.rs    # 管理操作
│   ├── template/            # テンプレートエンジン
│   │   ├── mod.rs
│   │   ├── loader.rs        # テンプレート読み込み
│   │   ├── renderer.rs      # 変数展開・描画
│   │   └── i18n.rs          # 国際化（言語リソース）
│   └── db/                  # データベース
│       ├── mod.rs
│       ├── schema.rs        # スキーマ定義
│       └── repository.rs    # データアクセス
└── tests/                   # テスト
    ├── integration/
    └── fixtures/
```

## 4. モジュール構成

### 4.1 Server層（Telnet）

```rust
// server/mod.rs
pub mod listener;   // TCP接続受付
pub mod session;    // セッション管理
pub mod encoding;   // ShiftJIS変換

// 主要な構造体
pub struct TelnetServer {
    listener: TcpListener,
    sessions: Arc<RwLock<HashMap<Uuid, TelnetSession>>>,
    config: ServerConfig,
}

/// 接続セッション（Telnet接続の状態を管理）
/// ※ AuthSession（認証セッション）とは別の概念
pub struct TelnetSession {
    id: Uuid,
    stream: TcpStream,
    user: Option<User>,           // ログイン中のユーザー（ゲスト/未ログインはNone）
    auth_session: Option<AuthSession>,  // 認証セッション
    terminal: TerminalProfile,    // 端末プロファイル
    encoding: CharacterEncoding,  // 文字エンコーディング（ShiftJIS/UTF-8）
    state: SessionState,
    last_activity: Instant,
}

/// 文字エンコーディング
#[derive(Debug, Clone, Copy, Default)]
pub enum CharacterEncoding {
    #[default]
    ShiftJIS,  // 日本語レガシー端末向け
    Utf8,      // モダン端末向け
    Cp437,     // IBM PC Code Page 437
    Petscii,   // Commodore 64/128
}

/// 出力モード（ANSIエスケープシーケンスの処理）
#[derive(Debug, Clone, Copy, Default)]
pub enum OutputMode {
    #[default]
    Ansi,        // ANSIシーケンスをそのまま出力
    Plain,       // ANSIシーケンスを除去
    PetsciiCtrl, // ANSIをPETSCII制御コードに変換
}
```

### 4.2 Terminal層（端末プロファイル）

```rust
// terminal/profile.rs

#[derive(Debug, Clone)]
pub struct TerminalProfile {
    pub name: String,                    // プロファイル名
    pub width: u16,                      // 画面幅（カラム数）
    pub height: u16,                     // 画面高（行数）
    pub cjk_width: u8,                   // 全角文字の幅（1 or 2）
    pub ansi_enabled: bool,              // ANSIエスケープシーケンス対応
    pub encoding: CharacterEncoding,     // 文字エンコーディング
    pub output_mode: OutputMode,         // 出力モード
    pub template_dir: String,            // テンプレートディレクトリ（"80" or "40"）
}

impl TerminalProfile {
    /// 標準端末（80x24、ShiftJIS、ANSI対応）
    pub fn standard() -> Self { /* ... */ }

    /// 標準UTF-8端末（80x24、UTF-8、ANSI対応）
    pub fn standard_utf8() -> Self { /* ... */ }

    /// DOS端末（80x25、CP437、ANSI対応）
    pub fn dos() -> Self { /* ... */ }

    /// Commodore 64（40x25、PETSCII、ANSIなし）
    pub fn c64() -> Self { /* ... */ }

    /// Commodore 64 PETSCII制御コード使用
    pub fn c64_petscii() -> Self { /* ... */ }

    /// Commodore 64 ANSI対応版
    pub fn c64_ansi() -> Self { /* ... */ }

    /// 利用可能なプロファイル名一覧
    pub fn available_profiles() -> Vec<&'static str> {
        vec!["standard", "standard_utf8", "dos", "c64", "c64_petscii", "c64_ansi"]
    }

    /// 名前からプロファイルを取得
    pub fn from_name(name: &str) -> Self { /* ... */ }

    /// カスタムプロファイルを含めてプロファイルを取得
    pub fn from_name_with_custom(
        name: &str,
        custom_profiles: &[ProfileConfig],
    ) -> Self { /* ... */ }

    /// 設定からプロファイルを生成
    pub fn from_config(config: &ProfileConfig) -> Self { /* ... */ }

    /// 文字列の表示幅を計算
    pub fn display_width(&self, s: &str) -> usize { /* ... */ }
}
```

カスタムプロファイルは `config.toml` で `[[terminal.profiles]]` として定義可能。

### 4.3 Screen層（画面表示）

```rust
// screen/mod.rs
pub mod ansi;       // ANSIコード
pub mod menu;       // メニュー
pub mod renderer;   // レンダラー
pub mod plain;      // プレーンテキスト描画

// ANSIカラー定義
pub enum Color {
    Black, Red, Green, Yellow,
    Blue, Magenta, Cyan, White,
}

// 画面描画トレイト
pub trait Screen {
    fn render(&self, profile: &TerminalProfile) -> String;
    fn handle_input(&mut self, input: &str) -> Action;
}

// 描画の分岐
impl dyn Screen {
    fn render_with_profile(&self, profile: &TerminalProfile) -> String {
        if profile.ansi_enabled {
            self.render_ansi(profile)
        } else {
            self.render_plain(profile)
        }
    }
}
```

### 4.4 Application層（各機能モジュール）

各機能モジュールは以下の共通パターンで実装：

```rust
// 例: board/mod.rs
pub struct BoardModule {
    db: Arc<Database>,
}

impl BoardModule {
    pub async fn list_boards(&self) -> Result<Vec<Board>>;
    pub async fn get_board(&self, id: i64) -> Result<Board>;
    pub async fn create_board(&self, data: CreateBoard) -> Result<Board>;
    // ...
}
```

### 4.5 Data層（永続化）

```rust
// db/mod.rs
pub struct Database {
    conn: Connection,  // rusqlite::Connection
}

impl Database {
    pub fn new(path: &Path) -> Result<Self>;
    pub fn migrate(&self) -> Result<()>;
}
```

## 5. データフロー

### 接続〜ログインの流れ

```
1. クライアント接続
   └─> TelnetServer::accept()
       └─> TelnetSession::new()
           └─> ウェルカム画面表示

2. ログイン操作
   └─> TelnetSession::handle_input()
       └─> AuthModule::login(username, password)
           └─> Database::get_user()
           └─> パスワード検証
           └─> AuthSession作成
           └─> TelnetSession::set_user()
               └─> メインメニュー表示
```

### 掲示板投稿の流れ

```
1. 掲示板選択
   └─> BoardModule::get_board()
       └─> 掲示板画面表示

2. 投稿作成
   └─> BoardModule::create_post()
       └─> 権限チェック
       └─> Database::insert_post()
       └─> 投稿完了画面表示
```

## 6. エンコーディング処理

### 6.1 処理フロー

```
受信（クライアント→サーバ）:
┌─────────────┐    ┌──────────────────┐    ┌─────────────┐
│  バイト列   │ -> │ encoding.rsで変換 │ -> │   UTF-8     │
│ (ShiftJIS/  │    │ (ユーザー設定に   │    │ (内部処理)  │
│  UTF-8)     │    │  基づく)          │    │             │
└─────────────┘    └──────────────────┘    └─────────────┘

送信（サーバ→クライアント）:
┌─────────────┐    ┌──────────────────┐    ┌─────────────┐
│   UTF-8     │ -> │ encoding.rsで変換 │ -> │  バイト列   │
│ (内部処理)  │    │ (ユーザー設定に   │    │ (ShiftJIS/  │
│             │    │  基づく)          │    │  UTF-8)     │
└─────────────┘    └──────────────────┘    └─────────────┘
```

### 6.2 エンコーディング変換の実装

```rust
// server/encoding.rs

pub fn encode_for_client(text: &str, encoding: CharacterEncoding) -> Vec<u8> {
    match encoding {
        CharacterEncoding::Utf8 => text.as_bytes().to_vec(),
        CharacterEncoding::ShiftJIS => encode_shiftjis(text),
    }
}

pub fn decode_from_client(bytes: &[u8], encoding: CharacterEncoding) -> String {
    match encoding {
        CharacterEncoding::Utf8 => String::from_utf8_lossy(bytes).to_string(),
        CharacterEncoding::ShiftJIS => decode_shiftjis(bytes),
    }
}
```

### 6.3 ユーザー設定との連携

```
1. ログイン時
   └─> User.encoding をセッションに適用
       └─> TelnetSession.encoding を設定

2. ゲストアクセス時
   └─> 端末選択画面でエンコーディングも選択
       └─> セッションに一時保存（DB非保存）

3. プロフィール変更時
   └─> User.encoding を更新
       └─> 現在のセッションにも即時反映
```

## 7. 設定ファイル

```toml
# config.toml

[server]
host = "0.0.0.0"
port = 2323
max_connections = 20
idle_timeout_secs = 300

[database]
path = "data/hobbs.db"

[files]
storage_path = "data/files"
max_upload_size_mb = 10

[bbs]
name = "HOBBS - Hobbyist BBS"
description = "レトロなパソコン通信BBSです"
sysop_name = "SysOp"

[locale]
language = "ja"              # 使用言語（ja / en）

[templates]
path = "templates"           # テンプレートディレクトリ

[logging]
level = "info"
file = "logs/hobbs.log"
```

## 8. エラーハンドリング

```rust
// 共通エラー型
#[derive(Debug, thiserror::Error)]
pub enum HobbsError {
    #[error("データベースエラー: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("IO エラー: {0}")]
    Io(#[from] std::io::Error),

    #[error("認証エラー: {0}")]
    Auth(String),

    #[error("権限エラー: {0}")]
    Permission(String),

    #[error("入力エラー: {0}")]
    Validation(String),
}

pub type Result<T> = std::result::Result<T, HobbsError>;
```

## 9. 非同期処理モデル

tokioランタイムを使用し、各接続を独立したタスクとして処理：

```rust
#[tokio::main]
async fn main() -> Result<()> {
    let server = TelnetServer::new(config).await?;

    loop {
        let (stream, addr) = server.accept().await?;

        // 各接続を独立したタスクで処理
        tokio::spawn(async move {
            let session = TelnetSession::new(stream, addr);
            session.run().await;
        });
    }
}
```

## 10. チャットのブロードキャスト

チャットメッセージは `tokio::sync::broadcast` チャネルで配信：

```rust
pub struct ChatRoom {
    tx: broadcast::Sender<ChatMessage>,
}

impl ChatRoom {
    pub fn join(&self) -> broadcast::Receiver<ChatMessage> {
        self.tx.subscribe()
    }

    pub fn send(&self, msg: ChatMessage) {
        let _ = self.tx.send(msg);
    }
}
```

## 11. プロトコル拡張

### 11.1 SSHトンネルサーバー

HOBBS内蔵のSSHサーバー（`src/server/ssh.rs`）により、Telnet通信をSSHで暗号化できる。
`russh` クレートを使用し、`direct-tcpip`（ポートフォワード）で内部Telnetポートへリレーする。

**方式: 内部TCPリレー型**
- SSHの `direct-tcpip` チャネルで内部Telnetポートへ双方向リレー
- Shell接続は非サポート（SSHターミナルはTelnet IACを処理できないため）
- BBS側のコードには一切変更なし — SSHは純粋なトランスポート層

```
現在の構成:

                     ┌──────────────────────────┐
                     │   Application Layer      │
                     │ (Session, Auth, BBS等)   │
                     └───────────┬──────────────┘
                                 │
         ┌───────────────────────┼───────────────────────┐
         │                       │                       │
    ┌────▼────┐            ┌─────▼─────┐          ┌──────▼──────┐
    │ Telnet  │            │SSH Tunnel │          │   Web UI    │
    │  Layer  │            │(direct-   │          │  (REST +    │
    │ :2323   │            │ tcpip)    │          │   SPA)      │
    └─────────┘            │ :2222     │          │  :8080      │
                           └─────┬─────┘          └─────────────┘
                                 │ TCP relay
                           ┌─────▼─────┐
                           │  Telnet   │
                           │  :2323    │
                           │(localhost)│
                           └───────────┘
```

SSHサーバーは `tokio::spawn` で独立タスクとして起動（Send-safe）。
接続数制限はセマフォでSSHハンドシェイク前に即拒否し、
チャネル数制限は `Arc<tokio::sync::Mutex<HashSet<ChannelId>>>` で管理する。

### 11.2 将来の拡張

- WebSocket対応（ブラウザからのアクセス）
- IPv6対応
- TLS over Telnet（STARTTLS）
