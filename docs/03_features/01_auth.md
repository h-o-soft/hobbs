# HOBBS - 機能仕様書: 認証・会員管理

## 1. 概要

ユーザーの認証、会員登録、プロフィール管理、権限管理を行う機能群。

## 2. 権限レベル

| レベル | 名称 | 説明 |
|--------|------|------|
| 0 | guest | ゲスト（未登録） |
| 1 | member | 一般会員 |
| 2 | subop | 副管理者 |
| 3 | sysop | システム管理者 |

### 権限による機能制限

| 機能 | guest | member | subop | sysop |
|------|-------|--------|-------|-------|
| 掲示板閲覧 | △* | ○ | ○ | ○ |
| 掲示板投稿 | △* | ○ | ○ | ○ |
| チャット参加 | × | ○ | ○ | ○ |
| メール送受信 | × | ○ | ○ | ○ |
| ファイル閲覧 | △* | ○ | ○ | ○ |
| ファイルアップロード | × | △* | ○ | ○ |
| 投稿削除（他人） | × | × | ○ | ○ |
| ユーザー管理 | × | × | △* | ○ |
| 掲示板管理 | × | × | ○ | ○ |
| システム設定 | × | × | × | ○ |

△* = 設定により制限される場合あり（掲示板/フォルダごとの権限設定に依存）

## 3. ログイン機能

### 3.1 ログインフロー

```
1. ユーザーID入力
2. パスワード入力（エコーなし）
3. 認証処理
   - ユーザー存在確認
   - パスワード照合（Argon2）
   - アクティブ状態確認
4. 成功: セッション作成 → メインメニューへ
   失敗: エラー表示 → リトライ（最大3回）
```

### 3.2 ログイン試行制限

| 項目 | 値 |
|------|-----|
| 最大試行回数 | 3回 |
| ロックアウト時間 | 5分 |
| ロックアウト解除 | 自動 |

### 3.3 セッション管理

```rust
/// 認証セッション（ログイン状態を管理）
/// ※ TelnetSession（接続セッション）とは別の概念
pub struct AuthSession {
    id: Uuid,
    user_id: i64,
    token: String,
    ip_address: String,
    created_at: DateTime,
    expires_at: DateTime,
    last_activity: DateTime,
}
```

- セッショントークン: UUID v4
- 有効期限: 24時間
- アイドルタイムアウト: 5分

## 4. ゲストアクセス

### 4.1 ゲストでできること

- 掲示板の閲覧（公開設定のもののみ）
- ファイル一覧の閲覧（公開設定のもののみ）
- 会員一覧の閲覧

### 4.2 ゲストでできないこと

- チャット参加
- メール送受信
- ファイルダウンロード
- ファイルアップロード
- プロフィール機能

※ 掲示板への投稿は、掲示板ごとの権限設定で許可されている場合のみ可能

## 5. 新規会員登録

### 5.1 登録フロー

```
1. ユーザーID入力
   - 4〜16文字
   - 英数字とアンダースコアのみ
   - 重複チェック
2. パスワード入力
   - 8文字以上
   - 確認入力
3. ニックネーム入力
   - 1〜20文字
4. メールアドレス入力（任意）
5. 端末タイプ選択
   - 標準端末 (80x24)
   - Commodore 64 (40x25)
   - C64 ANSI (40x25)
6. 利用規約同意
7. 登録完了 → 自動ログイン
```

### 5.2 バリデーションルール

| 項目 | ルール |
|------|--------|
| ユーザーID | 4-16文字、英数字アンダースコア、一意 |
| パスワード | 8文字以上 |
| ニックネーム | 1-20文字 |
| メールアドレス | RFC 5322準拠（任意） |

### 5.3 禁止ユーザーID

```
guest, admin, sysop, subop, root, system, anonymous, etc.
```

## 6. プロフィール管理

### 6.1 閲覧可能項目

| 項目 | 本人 | 他者 |
|------|------|------|
| ユーザーID | ○ | ○ |
| ニックネーム | ○ | ○ |
| 自己紹介 | ○ | ○ |
| 登録日 | ○ | ○ |
| 最終ログイン | ○ | × |
| 投稿数 | ○ | ○ |
| メールアドレス | ○ | × |
| 権限 | ○ | △ |

### 6.2 編集可能項目

- ニックネーム
- メールアドレス
- 自己紹介（プロフィール文）
- パスワード
- 端末タイプ（標準端末 / C64 / C64 ANSI）

### 6.3 パスワード変更

```
1. 現在のパスワード入力
2. 新しいパスワード入力
3. 新しいパスワード確認
4. 変更完了
```

## 7. 退会

### 7.1 退会フロー

```
1. パスワード確認
2. 退会理由入力（任意）
3. 最終確認
4. 退会処理
   - is_active = 0 に設定
   - セッション削除
   - （投稿データは残す）
```

### 7.2 退会後のデータ

- ユーザーデータ: 論理削除（is_active=0）
- 投稿データ: 残る（投稿者名は「退会済みユーザー」表示）
- メール: 送受信とも残る
- アップロードファイル: 残る

## 8. 会員一覧

### 8.1 表示項目

- ユーザーID
- ニックネーム
- 権限
- 登録日
- 最終ログイン（管理者のみ）
- オンライン状態

### 8.2 ソート

- ユーザーID順
- 登録日順
- 最終ログイン順

## 9. API仕様

```rust
pub trait AuthService {
    /// ログイン
    async fn login(&self, username: &str, password: &str) -> Result<AuthSession>;

    /// ログアウト
    async fn logout(&self, session_id: Uuid) -> Result<()>;

    /// 新規登録
    async fn register(&self, data: RegisterData) -> Result<User>;

    /// プロフィール取得
    async fn get_profile(&self, user_id: i64) -> Result<UserProfile>;

    /// プロフィール更新
    async fn update_profile(&self, user_id: i64, data: UpdateProfile) -> Result<()>;

    /// パスワード変更
    async fn change_password(&self, user_id: i64, old: &str, new: &str) -> Result<()>;

    /// 退会
    async fn withdraw(&self, user_id: i64, password: &str) -> Result<()>;

    /// セッション検証
    async fn validate_session(&self, token: &str) -> Result<AuthSession>;

    /// 権限チェック
    fn check_permission(&self, user: &User, required: Role) -> bool;
}

#[derive(Debug)]
pub struct RegisterData {
    pub username: String,
    pub password: String,
    pub nickname: String,
    pub email: Option<String>,
}

#[derive(Debug)]
pub struct UpdateProfile {
    pub nickname: Option<String>,
    pub email: Option<String>,
    pub profile: Option<String>,
}
```
