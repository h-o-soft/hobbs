# HOBBS - 開発ガイドライン

## プロジェクト概要

HOBBS (Hobbyist Bulletin Board System) は、Telnetプロトコルで接続するレトロなパソコン通信BBSホストプログラム。Rustで実装し、SQLiteまたはPostgreSQLをデータベースとして使用する（コンパイル時のfeature flagで選択）。

## 開発方針

### テスト駆動開発 (TDD)

本プロジェクトはテスト駆動開発で進める。

1. **Red**: 失敗するテストを先に書く
2. **Green**: テストを通す最小限のコードを書く
3. **Refactor**: コードを整理する

```bash
# テスト実行
cargo test

# 特定のテストを実行
cargo test test_name

# テストを監視モードで実行（cargo-watchが必要）
cargo watch -x test
```

### テストの種類

| 種類 | 場所 | 目的 |
|------|------|------|
| 単体テスト | 各モジュール内 `#[cfg(test)]` | 個々の関数・構造体のテスト |
| 統合テスト | `tests/` ディレクトリ | モジュール間連携のテスト |
| E2Eテスト | `tests/e2e_*.rs` | Telnet接続を含む全体テスト |

```bash
# E2Eテスト実行
cargo test --test e2e_connection --test e2e_auth --test e2e_board --test e2e_mail --test e2e_admin
```

## アーキテクチャ

### レイヤー構成

```
┌─────────────────────────────────────┐
│           Telnet Layer              │  ← 接続管理、ShiftJIS変換
├─────────────────────────────────────┤
│         Application Layer           │  ← 各機能モジュール
├─────────────────────────────────────┤
│            Data Layer               │  ← SQLite/PostgreSQL、ファイルストレージ
└─────────────────────────────────────┘
```

### セッション構造

2種類のセッションを区別する：

- **TelnetSession**: Telnet接続の状態管理（stream, terminal, state）
- **AuthSession**: 認証状態の管理（token, user_id, expires_at）

### 非同期処理

- ランタイム: tokio
- データベース: sqlx（非同期ネイティブ）
- チャットのブロードキャスト: `tokio::sync::broadcast`

## コーディング規約

### Rust スタイル

```bash
# フォーマット
cargo fmt

# Lint
cargo clippy
```

### 命名規則

| 対象 | 規則 | 例 |
|------|------|-----|
| 構造体・列挙型 | PascalCase | `TelnetSession`, `BoardType` |
| 関数・変数 | snake_case | `create_user`, `board_id` |
| 定数 | SCREAMING_SNAKE_CASE | `MAX_CONNECTIONS` |
| モジュール | snake_case | `auth`, `board` |

### エラーハンドリング

- `thiserror` クレートで共通エラー型 `HobbsError` を定義
- `Result<T>` は `Result<T, HobbsError>` のエイリアス
- ユーザー向けエラーメッセージは内部情報を漏らさない

```rust
#[derive(Debug, thiserror::Error)]
pub enum HobbsError {
    #[error("データベースエラー: {0}")]
    Database(String),
    #[error("認証エラー: {0}")]
    Auth(String),
    // ...
}
```

### 文字列処理

- 内部処理: UTF-8
- Telnet送受信: 端末プロファイルのエンコーディング設定に従う（ShiftJIS/UTF-8/CP437/PETSCII）
- 変換ライブラリ: `encoding_rs`
- 文字幅計算: `TerminalProfile` の `display_width()` を使用

## プロジェクトルール

### 権限レベル

```rust
pub enum Role {
    Guest = 0,   // ゲスト（未登録）
    Member = 1,  // 一般会員
    SubOp = 2,   // 副管理者
    SysOp = 3,   // システム管理者
}
```

権限チェック: `user.role >= required_role`

### 掲示板形式

- **thread**: スレッド形式（トピックにレスがつく）
- **flat**: フラット形式（時系列に記事が並ぶ）

### 端末プロファイル

| プロファイル | 幅 | 高 | 全角幅 | エンコーディング | 出力モード |
|--------------|----|----|--------|------------------|------------|
| standard | 80 | 24 | 2 | ShiftJIS | Ansi |
| standard_utf8 | 80 | 24 | 2 | UTF-8 | Ansi |
| dos | 80 | 25 | 1 | CP437 | Ansi |
| c64 | 40 | 25 | 1 | PETSCII | Plain |
| c64_petscii | 40 | 25 | 1 | PETSCII | PetsciiCtrl |
| c64_ansi | 40 | 25 | 1 | PETSCII | Ansi |
| 40col_sjis | 40 | 25 | 2 | ShiftJIS | Ansi |
| 40col_utf8 | 40 | 25 | 2 | UTF-8 | Ansi |

カスタムプロファイルは `config.toml` の `[[terminal.profiles]]` で定義可能。

### 国際化 (i18n)

初期実装から日本語・英語の2言語対応必須。

- 言語リソース: `locales/ja.toml`, `locales/en.toml`
- テンプレート内: `{{t "キー"}}` で翻訳参照
- ハードコードされた日本語文字列は禁止

### パスワード

- ハッシュ: Argon2id
- 最小長: 8文字
- 最大長: 128文字

### 入力制限

| 項目 | 最大長 |
|------|--------|
| ユーザーID | 16文字 |
| ニックネーム | 20文字 |
| 投稿タイトル | 50文字 |
| 投稿本文 | 10,000文字 |
| チャット発言 | 500文字 |

## ディレクトリ構成

```
hobbs/
├── Cargo.toml
├── config.toml           # サーバ設定
├── docs/                  # 設計ドキュメント
├── templates/             # 画面テンプレート
│   ├── 80/               # 80カラム用
│   └── 40/               # 40カラム用
├── locales/              # 言語リソース
│   ├── ja.toml
│   └── en.toml
├── data/                 # ランタイムデータ
│   ├── hobbs.db
│   └── files/
├── scripts/              # Luaスクリプト（ドアゲーム等）
├── src/
│   ├── main.rs
│   ├── lib.rs
│   ├── config.rs
│   ├── error.rs          # HobbsError定義
│   ├── server/           # Telnetサーバ層
│   ├── terminal/         # 端末プロファイル
│   ├── screen/           # 画面表示
│   ├── auth/             # 認証・会員管理
│   ├── board/            # 掲示板
│   ├── chat/             # チャット
│   ├── mail/             # メール
│   ├── file/             # ファイル管理
│   ├── admin/            # 管理機能
│   ├── template/         # テンプレートエンジン
│   ├── script/           # スクリプトプラグイン（Lua）
│   ├── xmodem/           # XMODEMファイル転送
│   └── db/               # データベース
└── tests/                # 統合テスト
```

## 依存クレート

```toml
[features]
default = ["sqlite"]
sqlite = ["sqlx/sqlite"]
postgres = ["sqlx/postgres"]

[dependencies]
tokio = { version = "1", features = ["full"] }
encoding_rs = "0.8"
sqlx = { version = "0.8", features = ["runtime-tokio", "chrono", "migrate"] }
argon2 = "0.5"
toml = "0.8"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tracing = "0.1"
tracing-subscriber = "0.3"
chrono = { version = "0.4", features = ["serde"] }
uuid = { version = "1", features = ["v4"] }
thiserror = "1"
mlua = { version = "0.10", features = ["lua54", "async", "serialize"] }
rand = "0.8"
```

ビルド時に `--features sqlite`（デフォルト）または `--no-default-features --features postgres` で選択。

## 開発ワークフロー

Issue対応は以下のフローで行う：

1. **ブランチ作成**: `issue-<番号>-<簡潔な説明>` 形式でブランチを作成
2. **実装**: TDDで機能を実装
3. **コミット**: 適切な粒度でコミット
4. **プルリクエスト作成**: mainブランチへのPRを作成
5. **レビュー**: 人間がレビューしてマージ

```bash
# ブランチ作成例
git checkout -b issue-1-cargo-setup

# PR作成例
gh pr create --title "[Phase 1-1] Cargo.tomlと基本構造の作成" --body "..."
```

**重要**: 作業完了後は必ずPRを作成し、人間のレビューを経てからマージする。直接mainブランチにコミットしない。

## コミットメッセージ

```
<type>: <subject>

<body>
```

type:
- `feat`: 新機能
- `fix`: バグ修正
- `refactor`: リファクタリング
- `test`: テスト追加・修正
- `docs`: ドキュメント
- `chore`: ビルド・設定変更

## 参考ドキュメント

詳細な仕様は `docs/` ディレクトリを参照：

- `01_overview.md` - プロジェクト概要
- `02_architecture.md` - システムアーキテクチャ
- `03_features/` - 機能仕様
- `04_database.md` - データベース設計
- `05_protocol.md` - プロトコル仕様
- `06_screens.md` - 画面設計
- `07_security.md` - セキュリティ仕様
- `operation_guide.md` - 運用ガイド
