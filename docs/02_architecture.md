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
│  │  │   SQLite Database   │  │    File Storage         │   │  │
│  │  │   (rusqlite)        │  │    (filesystem)         │   │  │
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

# データベース
rusqlite = { version = "0.31", features = ["bundled"] }

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
│   ├── server/              # Telnetサーバ層
│   │   ├── mod.rs
│   │   ├── listener.rs      # TCP接続受付
│   │   ├── session.rs       # セッション管理
│   │   └── encoding.rs      # ShiftJIS変換
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
    state: SessionState,
    last_activity: Instant,
}
```

### 4.2 Terminal層（端末プロファイル）

```rust
// terminal/profile.rs

#[derive(Debug, Clone)]
pub struct TerminalProfile {
    pub name: String,         // プロファイル名
    pub width: u16,           // 画面幅（カラム数）
    pub height: u16,          // 画面高（行数）
    pub cjk_width: u8,        // 全角文字の幅（1 or 2）
    pub ansi_enabled: bool,   // ANSIエスケープシーケンス対応
}

impl TerminalProfile {
    /// 標準端末（80x24、全角2幅、ANSI対応）
    pub fn standard() -> Self {
        Self {
            name: "standard".to_string(),
            width: 80, height: 24,
            cjk_width: 2, ansi_enabled: true,
        }
    }

    /// Commodore 64（40x25、全角1幅、ANSIなし）
    pub fn c64() -> Self {
        Self {
            name: "c64".to_string(),
            width: 40, height: 25,
            cjk_width: 1, ansi_enabled: false,
        }
    }

    /// Commodore 64 ANSI対応版
    pub fn c64_ansi() -> Self {
        Self {
            name: "c64_ansi".to_string(),
            width: 40, height: 25,
            cjk_width: 1, ansi_enabled: true,
        }
    }

    /// 文字列の表示幅を計算
    pub fn display_width(&self, s: &str) -> usize {
        if self.cjk_width == 1 {
            s.chars().count()  // C64: 全て1幅
        } else {
            s.chars().map(|c| if c.is_ascii() { 1 } else { 2 }).sum()
        }
    }
}
```

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

## 6. 設定ファイル

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

## 7. エラーハンドリング

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

## 8. 非同期処理モデル

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

## 9. チャットのブロードキャスト

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
