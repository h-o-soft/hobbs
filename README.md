# HOBBS - Hobbyist Bulletin Board System

[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

Telnetプロトコルで接続するレトロなパソコン通信BBSホストプログラムです。1980〜90年代に栄えたパソコン通信文化を現代の技術で再現し、テキストベースのコミュニケーション体験を提供します。

## 機能一覧

| 機能 | 概要 |
|------|------|
| 会員管理 | 入会・退会・会員情報管理・3段階権限システム（SysOp/SubOp/一般） |
| 掲示板 | スレッド形式/フラット形式両対応、動的追加・削除 |
| チャット | 単一ロビー形式のリアルタイムチャット |
| メール | BBS内部のプライベートメッセージ機能 |
| ファイル | フォルダ単位の権限設定可能なファイル共有、XMODEMプロトコル対応 |
| ドアゲーム | Luaスクリプトによる拡張機能（じゃんけん、数当て、おみくじ等） |
| ゲストアクセス | 会員登録なしでの一部コンテンツ閲覧 |
| 国際化 | 日本語・英語の2言語対応 |
| Web UI | REST API + SPA によるブラウザアクセス（オプション） |

## 技術仕様

| 項目 | 仕様 |
|------|------|
| 通信プロトコル | Telnet |
| 文字コード | ShiftJIS（クライアント側）/ UTF-8（内部処理） |
| 画面装飾 | ANSIエスケープシーケンス対応 |
| 実装言語 | Rust |
| データベース | SQLite |
| 想定規模 | 同時接続〜20人程度 |

## 必要環境

- Rust 1.70以上
- SQLite 3.x（bundled版を使用するため個別インストール不要）
- Node.js 18以上（Web UI をビルドする場合）

## インストール

### ソースからビルド

```bash
# リポジトリをクローン
git clone https://github.com/h-o-soft/hobbs.git
cd hobbs

# リリースビルド
cargo build --release

# 実行ファイルは target/release/hobbs に生成されます
```

### 初期設定

1. 設定ファイルを編集:

```bash
cp config.toml config.local.toml  # 必要に応じてコピーして編集
```

2. 必要なディレクトリを作成:

```bash
mkdir -p data/files logs
```

3. サーバーを起動:

```bash
./target/release/hobbs
```

## 設定ファイル

`config.toml` でサーバーの動作を設定します。

```toml
[server]
host = "0.0.0.0"          # バインドアドレス
port = 2323               # ポート番号
max_connections = 20      # 最大同時接続数
idle_timeout_secs = 300   # アイドルタイムアウト（秒）

[database]
path = "data/hobbs.db"    # データベースファイルパス

[files]
storage_path = "data/files"   # ファイルストレージパス
max_upload_size_mb = 10       # 最大アップロードサイズ（MB）

[bbs]
name = "HOBBS - Hobbyist BBS"   # BBS名
description = "A retro BBS system"  # 説明
sysop_name = "SysOp"            # SysOp名

[locale]
language = "ja"           # デフォルト言語（ja/en）

[templates]
path = "templates"        # テンプレートディレクトリ

[logging]
level = "info"            # ログレベル（debug/info/warn/error）
file = "logs/hobbs.log"   # ログファイルパス
```

## 使い方

### サーバー起動

```bash
# 通常起動
./target/release/hobbs

# 開発モード（デバッグログ有効）
RUST_LOG=debug cargo run
```

### クライアント接続

任意のTelnetクライアントで接続できます:

```bash
# macOS/Linux
telnet localhost 2323

# Windows (Windows Terminal)
telnet localhost 2323

# または専用Telnetクライアント（Tera Term, PuTTY等）を使用
```

### Web UI セットアップ（オプション）

Telnet に加えて、ブラウザからアクセスできる Web UI を有効にできます。

```bash
# 1. フロントエンドのビルド
cd web
npm ci
npm run build
cd ..

# 2. config.toml に Web 設定を追加
```

```toml
[web]
enabled = true
host = "0.0.0.0"
port = 8080
serve_static = true
static_path = "web/dist"
jwt_secret = ""  # 環境変数 HOBBS_JWT_SECRET で設定推奨
```

```bash
# 3. 環境変数で JWT 秘密鍵を設定（必須）
export HOBBS_JWT_SECRET=$(openssl rand -base64 32)

# 4. サーバー起動
./target/release/hobbs
```

ブラウザで `http://localhost:8080` にアクセスできます。

詳細は [運用ガイド](docs/operation_guide.md) を参照してください。

### 初回セットアップ

1. 最初に登録したユーザーが自動的にSysOp（管理者）になります
2. 「R」キーで新規登録
3. ユーザー名、パスワード、ニックネームを入力
4. 登録完了後、メインメニューが表示されます

### メインメニュー

```
=== Main Menu ===
[B] 掲示板
[C] チャット
[M] メール
[F] ファイル
[D] ドア（ゲーム）
[P] プロフィール
[A] 管理（管理者のみ）
[Q] 終了
```

## ディレクトリ構成

```
hobbs/
├── Cargo.toml          # プロジェクト設定
├── config.toml         # サーバー設定
├── CLAUDE.md           # 開発ガイドライン
├── README.md           # このファイル
├── docs/               # 設計ドキュメント
│   ├── 01_overview.md
│   ├── 02_architecture.md
│   ├── 03_features/
│   ├── 04_database.md
│   ├── 05_protocol.md
│   ├── 06_screens.md
│   └── 07_security.md
├── templates/          # 画面テンプレート
│   ├── 80/            # 80カラム用
│   └── 40/            # 40カラム用
├── locales/           # 言語リソース
│   ├── ja.toml
│   └── en.toml
├── scripts/           # Luaスクリプト（ドアゲーム等）
├── data/              # ランタイムデータ（自動生成）
│   ├── hobbs.db
│   └── files/
├── logs/              # ログファイル
└── src/               # ソースコード
```

## 開発

### テスト実行

```bash
# 全テスト実行
cargo test

# E2Eテストのみ
cargo test --test e2e_connection --test e2e_auth --test e2e_board --test e2e_mail --test e2e_admin

# 特定のテスト
cargo test test_name
```

### コード品質

```bash
# フォーマット
cargo fmt

# Lint
cargo clippy

# ドキュメント生成
cargo doc --open
```

### 開発ガイドライン

詳細は [CLAUDE.md](CLAUDE.md) を参照してください。

## 用語

| 用語 | 説明 |
|------|------|
| SysOp | System Operator。システム管理者。全権限を持つ |
| SubOp | Sub Operator。副管理者。一部管理権限を持つ |
| ゲスト | 未登録ユーザー。閲覧のみ可能 |
| スレッド形式 | トピックごとにレスがつく掲示板形式 |
| フラット形式 | 時系列に記事が並ぶ従来のBBS形式 |

## ライセンス

MIT License

Copyright (c) 2024-2025 HOBBS Project

詳細は [LICENSE](LICENSE) ファイルを参照してください。

## 関連リンク

- [設計ドキュメント](docs/)
- [Issue Tracker](https://github.com/h-o-soft/hobbs/issues)
