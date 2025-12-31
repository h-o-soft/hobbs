# HOBBS 実装計画

## 概要

本ドキュメントは、HOBBSの実装計画をフェーズごとにまとめたものです。
各タスクはGitHub issueの粒度（半日〜1日で完了）で記載しています。

## 開発フロー

1. 人間がissue登録を指示（直近1-2フェーズ分のみ）
2. 人間がissueを指示 → AI開発 → ブランチ → PR → レビュー
3. フェーズ完了後、次フェーズのissue登録を検討

## フェーズ一覧

| Phase | 名称 | 概要 | 依存 |
|-------|------|------|------|
| 1 | プロジェクト基盤 | Cargo.toml, エラー型, 設定, ログ | - |
| 2 | Telnetサーバ基盤 | 接続受付, セッション, 文字コード | Phase 1 |
| 3 | データベース・認証 | DB基盤, ユーザー管理, ログイン | Phase 2 |
| 4 | 掲示板機能 | 掲示板, スレッド, 投稿, 未読 | Phase 3 |
| 5 | チャット機能 | チャットルーム, ブロードキャスト | Phase 3 |
| 6 | メール機能 | 内部メール送受信 | Phase 3 |
| 7 | ファイル管理 | フォルダ, アップロード/ダウンロード | Phase 3 |
| 8 | 管理機能 | 管理メニュー, ユーザー/コンテンツ管理 | Phase 4-7 |
| 9 | テンプレート・国際化 | テンプレートエンジン, i18n | Phase 2 |
| 10 | 統合・調整 | 画面遷移, E2Eテスト | Phase 1-9 |

---

## Phase 1: プロジェクト基盤 ✅

### 1-1. Cargo.toml と基本構造の作成 ✅

**概要**: プロジェクトの骨格を作成する

**完了条件**:
- [x] Cargo.toml に依存クレートを記載
- [x] src/main.rs, src/lib.rs を作成
- [x] `cargo build` が成功する
- [x] `cargo test` が実行できる（テストは空でも可）

**関連ファイル**:
- `Cargo.toml`
- `src/main.rs`
- `src/lib.rs`

---

### 1-2. エラー型の定義 ✅

**概要**: 共通エラー型 `HobbsError` を定義する

**完了条件**:
- [x] `src/error.rs` を作成
- [x] `HobbsError` 列挙型を定義（Database, Io, Auth, Permission, Validation, NotFound）
- [x] `thiserror` を使用
- [x] `Result<T>` エイリアスを定義
- [x] 単体テストを作成

**関連ファイル**:
- `src/error.rs`
- `src/lib.rs`（mod error を追加）

---

### 1-3. 設定ファイル読み込み ✅

**概要**: config.toml を読み込む機能を実装する

**完了条件**:
- [x] `src/config.rs` を作成
- [x] `Config` 構造体を定義（server, database, files, bbs, locale, templates, logging）
- [x] TOML ファイルの読み込み機能
- [x] デフォルト値の設定
- [x] 単体テスト（正常系・異常系）
- [x] サンプル `config.toml` を作成

**関連ファイル**:
- `src/config.rs`
- `config.toml`

---

### 1-4. ロギング設定 ✅

**概要**: tracing を使用したログ出力を設定する

**完了条件**:
- [x] tracing-subscriber の初期化
- [x] ログレベル設定（config.toml から読み込み）
- [x] ファイル出力設定
- [x] 動作確認テスト

**関連ファイル**:
- `src/main.rs`（初期化処理）
- `src/config.rs`（logging セクション）

---

## Phase 2: Telnetサーバ基盤

### 2-1. TCP接続受付

**概要**: tokio を使用した TCP リスナーを実装する

**完了条件**:
- [ ] `src/server/mod.rs`, `src/server/listener.rs` を作成
- [ ] `TelnetServer` 構造体を定義
- [ ] 指定ポートで接続を受け付ける
- [ ] 最大接続数の制御
- [ ] 統合テスト（接続・切断）

**関連ファイル**:
- `src/server/mod.rs`
- `src/server/listener.rs`

---

### 2-2. セッション管理基盤

**概要**: 接続ごとのセッション管理を実装する

**完了条件**:
- [ ] `src/server/session.rs` を作成
- [ ] `TelnetSession` 構造体を定義
- [ ] セッション状態（SessionState）の定義
- [ ] アイドルタイムアウト処理
- [ ] セッション一覧管理（HashMap）
- [ ] 単体テスト

**関連ファイル**:
- `src/server/session.rs`
- `src/server/mod.rs`

---

### 2-3. ShiftJIS変換

**概要**: encoding_rs を使用した文字コード変換を実装する

**完了条件**:
- [ ] `src/server/encoding.rs` を作成
- [ ] ShiftJIS → UTF-8 変換（受信用）
- [ ] UTF-8 → ShiftJIS 変換（送信用）
- [ ] 変換エラーのハンドリング
- [ ] 単体テスト（日本語文字列、制御文字）

**関連ファイル**:
- `src/server/encoding.rs`

---

### 2-4. 端末プロファイル

**概要**: 端末タイプ（80x24, C64等）を管理する

**完了条件**:
- [ ] `src/terminal/mod.rs`, `src/terminal/profile.rs` を作成
- [ ] `TerminalProfile` 構造体を定義
- [ ] standard(), c64(), c64_ansi() プリセット
- [ ] display_width() 関数（全角幅計算）
- [ ] truncate_to_width() 関数
- [ ] 単体テスト

**関連ファイル**:
- `src/terminal/mod.rs`
- `src/terminal/profile.rs`

---

### 2-5. 入力処理・Telnetプロトコル

**概要**: Telnet IAC コマンドと入力処理を実装する

**完了条件**:
- [ ] Telnet ネゴシエーション（ECHO, SGA）
- [ ] 行バッファリング入力
- [ ] バックスペース、Ctrl+C 処理
- [ ] パスワード入力（エコーなし）
- [ ] 統合テスト

**関連ファイル**:
- `src/server/session.rs`（拡張）

---

## Phase 3: データベース・認証

### 3-1. データベース基盤

**概要**: SQLite 接続とマイグレーション機能を実装する

**完了条件**:
- [ ] `src/db/mod.rs`, `src/db/schema.rs` を作成
- [ ] `Database` 構造体（接続プール的な管理）
- [ ] マイグレーション機能
- [ ] schema_version テーブル
- [ ] 初期スキーマ（users テーブル）
- [ ] 単体テスト

**関連ファイル**:
- `src/db/mod.rs`
- `src/db/schema.rs`
- `src/db/migrations/001_initial.sql`

---

### 3-2. ユーザーリポジトリ

**概要**: ユーザーの CRUD 操作を実装する

**完了条件**:
- [ ] `src/db/repository.rs` または `src/auth/repository.rs` を作成
- [ ] ユーザー作成・取得・更新・削除
- [ ] ユーザー名による検索
- [ ] 単体テスト

**関連ファイル**:
- `src/db/repository.rs`
- `src/auth/user.rs`

---

### 3-3. パスワードハッシュ

**概要**: Argon2id によるパスワードハッシュを実装する

**完了条件**:
- [ ] `src/auth/password.rs` を作成
- [ ] hash_password() 関数
- [ ] verify_password() 関数
- [ ] 単体テスト

**関連ファイル**:
- `src/auth/password.rs`

---

### 3-4. ログイン・ログアウト

**概要**: ログイン認証とセッション作成を実装する

**完了条件**:
- [ ] `src/auth/mod.rs`, `src/auth/session.rs` を作成
- [ ] `AuthSession` 構造体
- [ ] login() 関数（認証 + セッション作成）
- [ ] logout() 関数
- [ ] セッション検証
- [ ] ログイン試行制限（3回、5分ロック）
- [ ] 単体テスト

**関連ファイル**:
- `src/auth/mod.rs`
- `src/auth/session.rs`

---

### 3-5. 新規登録

**概要**: ユーザー登録機能を実装する

**完了条件**:
- [ ] register() 関数
- [ ] バリデーション（ユーザーID、パスワード、ニックネーム）
- [ ] 禁止ユーザーIDチェック
- [ ] 重複チェック
- [ ] 単体テスト

**関連ファイル**:
- `src/auth/mod.rs`（拡張）
- `src/auth/validation.rs`

---

### 3-6. 権限チェック

**概要**: Role ベースの権限チェックを実装する

**完了条件**:
- [ ] `src/auth/permission.rs` を作成
- [ ] `Role` 列挙型（Guest, Member, SubOp, SysOp）
- [ ] can_access() 関数
- [ ] check_permission() 関数
- [ ] 単体テスト

**関連ファイル**:
- `src/auth/permission.rs`

---

### 3-7. プロフィール管理

**概要**: ユーザープロフィールの閲覧・編集を実装する

**完了条件**:
- [ ] get_profile() 関数
- [ ] update_profile() 関数
- [ ] change_password() 関数
- [ ] 単体テスト

**関連ファイル**:
- `src/auth/mod.rs`（拡張）

---

## Phase 4: 掲示板機能

### 4-1. 掲示板テーブル実装

**概要**: boards テーブルとリポジトリを実装する

**完了条件**:
- [ ] マイグレーション追加（boards テーブル）
- [ ] 掲示板 CRUD 操作
- [ ] 権限による絞り込み
- [ ] 単体テスト

**関連ファイル**:
- `src/db/migrations/002_boards.sql`
- `src/board/mod.rs`
- `src/board/repository.rs`

---

### 4-2. スレッド・投稿テーブル実装

**概要**: threads, posts テーブルとリポジトリを実装する

**完了条件**:
- [ ] マイグレーション追加
- [ ] スレッド CRUD
- [ ] 投稿 CRUD
- [ ] スレッド形式 / フラット形式の区別
- [ ] 単体テスト

**関連ファイル**:
- `src/db/migrations/003_threads_posts.sql`
- `src/board/thread.rs`
- `src/board/post.rs`

---

### 4-3. 掲示板一覧・詳細表示

**概要**: 掲示板一覧と詳細の取得ロジックを実装する

**完了条件**:
- [ ] list_boards() 関数
- [ ] get_board() 関数
- [ ] ページング対応
- [ ] 単体テスト

**関連ファイル**:
- `src/board/mod.rs`

---

### 4-4. スレッド・投稿の作成と削除

**概要**: 投稿の作成・削除ロジックを実装する

**完了条件**:
- [ ] create_thread() 関数
- [ ] create_post() 関数
- [ ] delete_post() 関数（権限チェック付き）
- [ ] スレッドの updated_at 更新
- [ ] 単体テスト

**関連ファイル**:
- `src/board/mod.rs`

---

### 4-5. 未読管理

**概要**: 未読管理機能を実装する

**完了条件**:
- [ ] read_positions テーブル実装
- [ ] get_unread_count() 関数
- [ ] mark_as_read() 関数
- [ ] 未読一気読み用の get_unread_posts() 関数
- [ ] 単体テスト

**関連ファイル**:
- `src/db/migrations/004_read_positions.sql`
- `src/board/unread.rs`

---

## Phase 5: チャット機能

### 5-1. チャットルーム基盤

**概要**: チャットルームとブロードキャストを実装する

**完了条件**:
- [ ] `src/chat/mod.rs`, `src/chat/room.rs` を作成
- [ ] `ChatRoom` 構造体
- [ ] tokio::sync::broadcast によるメッセージ配信
- [ ] 参加者管理
- [ ] 単体テスト

**関連ファイル**:
- `src/chat/mod.rs`
- `src/chat/room.rs`

---

### 5-2. チャットコマンド処理

**概要**: /quit, /who, /me などのコマンドを実装する

**完了条件**:
- [ ] コマンドパーサー
- [ ] /quit（退室）
- [ ] /who（参加者一覧）
- [ ] /me（アクション）
- [ ] /help
- [ ] 単体テスト

**関連ファイル**:
- `src/chat/command.rs`

---

### 5-3. チャットログ保存

**概要**: チャットログのDB保存を実装する

**完了条件**:
- [ ] chat_logs テーブル実装
- [ ] ログ保存機能
- [ ] 直近ログ取得（入室時表示用）
- [ ] 単体テスト

**関連ファイル**:
- `src/db/migrations/005_chat_logs.sql`
- `src/chat/log.rs`

---

## Phase 6: メール機能

### 6-1. メールテーブル実装

**概要**: mails テーブルとリポジトリを実装する

**完了条件**:
- [ ] マイグレーション追加
- [ ] メール CRUD
- [ ] 論理削除対応
- [ ] 単体テスト

**関連ファイル**:
- `src/db/migrations/006_mails.sql`
- `src/mail/mod.rs`
- `src/mail/repository.rs`

---

### 6-2. メール送受信

**概要**: メールの送受信ロジックを実装する

**完了条件**:
- [ ] send_mail() 関数
- [ ] list_inbox() / list_sent() 関数
- [ ] get_mail() 関数（既読化含む）
- [ ] delete_mail() 関数
- [ ] 単体テスト

**関連ファイル**:
- `src/mail/mod.rs`

---

### 6-3. システムメール

**概要**: ウェルカムメールなどのシステムメールを実装する

**完了条件**:
- [ ] send_welcome_mail() 関数
- [ ] SysOp ユーザーの取得方法
- [ ] 単体テスト

**関連ファイル**:
- `src/mail/system.rs`

---

## Phase 7: ファイル管理

### 7-1. フォルダ・ファイルテーブル実装

**概要**: folders, files テーブルとリポジトリを実装する

**完了条件**:
- [ ] マイグレーション追加
- [ ] フォルダ CRUD
- [ ] ファイルメタデータ CRUD
- [ ] 階層構造対応
- [ ] 単体テスト

**関連ファイル**:
- `src/db/migrations/007_files.sql`
- `src/file/mod.rs`
- `src/file/folder.rs`

---

### 7-2. ファイルストレージ

**概要**: ファイルの物理保存を実装する

**完了条件**:
- [ ] ファイル保存（UUID ベース）
- [ ] ディレクトリ分割（先頭2文字）
- [ ] ファイル読み込み
- [ ] ファイル削除
- [ ] 単体テスト

**関連ファイル**:
- `src/file/storage.rs`

---

### 7-3. アップロード・ダウンロード

**概要**: アップロード・ダウンロードのロジックを実装する

**完了条件**:
- [ ] upload() 関数
- [ ] download() 関数（ダウンロードカウント更新）
- [ ] 権限チェック
- [ ] サイズ制限
- [ ] 単体テスト

**関連ファイル**:
- `src/file/transfer.rs`

---

## Phase 8: 管理機能

### 8-1. 管理メニュー基盤

**概要**: 管理メニューの基盤を実装する

**完了条件**:
- [ ] `src/admin/mod.rs` を作成
- [ ] 権限チェック（SubOp/SysOp）
- [ ] 管理メニュー構造
- [ ] 単体テスト

**関連ファイル**:
- `src/admin/mod.rs`

---

### 8-2. 掲示板・フォルダ管理

**概要**: 掲示板・フォルダの管理機能を実装する

**完了条件**:
- [ ] 掲示板の追加・編集・削除
- [ ] フォルダの追加・編集・削除
- [ ] 権限による制限（SysOpのみ削除可能等）
- [ ] 単体テスト

**関連ファイル**:
- `src/admin/board.rs`
- `src/admin/folder.rs`

---

### 8-3. ユーザー管理

**概要**: ユーザー管理機能を実装する

**完了条件**:
- [ ] ユーザー一覧
- [ ] ユーザー編集（SubOpは一般会員のみ）
- [ ] 権限変更（SysOpのみ）
- [ ] アカウント停止/復活
- [ ] 単体テスト

**関連ファイル**:
- `src/admin/user.rs`

---

### 8-4. 接続ユーザー管理

**概要**: 接続中ユーザーの管理機能を実装する

**完了条件**:
- [ ] 接続中ユーザー一覧
- [ ] 強制切断（SysOpのみ）
- [ ] 単体テスト

**関連ファイル**:
- `src/admin/session.rs`

---

## Phase 9: テンプレート・国際化

### 9-1. 言語リソース読み込み

**概要**: TOML 形式の言語リソースを読み込む

**完了条件**:
- [ ] `src/template/i18n.rs` を作成
- [ ] `I18n` 構造体
- [ ] translate() / translate_with() 関数
- [ ] locales/ja.toml, locales/en.toml を作成
- [ ] 単体テスト

**関連ファイル**:
- `src/template/i18n.rs`
- `locales/ja.toml`
- `locales/en.toml`

---

### 9-2. テンプレートエンジン

**概要**: 変数展開と条件分岐をサポートするテンプレートエンジンを実装する

**完了条件**:
- [ ] `src/template/mod.rs`, `src/template/renderer.rs` を作成
- [ ] 変数展開 `{{変数名}}`
- [ ] 翻訳参照 `{{t "キー"}}`
- [ ] 条件分岐 `{{#if 条件}}...{{/if}}`
- [ ] 単体テスト

**関連ファイル**:
- `src/template/mod.rs`
- `src/template/renderer.rs`

---

### 9-3. テンプレートファイル作成

**概要**: 画面テンプレートファイルを作成する

**完了条件**:
- [ ] templates/80/ ディレクトリ（welcome.txt, main_menu.txt, help.txt）
- [ ] templates/40/ ディレクトリ（同上）
- [ ] 端末幅による自動選択
- [ ] 動作確認

**関連ファイル**:
- `templates/80/*.txt`
- `templates/40/*.txt`
- `src/template/loader.rs`

---

### 9-4. 画面表示・ANSI対応

**概要**: ANSI エスケープシーケンスによる画面装飾を実装する

**完了条件**:
- [ ] `src/screen/mod.rs`, `src/screen/ansi.rs` を作成
- [ ] 色定数（Color 列挙型）
- [ ] color_text(), goto() などのヘルパー
- [ ] ANSI 無効時のプレーンテキスト対応
- [ ] 単体テスト

**関連ファイル**:
- `src/screen/mod.rs`
- `src/screen/ansi.rs`
- `src/screen/plain.rs`

---

## Phase 10: 統合・調整

### 10-1. メインメニュー統合

**概要**: 各機能をメインメニューから呼び出せるようにする

**完了条件**:
- [ ] メインメニュー実装
- [ ] 各機能への遷移
- [ ] ゲストモード対応
- [ ] 統合テスト

**関連ファイル**:
- `src/screen/menu.rs`
- `src/main.rs`

---

### 10-2. 画面遷移の実装

**概要**: 全画面の遷移を実装する

**完了条件**:
- [ ] ウェルカム画面
- [ ] ログイン/新規登録画面
- [ ] 各機能の画面
- [ ] 統合テスト

**関連ファイル**:
- `src/screen/*.rs`

---

### 10-3. E2Eテスト

**概要**: エンドツーエンドテストを作成する

**完了条件**:
- [ ] Telnet 接続テスト
- [ ] ログイン/ログアウトテスト
- [ ] 掲示板投稿テスト
- [ ] チャットテスト
- [ ] テスト自動化

**関連ファイル**:
- `tests/e2e/*.rs`

---

### 10-4. ドキュメント・README整備

**概要**: README と運用ドキュメントを整備する

**完了条件**:
- [ ] README.md 作成（インストール方法、使い方）
- [ ] 運用ガイド
- [ ] CLAUDE.md の最終更新

**関連ファイル**:
- `README.md`
- `CLAUDE.md`

---

## 備考

- 各タスクはTDDで進める（テスト先行）
- 1 issue = 1 PR を原則とする
- ブランチ命名: `feature/issue-番号-簡潔な説明`
- フェーズ完了時に振り返りを行い、次フェーズの計画を調整する
