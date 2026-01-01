# HOBBS 実装計画

## 概要

本ドキュメントは、HOBBSの実装計画をフェーズごとにまとめたものです。
各タスクはGitHub issueの粒度（半日〜1日で完了）で記載しています。

## 開発フロー

1. 人間がissue登録を指示（直近1-2フェーズ分のみ）
2. 人間がissueを指示 → AI開発 → ブランチ → PR → レビュー
3. フェーズ完了後、次フェーズのissue登録を検討

## フェーズ一覧

| Phase | 名称 | 概要 | 依存 | 状態 |
|-------|------|------|------|------|
| 1 | プロジェクト基盤 | Cargo.toml, エラー型, 設定, ログ | - | ✅ |
| 2 | Telnetサーバ基盤 | 接続受付, セッション, 文字コード | Phase 1 | ✅ |
| 3 | データベース・認証 | DB基盤, ユーザー管理, ログイン | Phase 2 | ✅ |
| 3.5 | エンコーディング対応 | ShiftJIS/UTF-8選択、ユーザー設定 | Phase 3 | ✅ |
| 4 | 掲示板機能 | 掲示板, スレッド, 投稿, 未読 | Phase 3.5 | ✅ |
| 5 | チャット機能 | チャットルーム, ブロードキャスト基盤 | Phase 3.5 | ✅ |
| 6 | メール機能 | 内部メール送受信 | Phase 3.5 | ✅ |
| 7 | ファイル管理 | フォルダ, アップロード/ダウンロード | Phase 3.5 | ✅ |
| 8 | 管理機能 | 管理メニュー, ユーザー/コンテンツ管理 | Phase 4-7 | ✅ |
| 9 | テンプレート・国際化 | テンプレートエンジン, i18n | Phase 2 | ✅ |
| 10 | 統合・調整 | 画面遷移, E2Eテスト | Phase 1-9 | ✅ |
| 10.5 | 言語・エンコーディング動的選択 | 接続時の言語選択、ユーザー設定適用 | Phase 10 | ✅ |
| 10.6 | チャット画面統合 | ChatScreenからルーム入室・発言 | Phase 5, 10 | ✅ |
| 10.7 | エンコーディング変換修正 | ScreenContextのエンコーディング引き継ぎ | Phase 10.5 | ✅ |
| 10.8 | 端末プロファイル選択 | 40/80カラム切り替え、ログイン時適用 | Phase 10 | ✅ |
| 10.9 | 掲示板未読管理UI | 未読数表示、一気読み、既読マーク | Phase 4, 10 | ✅ |
| 11+ | 将来拡張 | SSH対応, WebSocket等 | Phase 10.9 |

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

## Phase 2: Telnetサーバ基盤 ✅

### 2-1. TCP接続受付 ✅

**概要**: tokio を使用した TCP リスナーを実装する

**完了条件**:
- [x] `src/server/mod.rs`, `src/server/listener.rs` を作成
- [x] `TelnetServer` 構造体を定義
- [x] 指定ポートで接続を受け付ける
- [x] 最大接続数の制御
- [x] 統合テスト（接続・切断）

**関連ファイル**:
- `src/server/mod.rs`
- `src/server/listener.rs`

---

### 2-2. セッション管理基盤 ✅

**概要**: 接続ごとのセッション管理を実装する

**完了条件**:
- [x] `src/server/session.rs` を作成
- [x] `TelnetSession` 構造体を定義
- [x] セッション状態（SessionState）の定義
- [x] アイドルタイムアウト処理
- [x] セッション一覧管理（HashMap）
- [x] 単体テスト

**関連ファイル**:
- `src/server/session.rs`
- `src/server/mod.rs`

---

### 2-3. ShiftJIS変換 ✅

**概要**: encoding_rs を使用した文字コード変換を実装する

**完了条件**:
- [x] `src/server/encoding.rs` を作成
- [x] ShiftJIS → UTF-8 変換（受信用）
- [x] UTF-8 → ShiftJIS 変換（送信用）
- [x] 変換エラーのハンドリング
- [x] 単体テスト（日本語文字列、制御文字）

**関連ファイル**:
- `src/server/encoding.rs`

---

### 2-4. 端末プロファイル ✅

**概要**: 端末タイプ（80x24, C64等）を管理する

**完了条件**:
- [x] `src/terminal/mod.rs`, `src/terminal/profile.rs` を作成
- [x] `TerminalProfile` 構造体を定義
- [x] standard(), c64(), c64_ansi() プリセット
- [x] display_width() 関数（全角幅計算）
- [x] truncate_to_width() 関数
- [x] 単体テスト

**関連ファイル**:
- `src/terminal/mod.rs`
- `src/terminal/profile.rs`

---

### 2-5. 入力処理・Telnetプロトコル ✅

**概要**: Telnet IAC コマンドと入力処理を実装する

**完了条件**:
- [x] Telnet ネゴシエーション（ECHO, SGA）
- [x] 行バッファリング入力
- [x] バックスペース、Ctrl+C 処理
- [x] パスワード入力（エコーなし）
- [x] 統合テスト

**関連ファイル**:
- `src/server/telnet.rs`
- `src/server/input.rs`

---

## Phase 3: データベース・認証 ✅

### 3-1. データベース基盤 ✅

**概要**: SQLite 接続とマイグレーション機能を実装する

**完了条件**:
- [x] `src/db/mod.rs`, `src/db/schema.rs` を作成
- [x] `Database` 構造体（接続プール的な管理）
- [x] マイグレーション機能
- [x] schema_version テーブル
- [x] 初期スキーマ（users テーブル）
- [x] 単体テスト

**関連ファイル**:
- `src/db/mod.rs`
- `src/db/schema.rs`
- `src/db/migrations/001_initial.sql`

---

### 3-2. ユーザーリポジトリ ✅

**概要**: ユーザーの CRUD 操作を実装する

**完了条件**:
- [x] `src/db/repository.rs` または `src/auth/repository.rs` を作成
- [x] ユーザー作成・取得・更新・削除
- [x] ユーザー名による検索
- [x] 単体テスト

**関連ファイル**:
- `src/db/repository.rs`
- `src/auth/user.rs`

---

### 3-3. パスワードハッシュ ✅

**概要**: Argon2id によるパスワードハッシュを実装する

**完了条件**:
- [x] `src/auth/password.rs` を作成
- [x] hash_password() 関数
- [x] verify_password() 関数
- [x] 単体テスト

**関連ファイル**:
- `src/auth/password.rs`

---

### 3-4. ログイン・ログアウト ✅

**概要**: ログイン認証とセッション作成を実装する

**完了条件**:
- [x] `src/auth/mod.rs`, `src/auth/session.rs` を作成
- [x] `AuthSession` 構造体
- [x] login() 関数（認証 + セッション作成）
- [x] logout() 関数
- [x] セッション検証
- [x] ログイン試行制限（3回、5分ロック）
- [x] 単体テスト

**関連ファイル**:
- `src/auth/mod.rs`
- `src/auth/session.rs`

---

### 3-5. 新規登録 ✅

**概要**: ユーザー登録機能を実装する

**完了条件**:
- [x] register() 関数
- [x] バリデーション（ユーザーID、パスワード、ニックネーム）
- [x] 禁止ユーザーIDチェック
- [x] 重複チェック
- [x] 単体テスト

**関連ファイル**:
- `src/auth/mod.rs`（拡張）
- `src/auth/validation.rs`

---

### 3-6. 権限チェック ✅

**概要**: Role ベースの権限チェックを実装する

**完了条件**:
- [x] `src/auth/permission.rs` を作成
- [x] `Role` 列挙型（Guest, Member, SubOp, SysOp）
- [x] can_access() 関数
- [x] check_permission() 関数
- [x] 単体テスト

**関連ファイル**:
- `src/auth/permission.rs`

---

### 3-7. プロフィール管理 ✅

**概要**: ユーザープロフィールの閲覧・編集を実装する

**完了条件**:
- [x] get_profile() 関数
- [x] update_profile() 関数
- [x] change_password() 関数
- [x] 単体テスト

**関連ファイル**:
- `src/auth/mod.rs`（拡張）

---

## Phase 3.5: エンコーディング対応 ✅

### 3.5-1. CharacterEncoding基盤 ✅

**概要**: 文字エンコーディング選択の基盤を実装する

**完了条件**:
- [x] `CharacterEncoding` enum定義（ShiftJIS, UTF-8）
- [x] `src/server/encoding.rs` にエンコーディング判定ヘルパー追加
- [x] `encode_for_client()`, `decode_from_client()` 関数
- [x] 単体テスト

**関連ファイル**:
- `src/server/encoding.rs`

---

### 3.5-2. セッションエンコーディング対応 ✅

**概要**: Telnetセッションでエンコーディング設定を使用する

**完了条件**:
- [x] `TelnetSession` に `encoding` フィールド追加
- [x] `LineBuffer` のエンコーディング対応
- [x] 入出力処理のエンコーディング変換
- [x] 統合テスト

**関連ファイル**:
- `src/server/session.rs`
- `src/server/input.rs`

---

### 3.5-3. ユーザーエンコーディング設定 ✅

**概要**: ユーザー設定でエンコーディングを選択・保存する

**完了条件**:
- [x] `users` テーブルに `encoding` カラム追加（マイグレーション）
- [x] `User`, `NewUser`, `UserUpdate` 構造体に `encoding` フィールド追加
- [x] `UserRepository` 更新
- [x] ログイン時のエンコーディング設定読み込み
- [x] ゲスト用のエンコーディング選択（セッションのみ）
- [x] 単体テスト

**関連ファイル**:
- `src/db/schema.rs` (マイグレーション追加)
- `src/db/user.rs`
- `src/db/repository.rs`

---

## Phase 4: 掲示板機能 ✅

### 4-1. 掲示板テーブル実装 ✅

**概要**: boards テーブルとリポジトリを実装する

**完了条件**:
- [x] マイグレーション追加（boards テーブル）
- [x] 掲示板 CRUD 操作
- [x] 権限による絞り込み
- [x] 単体テスト

**関連ファイル**:
- `src/db/schema.rs` (マイグレーション v3)
- `src/board/mod.rs`
- `src/board/repository.rs`
- `src/board/types.rs`

---

### 4-2. スレッド・投稿テーブル実装 ✅

**概要**: threads, posts テーブルとリポジトリを実装する

**完了条件**:
- [x] マイグレーション追加
- [x] スレッド CRUD
- [x] 投稿 CRUD
- [x] スレッド形式 / フラット形式の区別
- [x] 単体テスト

**関連ファイル**:
- `src/db/schema.rs` (マイグレーション v4, v5)
- `src/board/thread.rs`
- `src/board/thread_repository.rs`
- `src/board/post.rs`
- `src/board/post_repository.rs`

---

### 4-3. 掲示板一覧・詳細表示 ✅

**概要**: 掲示板一覧と詳細の取得ロジックを実装する

**完了条件**:
- [x] list_boards() 関数
- [x] get_board() 関数
- [x] ページング対応
- [x] 単体テスト

**関連ファイル**:
- `src/board/service.rs`

---

### 4-4. スレッド・投稿の作成と削除 ✅

**概要**: 投稿の作成・削除ロジックを実装する

**完了条件**:
- [x] create_thread() 関数
- [x] create_post() 関数
- [x] delete_post() 関数（権限チェック付き）
- [x] スレッドの updated_at 更新
- [x] post_count の自動更新
- [x] 単体テスト

**関連ファイル**:
- `src/board/service.rs`

---

### 4-5. 未読管理 ✅

**概要**: 未読管理機能を実装する

**完了条件**:
- [x] read_positions テーブル実装
- [x] get_unread_count() 関数
- [x] mark_as_read() 関数
- [x] 未読一気読み用の get_unread_posts() 関数
- [x] 単体テスト

**関連ファイル**:
- `src/db/schema.rs` (マイグレーション v6)
- `src/board/unread.rs`

---

## Phase 5: チャット機能 ✅

### 5-1. チャットルーム基盤 ✅

**概要**: チャットルームとブロードキャストを実装する

**完了条件**:
- [x] `src/chat/mod.rs`, `src/chat/room.rs` を作成
- [x] `ChatRoom` 構造体
- [x] tokio::sync::broadcast によるメッセージ配信
- [x] 参加者管理
- [x] 単体テスト

**関連ファイル**:
- `src/chat/mod.rs`
- `src/chat/room.rs`

---

### 5-2. チャットコマンド処理 ✅

**概要**: /quit, /who, /me などのコマンドを実装する

**完了条件**:
- [x] コマンドパーサー
- [x] /quit（退室）
- [x] /who（参加者一覧）
- [x] /me（アクション）
- [x] /help
- [x] 単体テスト

**関連ファイル**:
- `src/chat/command.rs`

---

### 5-3. チャットログ保存 ✅

**概要**: チャットログのDB保存を実装する

**完了条件**:
- [x] chat_logs テーブル実装
- [x] ログ保存機能
- [x] 直近ログ取得（入室時表示用）
- [x] 単体テスト

**関連ファイル**:
- `src/db/schema.rs` (マイグレーション v7)
- `src/chat/log.rs`

---

## Phase 6: メール機能 ✅

### 6-1. メールテーブル実装 ✅

**概要**: mails テーブルとリポジトリを実装する

**完了条件**:
- [x] マイグレーション追加
- [x] メール CRUD
- [x] 論理削除対応
- [x] 単体テスト

**関連ファイル**:
- `src/db/schema.rs` (マイグレーション v8)
- `src/mail/mod.rs`
- `src/mail/repository.rs`
- `src/mail/types.rs`

---

### 6-2. メール送受信 ✅

**概要**: メールの送受信ロジックを実装する

**完了条件**:
- [x] send_mail() 関数
- [x] list_inbox() / list_sent() 関数
- [x] get_mail() 関数（既読化含む）
- [x] delete_mail() 関数
- [x] 単体テスト

**関連ファイル**:
- `src/mail/service.rs`

---

### 6-3. システムメール ✅

**概要**: ウェルカムメールなどのシステムメールを実装する

**完了条件**:
- [x] send_welcome_mail() 関数
- [x] get_sysop_user() ヘルパー
- [x] send_notification() / broadcast_notification() 関数
- [x] 単体テスト

**関連ファイル**:
- `src/mail/system.rs`

---

## Phase 7: ファイル管理 ✅

### 7-1. フォルダ・ファイルテーブル実装 ✅

**概要**: folders, files テーブルとリポジトリを実装する

**完了条件**:
- [x] マイグレーション追加
- [x] フォルダ CRUD
- [x] ファイルメタデータ CRUD
- [x] 階層構造対応
- [x] 単体テスト

**関連ファイル**:
- `src/db/schema.rs` (マイグレーション v9, v10)
- `src/file/mod.rs`
- `src/file/folder.rs`
- `src/file/metadata.rs`

---

### 7-2. ファイルストレージ ✅

**概要**: ファイルの物理保存を実装する

**完了条件**:
- [x] ファイル保存（UUID ベース）
- [x] ディレクトリ分割（先頭2文字）
- [x] ファイル読み込み
- [x] ファイル削除
- [x] 単体テスト

**関連ファイル**:
- `src/file/storage.rs`

---

### 7-3. アップロード・ダウンロード ✅

**概要**: アップロード・ダウンロードのロジックを実装する

**完了条件**:
- [x] upload() 関数
- [x] download() 関数（ダウンロードカウント更新）
- [x] 権限チェック
- [x] サイズ制限
- [x] 単体テスト

**関連ファイル**:
- `src/file/service.rs`

---

## Phase 8: 管理機能 ✅

### 8-1. 管理メニュー基盤 ✅

**概要**: 管理メニューの基盤を実装する

**完了条件**:
- [x] `src/admin/mod.rs` を作成
- [x] 権限チェック（SubOp/SysOp）
- [x] 管理メニュー構造（AdminService, AdminError）
- [x] 権限ヘルパー（require_admin, is_admin, is_sysop, can_change_role, can_edit_user）
- [x] 単体テスト

**関連ファイル**:
- `src/admin/mod.rs`

---

### 8-2. 掲示板・フォルダ管理 ✅

**概要**: 掲示板・フォルダの管理機能を実装する

**完了条件**:
- [x] 掲示板の追加・編集・削除（BoardAdminService）
- [x] フォルダの追加・編集・削除（FolderAdminService）
- [x] コンテンツ管理（ContentAdminService: 投稿・ファイル削除）
- [x] 権限による制限（SysOpのみ削除可能等）
- [x] 単体テスト

**関連ファイル**:
- `src/admin/board.rs`
- `src/admin/folder.rs`
- `src/admin/content.rs`

---

### 8-3. ユーザー管理 ✅

**概要**: ユーザー管理機能を実装する

**完了条件**:
- [x] ユーザー一覧（ページネーション対応）
- [x] ユーザー詳細（UserDetail: 投稿数・ファイル数・メール数）
- [x] ユーザー編集（SubOpは一般会員のみ）
- [x] パスワードリセット（ランダム生成）
- [x] 権限変更（SysOpのみ）
- [x] アカウント停止/復活
- [x] ユーザー検索
- [x] 単体テスト

**関連ファイル**:
- `src/admin/user.rs`

---

### 8-4. 接続ユーザー管理 ✅

**概要**: 接続中ユーザーの管理機能を実装する

**完了条件**:
- [x] 接続中ユーザー一覧（SessionAdminService）
- [x] セッション統計（SessionStatistics）
- [x] 強制切断（SysOpのみ）
- [x] 強制切断シグナリング機能（SessionManager拡張）
- [x] ヘルパー関数（format_session_state, format_duration）
- [x] 単体テスト

**関連ファイル**:
- `src/admin/session.rs`
- `src/server/session.rs`（拡張）

---

## Phase 9: テンプレート・国際化 ✅

### 9-1. 言語リソース読み込み ✅

**概要**: TOML 形式の言語リソースを読み込む

**完了条件**:
- [x] `src/i18n/mod.rs` を作成
- [x] `I18n` 構造体
- [x] t() / t_with() 関数
- [x] locales/ja.toml, locales/en.toml を作成
- [x] 単体テスト

**関連ファイル**:
- `src/i18n/mod.rs`
- `locales/ja.toml`
- `locales/en.toml`

---

### 9-2. テンプレートエンジン ✅

**概要**: 変数展開と条件分岐をサポートするテンプレートエンジンを実装する

**完了条件**:
- [x] `src/template/mod.rs`, `src/template/renderer.rs` を作成
- [x] 変数展開 `{{変数名}}`
- [x] 翻訳参照 `{{t "キー"}}`
- [x] 条件分岐 `{{#if 条件}}...{{/if}}`
- [x] ループ `{{#each items}}...{{/each}}`
- [x] 単体テスト

**関連ファイル**:
- `src/template/mod.rs`
- `src/template/engine.rs`
- `src/template/parser.rs`
- `src/template/renderer.rs`
- `src/template/context.rs`
- `src/template/value.rs`

---

### 9-3. テンプレートファイル作成 ✅

**概要**: 画面テンプレートファイルを作成する

**完了条件**:
- [x] templates/80/ ディレクトリ（welcome.txt, main_menu.txt, help.txt）
- [x] templates/40/ ディレクトリ（同上）
- [x] 掲示板・チャット・メール・ファイル・管理テンプレート
- [x] 端末幅による自動選択
- [x] 動作確認

**関連ファイル**:
- `templates/80/*.txt`
- `templates/40/*.txt`
- `src/template/loader.rs`

---

### 9-4. 画面表示・ANSI対応 ✅

**概要**: ANSI エスケープシーケンスによる画面装飾を実装する

**完了条件**:
- [x] `src/screen/mod.rs`, `src/screen/ansi.rs` を作成
- [x] 色定数（Color 列挙型）
- [x] color_text(), goto() などのヘルパー
- [x] ANSI 無効時のプレーンテキスト対応
- [x] 単体テスト

**関連ファイル**:
- `src/screen/mod.rs`
- `src/screen/ansi.rs`
- `src/screen/plain.rs`

---

## Phase 10: 統合・調整 ✅

### 10-1. メインメニュー統合 ✅

**概要**: 各機能をメインメニューから呼び出せるようにする

**完了条件**:
- [x] メインメニュー実装
- [x] 各機能への遷移
- [x] ゲストモード対応
- [x] 統合テスト

**関連ファイル**:
- `src/app/session_handler.rs`
- `src/main.rs`

---

### 10-2. 画面遷移の実装 ✅

**概要**: 全画面の遷移を実装する

**完了条件**:
- [x] ウェルカム画面
- [x] ログイン/新規登録画面
- [x] 各機能の画面
- [x] 統合テスト

**関連ファイル**:
- `src/app/screens/*.rs`

---

### 10-3. E2Eテスト ✅

**概要**: エンドツーエンドテストを作成する

**完了条件**:
- [x] Telnet 接続テスト
- [x] ログイン/ログアウトテスト
- [x] 掲示板投稿テスト
- [x] チャットテスト
- [x] テスト自動化

**関連ファイル**:
- `tests/e2e_*.rs`

---

### 10-4. ドキュメント・README整備 ✅

**概要**: README と運用ドキュメントを整備する

**完了条件**:
- [x] README.md 作成（インストール方法、使い方）
- [x] 運用ガイド
- [x] CLAUDE.md の最終更新

**関連ファイル**:
- `README.md`
- `CLAUDE.md`

---

## Phase 10.5: 言語・エンコーディング動的選択 ✅

### 10.5-1. usersテーブルにlanguageカラム追加 ✅

**概要**: ユーザーごとの言語設定を保存するため、usersテーブルにlanguageカラムを追加する

**完了条件**:
- [x] マイグレーション追加（ALTER TABLE users ADD COLUMN language）
- [x] User, NewUser, UserUpdate構造体にlanguageフィールド追加
- [x] UserRepository更新
- [x] 単体テスト

**関連ファイル**:
- `src/db/schema.rs`
- `src/db/user.rs`
- `src/db/repository.rs`

---

### 10.5-2. ウェルカム画面で言語/エンコーディング選択 ✅

**概要**: 接続直後に言語とエンコーディングを選択できるようにする

**画面イメージ**:
```
Select language / 言語選択:
[E] English (UTF-8)
[J] Japanese / 日本語 (ShiftJIS)
[U] Japanese / 日本語 (UTF-8)
```

**完了条件**:
- [x] ウェルカム画面の前に選択画面を追加
- [x] セッションに選択結果を適用（encoding, i18n）
- [x] 選択後にウェルカム画面を表示
- [x] 統合テスト

**関連ファイル**:
- `src/app/session_handler.rs`

---

### 10.5-3. SessionHandlerの言語動的変更対応 ✅

**概要**: SessionHandlerで言語（i18n）を動的に変更できるようにする

**完了条件**:
- [x] SessionHandlerにset_language()メソッド追加
- [x] i18nフィールドを変更可能に（Arc<I18n>のまま差し替え）
- [x] ScreenContextへの反映
- [x] 単体テスト

**関連ファイル**:
- `src/app/session_handler.rs`
- `src/app/screens/common.rs`

---

### 10.5-4. ログイン時のユーザー設定適用 ✅

**概要**: ログイン時にDBからユーザーのlanguage/encoding設定を読み込み、セッションに適用する

**完了条件**:
- [x] ログイン処理でユーザー設定を読み込み
- [x] セッションのencoding/i18nを更新
- [x] ゲストの場合はウェルカム選択を維持
- [x] 統合テスト

**関連ファイル**:
- `src/app/session_handler.rs`

---

### 10.5-5. 設定画面での言語/エンコーディング変更 ✅

**概要**: プロフィール編集または設定画面で言語/エンコーディングを変更・保存できるようにする

**完了条件**:
- [x] 設定変更画面の実装
- [x] DB保存処理
- [x] 即時反映（セッション更新）
- [x] 統合テスト

**関連ファイル**:
- `src/app/screens/profile.rs` または `src/app/screens/settings.rs`

---

## Phase 10.6: チャット画面統合 ✅

### 10.6-1. チャットルーム入室機能 ✅

**概要**: ChatScreen からチャットルームへの入室・発言機能を実装する

**背景**: Phase 5 でチャット基盤（ChatRoom, ChatRoomManager）は実装済みだが、ChatScreen からの呼び出しが未実装のまま「not implemented」と表示されている。

**完了条件**:
- [x] ChatScreen から ChatRoomManager への接続
- [x] チャットルームへの入室処理
- [x] メッセージ送受信の実装
- [x] /quit, /who, /me コマンドの動作確認
- [x] 統合テスト

**関連ファイル**:
- `src/app/screens/chat.rs`
- `src/chat/room.rs`
- `src/chat/manager.rs`

---

### 10.6-2. チャットルーム作成・管理 ✅

**概要**: チャットルームの作成と管理機能を実装する

**完了条件**:
- [x] 管理画面からのチャットルーム作成
- [x] デフォルトルームの自動作成
- [x] ルームの削除・編集機能
- [x] 単体テスト

**関連ファイル**:
- `src/admin/chat.rs`
- `src/chat/manager.rs`

---

## Phase 10.7: エンコーディング変換の修正 ✅

### 10.7-1. ScreenContextのエンコーディング引き継ぎ ✅

**概要**: ScreenContext 作成時に SessionHandler のエンコーディング設定を引き継ぐよう修正する

**背景**: 現在 ScreenContext は独立した LineBuffer を持ち、毎回 ShiftJIS で初期化されるため、UTF-8 ユーザーが投稿した際にデータが破損する。

**問題の詳細**:
1. ScreenContext::new() で LineBuffer が CharacterEncoding::default() (ShiftJIS) で初期化される
2. SessionHandler.line_buffer のエンコーディング設定が ScreenContext に渡されない
3. UTF-8 ユーザーの入力が ShiftJIS として解釈され、データ破損が発生

**完了条件**:
- [x] ScreenContext::new() にエンコーディングパラメータを追加
- [x] SessionHandler::create_screen_context() で現在のエンコーディングを渡す
- [x] 入出力のエンコーディング変換を検証
- [x] 単体テスト・統合テスト

**関連ファイル**:
- `src/app/screens/common.rs`
- `src/app/session_handler.rs`

---

### 10.7-2. 入出力エンコーディング検証 ✅

**概要**: 掲示板、メール、チャット等での入出力エンコーディング変換を検証・修正する

**完了条件**:
- [x] ShiftJIS → 投稿 → ShiftJIS で正常表示
- [x] ShiftJIS → 投稿 → UTF-8 で正常表示
- [x] UTF-8 → 投稿 → UTF-8 で正常表示
- [x] UTF-8 → 投稿 → ShiftJIS で正常表示
- [x] E2Eテスト追加

**関連ファイル**:
- `tests/e2e_encoding.rs`

---

## Phase 10.8: 端末プロファイル選択 ✅

### 10.8-1. config.tomlにデフォルト端末プロファイル設定を追加 ✅

**概要**: システム全体のデフォルト端末プロファイルをconfig.tomlで設定可能にする

**完了条件**:
- [x] `TerminalConfig` 構造体を追加（default_profile: "standard" | "c64" | "c64_ansi"）
- [x] `Config` 構造体に `terminal` フィールドを追加
- [x] デフォルト値（"standard"）を設定
- [x] 単体テスト

**関連ファイル**:
- `src/config.rs`
- `config.toml`

---

### 10.8-2. SessionHandlerでの端末プロファイル適用 ✅

**概要**: SessionHandlerで端末プロファイルを管理し、画面表示に反映する

**完了条件**:
- [x] SessionHandlerに`terminal_profile`フィールドを追加
- [x] ログイン時にユーザーのterminal設定を読み込みプロファイル適用
- [x] ScreenContextへのプロファイル引き継ぎ
- [x] ゲストユーザーはconfig.tomlのデフォルトを使用
- [x] 統合テスト

**関連ファイル**:
- `src/app/session_handler.rs`
- `src/app/screens/common.rs`

---

### 10.8-3. 設定画面での端末プロファイル変更 ✅

**概要**: ユーザー設定画面で端末プロファイルを選択・保存できるようにする

**画面イメージ**:
```
端末プロファイル:
[1] Standard (80x24)
[2] C64 (40x25, ANSI無効)
[3] C64 ANSI (40x25)
```

**完了条件**:
- [x] プロフィール編集画面または設定画面に端末選択項目を追加
- [x] DBへの保存処理
- [x] 即時反映（セッション更新）
- [x] ローカライズメッセージ追加
- [x] 統合テスト

**関連ファイル**:
- `src/app/screens/profile.rs`
- `locales/en.toml`
- `locales/ja.toml`

---

## Phase 10.9: 掲示板未読管理UI ✅

### 10.9-1. 掲示板一覧に未読数を表示 ✅

**概要**: 掲示板一覧画面で各掲示板の未読投稿数を表示する

**背景**: `UnreadRepository` は既に実装済みだが、`BoardScreen` のUIで使用されていない

**完了条件**:
- [x] 掲示板一覧で未読数を表示（例: `[3件未読]`）
- [x] 未読がある掲示板を視覚的に区別（ANSIカラーまたはマーク）
- [x] ゲストユーザーは未読表示なし
- [x] 単体テスト

**関連ファイル**:
- `src/app/screens/board.rs`
- `src/board/unread.rs`

---

### 10.9-2. 未読一気読み機能 ✅

**概要**: 掲示板内の未読投稿を連続して読む「一気読み」機能を実装する

**画面イメージ**:
```
[N]=次へ [P]=前へ [U]=未読一気読み [A]=全て既読 [W]=新規投稿 [Q]=戻る
```

**完了条件**:
- [x] 掲示板画面に「未読一気読み [U]」オプションを追加
- [x] 未読投稿を順番に表示、各投稿後に次へ進むプロンプト
- [x] 一気読み中に中断可能（[Q]で終了）
- [x] 表示した投稿を既読としてマーク
- [x] 統合テスト

**関連ファイル**:
- `src/app/screens/board.rs`
- `src/board/unread.rs`

---

### 10.9-3. 投稿閲覧時の既読マーク ✅

**概要**: 投稿を閲覧した際に自動的に既読としてマークする

**完了条件**:
- [x] スレッド表示時に表示した投稿を既読マーク
- [x] フラット掲示板の投稿表示時に既読マーク
- [x] ページ送りで新しい投稿を見た際も既読マーク
- [x] 単体テスト

**関連ファイル**:
- `src/app/screens/board.rs`
- `src/board/unread.rs`

---

### 10.9-4. 全て既読にする機能 ✅

**概要**: 掲示板内の全投稿を既読としてマークする機能を実装する

**完了条件**:
- [x] 掲示板画面に「全て既読 [A]」オプションを追加
- [x] 確認プロンプトを表示
- [x] `mark_all_as_read()` を呼び出して実行
- [x] ローカライズメッセージ追加
- [x] 単体テスト

**関連ファイル**:
- `src/app/screens/board.rs`
- `locales/en.toml`
- `locales/ja.toml`

---

### 10.9-5. スレッド・投稿一覧の未読マーク ✅

**概要**: スレッド一覧や投稿一覧で未読のものにマークを表示する

**画面イメージ**:
```
  No.  タイトル                       返信
  ─────────────────────────────────────
* 1    新機能について                 5
  2    バグ報告                       3
* 3    質問があります                 12
```
（`*` が未読マーク）

**完了条件**:
- [x] 未読スレッド/投稿の先頭に `*` または `[N]` マークを表示
- [x] スレッド形式：最後に読んだ投稿以降に新しい投稿があれば未読
- [x] フラット形式：まだ読んでいない投稿は未読
- [x] 単体テスト

**関連ファイル**:
- `src/app/screens/board.rs`
- `src/board/unread.rs`

---

### 10.9-6. 全掲示板の未読一気読み ✅

**概要**: 掲示板一覧画面から、複数の掲示板をまたがって全ての未読投稿を一気読みする機能を実装する

**画面イメージ**:
```
=== 掲示板一覧 ===

  No.  タイトル              投稿数    未読
  ────────────────────────────────────────
  1    一般                    25      [3]
  2    お知らせ                10      [1]

[U]=全未読一気読み [Q]=戻る:
```

**動作**:
1. [U] 選択時、全掲示板の未読投稿を掲示板順に取得
2. 各投稿に掲示板名を表示して順番に表示
3. 表示した投稿は即座に既読マーク
4. 途中で [Q] で中断可能

**完了条件**:
- [x] 掲示板一覧画面に「全未読一気読み [U]」オプションを追加
- [x] 全掲示板から未読投稿を取得するメソッドを実装
- [x] 各投稿表示時に掲示板名を表示
- [x] 表示した投稿を既読としてマーク
- [x] ローカライズメッセージ追加
- [x] 統合テスト

**関連ファイル**:
- `src/app/screens/board.rs`
- `src/board/unread.rs`
- `locales/en.toml`
- `locales/ja.toml`

---

## Phase 11+: 将来拡張（検討）

### 11-1. SSH対応

**概要**: SSHプロトコルでの接続をサポートする

**完了条件**:
- [ ] SSHライブラリの選定（`thrussh`等）
- [ ] `src/server/ssh.rs` の実装
- [ ] SSH用設定項目の追加（ポート番号等）
- [ ] パスワード認証の実装
- [ ] 統合テスト

**関連ファイル**:
- `src/server/ssh.rs`
- `src/config.rs`
- `Cargo.toml`

**備考**:
- 現アーキテクチャはTelnet固有処理を局所化済みのため、後から追加可能
- SSH認証はパスワード認証のみ（公開鍵認証は将来検討）

---

### 11-2. その他の検討事項

以下は必要に応じて検討：

- **WebSocket対応**: ブラウザからのアクセス
- **IPv6対応**: デュアルスタック対応
- **TLS over Telnet**: STARTTLS対応
- **XMODEM/ZMODEM**: バイナリファイル転送

---

## 備考

- 各タスクはTDDで進める（テスト先行）
- 1 issue = 1 PR を原則とする
- ブランチ命名: `feature/issue-番号-簡潔な説明`
- フェーズ完了時に振り返りを行い、次フェーズの計画を調整する
