# HOBBS - セキュリティ仕様

## 1. 概要

HOBBSのセキュリティ設計方針と実装仕様。Telnetプロトコルの制約を考慮しつつ、アプリケーションレベルでのセキュリティを確保する。

## 2. 認証

### 2.1 パスワード保存

| 項目 | 仕様 |
|------|------|
| ハッシュアルゴリズム | Argon2id |
| ソルト | ランダム生成（自動付加） |
| メモリコスト | 64MB |
| 時間コスト | 3 |
| 並列度 | 4 |

```rust
use argon2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use rand_core::OsRng;

pub fn hash_password(password: &str) -> Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let hash = argon2.hash_password(password.as_bytes(), &salt)?;
    Ok(hash.to_string())
}

pub fn verify_password(password: &str, hash: &str) -> Result<bool> {
    let parsed_hash = PasswordHash::new(hash)?;
    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok())
}
```

### 2.2 パスワードポリシー

| 項目 | 要件 |
|------|------|
| 最小文字数 | 8文字 |
| 最大文字数 | 128文字 |
| 必須文字種 | なし（推奨はあり） |
| 禁止パターン | ユーザーIDと同一 |

### 2.3 ログイン試行制限

```rust
pub struct LoginLimiter {
    attempts: HashMap<String, Vec<Instant>>,
    max_attempts: u32,      // 3回
    window: Duration,       // 5分
    lockout: Duration,      // 5分
}

impl LoginLimiter {
    pub fn check(&mut self, username: &str) -> LimitResult {
        let now = Instant::now();
        let attempts = self.attempts.entry(username.to_string()).or_default();

        // 古い試行を削除
        attempts.retain(|t| now.duration_since(*t) < self.window);

        if attempts.len() >= self.max_attempts as usize {
            let oldest = attempts.first().unwrap();
            let remaining = self.lockout - now.duration_since(*oldest);
            return LimitResult::Locked(remaining);
        }

        LimitResult::Allowed
    }

    pub fn record_failure(&mut self, username: &str) {
        self.attempts
            .entry(username.to_string())
            .or_default()
            .push(Instant::now());
    }

    pub fn clear(&mut self, username: &str) {
        self.attempts.remove(username);
    }
}
```

## 3. セッション管理

### 3.1 セッショントークン

| 項目 | 仕様 |
|------|------|
| 形式 | UUID v4 |
| 長さ | 36文字（ハイフン含む） |
| 生成 | 暗号学的乱数生成器 |

```rust
use uuid::Uuid;

pub fn generate_session_token() -> String {
    Uuid::new_v4().to_string()
}
```

### 3.2 セッション有効期限

| 項目 | 値 |
|------|-----|
| 最大有効期限 | 24時間 |
| アイドルタイムアウト | 5分（300秒） |
| 最終アクティビティ更新 | 操作ごと |

### 3.3 セッション無効化

以下のタイミングでセッションを無効化：

- ログアウト時
- アイドルタイムアウト到達時
- 有効期限到達時
- パスワード変更時（全セッション）
- アカウント停止時（全セッション）
- 管理者による強制切断時

## 4. 権限モデル（RBAC）

### 4.1 ロール定義

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Role {
    Guest = 0,
    Member = 1,
    SubOp = 2,
    SysOp = 3,
}

impl Role {
    pub fn can_access(&self, required: Role) -> bool {
        *self >= required
    }
}
```

### 4.2 権限チェック

```rust
pub fn check_permission(user: &Option<User>, required: Role) -> Result<()> {
    let user_role = user.as_ref().map(|u| u.role).unwrap_or(Role::Guest);

    if !user_role.can_access(required) {
        return Err(HobbsError::Permission(format!(
            "この操作には {} 以上の権限が必要です",
            required.display_name()
        )));
    }

    Ok(())
}
```

### 4.3 リソース別権限

| リソース | 操作 | 必要権限 |
|----------|------|----------|
| 掲示板 | 閲覧 | 掲示板設定による |
| 掲示板 | 投稿 | 掲示板設定による |
| 投稿 | 削除（自分） | Member |
| 投稿 | 削除（他人） | SubOp |
| フォルダ | 閲覧 | フォルダ設定による |
| ファイル | ダウンロード | Member |
| ファイル | アップロード | フォルダ設定による |
| チャット | 参加 | Member |
| メール | 送受信 | Member |
| ユーザー | 編集（自分） | Member |
| ユーザー | 編集（他人） | SubOp |
| ユーザー | 権限変更 | SysOp |
| 掲示板 | 管理 | SubOp |
| システム | 設定変更 | SysOp |

## 5. 入力検証

### 5.1 バリデーション関数

```rust
pub mod validation {
    use regex::Regex;
    use once_cell::sync::Lazy;

    static USERNAME_RE: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"^[a-zA-Z0-9_]{4,16}$").unwrap()
    });

    static EMAIL_RE: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"^[^\s@]+@[^\s@]+\.[^\s@]+$").unwrap()
    });

    pub fn validate_username(s: &str) -> Result<()> {
        if !USERNAME_RE.is_match(s) {
            return Err("ユーザーIDは4-16文字の英数字とアンダースコアのみ");
        }
        if is_reserved(s) {
            return Err("このユーザーIDは使用できません");
        }
        Ok(())
    }

    pub fn validate_password(s: &str) -> Result<()> {
        if s.len() < 8 {
            return Err("パスワードは8文字以上必要です");
        }
        if s.len() > 128 {
            return Err("パスワードは128文字以内にしてください");
        }
        Ok(())
    }

    pub fn validate_email(s: &str) -> Result<()> {
        if !s.is_empty() && !EMAIL_RE.is_match(s) {
            return Err("メールアドレスの形式が正しくありません");
        }
        Ok(())
    }

    fn is_reserved(s: &str) -> bool {
        const RESERVED: &[&str] = &[
            "guest", "admin", "sysop", "subop", "root",
            "system", "anonymous", "administrator", "moderator",
        ];
        RESERVED.iter().any(|r| r.eq_ignore_ascii_case(s))
    }
}
```

### 5.2 サニタイズ

```rust
pub mod sanitize {
    /// 制御文字を除去
    pub fn remove_control_chars(s: &str) -> String {
        s.chars()
            .filter(|c| !c.is_control() || *c == '\n' || *c == '\r')
            .collect()
    }

    /// 文字数制限
    pub fn truncate(s: &str, max_chars: usize) -> String {
        s.chars().take(max_chars).collect()
    }

    /// 表示用にエスケープ（ANSIシーケンス除去）
    pub fn escape_for_display(s: &str) -> String {
        // ESCで始まるシーケンスを除去
        let re = Regex::new(r"\x1b\[[0-9;]*[A-Za-z]").unwrap();
        re.replace_all(s, "").to_string()
    }
}
```

### 5.3 入力長制限

| 項目 | 最大長 |
|------|--------|
| ユーザーID | 16文字 |
| パスワード | 128文字 |
| ニックネーム | 20文字 |
| メールアドレス | 254文字 |
| 投稿タイトル | 50文字 |
| 投稿本文 | 10,000文字 |
| メール件名 | 50文字 |
| メール本文 | 10,000文字 |
| ファイル名 | 100文字 |
| チャット発言 | 500文字 |

## 6. 通信セキュリティ

### 6.1 Telnetの制約

Telnetは平文通信のため、以下の点に注意：

- パスワードは平文で送信される
- 通信内容は傍受可能
- 中間者攻撃のリスクあり

### 6.2 推奨される対策

1. **SSHトンネル経由（推奨）**: HOBBS内蔵のSSHサーバーでポートフォワード
2. **ローカルネットワーク限定**: インターネット公開は非推奨
3. **VPN経由**: VPNを使用して暗号化

#### SSH トンネル構成（推奨）

HOBBS内蔵のSSHサーバー（`direct-tcpip` ポートフォワード専用）を使用し、
Telnet通信をSSHで暗号化する。BBS側のコードには変更不要（純粋なトランスポート層）。

```
推奨構成:
[クライアント] --Telnet--> [中継サーバー] --SSH--> [HOBBS SSH:2222]
                            ssh -L 12323:                ↓ direct-tcpip
                            localhost:2323          [HOBBS Telnet:2323]
              --Telnet-->   localhost:12323          (127.0.0.1のみ)
```

SSH有効時の推奨設定:
- `server.host = "127.0.0.1"` でTelnetをローカル限定にする
- SSHパスワードは `config.toml` または環境変数 `HOBBS_SSH_PASSWORD` で設定
- Shell接続は非サポート（SSHターミナルはTelnet IACを処理できないため）

詳細は[運用ガイド](operation_guide.md)のSSHセクションを参照。

#### VPN構成

```
[クライアント] --VPN--> [サーバー] --localhost--> [HOBBS]
```

### 6.3 将来検討

- TLS/SSL対応（Telnet over TLS）

## 7. データ保護

### 7.1 機密データの取り扱い

| データ | 保護方法 |
|--------|----------|
| パスワード | Argon2idハッシュ（復号不可） |
| セッショントークン | 暗号学的乱数生成 |
| メール | 平文保存（将来：暗号化検討） |
| 個人情報 | アクセス制限 |

### 7.2 データベースセキュリティ

#### SQLite版

```rust
// データベースファイルのパーミッション
#[cfg(unix)]
fn secure_db_permissions(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let perms = std::fs::Permissions::from_mode(0o600);
    std::fs::set_permissions(path, perms)?;
    Ok(())
}
```

#### PostgreSQL版

- 接続にはパスワード認証またはSSL証明書を使用
- `pg_hba.conf` で接続元IPアドレスを制限
- SSL接続の使用を推奨（`sslmode=require`）
- データベースユーザーには最小限の権限のみ付与
- 接続文字列のパスワードは環境変数で管理を推奨

```bash
# 環境変数で接続文字列を設定
export DATABASE_URL="postgres://hobbs:password@localhost/hobbs?sslmode=require"
```

### 7.3 バックアップ

- データベースの定期バックアップを推奨
- バックアップファイルは暗号化して保管

## 8. ログ記録

### 8.1 セキュリティログ

```rust
pub enum SecurityEvent {
    LoginSuccess { username: String, ip: String },
    LoginFailure { username: String, ip: String, reason: String },
    Logout { username: String, ip: String },
    PasswordChange { username: String },
    AccountCreated { username: String, ip: String },
    AccountSuspended { username: String, by: String },
    PermissionDenied { username: String, action: String },
    ForceDisconnect { username: String, by: String },
}
```

### 8.2 ログ出力

```
2025-01-15 20:30:15 [SECURITY] LoginSuccess: username=たろう ip=192.168.1.10
2025-01-15 20:30:20 [SECURITY] LoginFailure: username=不正者 ip=192.168.1.99 reason=wrong_password
```

### 8.3 ログローテーション

- 日次でファイルをローテーション
- 保持期間: 30日（設定可能）

## 9. エラーハンドリング

### 9.1 情報漏洩防止

```rust
// 内部エラーはユーザーに詳細を見せない
pub fn user_facing_error(err: &HobbsError) -> String {
    match err {
        HobbsError::Auth(_) => "認証に失敗しました".to_string(),
        HobbsError::Permission(_) => "権限がありません".to_string(),
        HobbsError::Validation(msg) => msg.clone(),
        HobbsError::NotFound(_) => "見つかりませんでした".to_string(),
        // 内部エラーは詳細を隠す
        HobbsError::Database(_) | HobbsError::Io(_) => {
            "システムエラーが発生しました".to_string()
        }
    }
}
```

## 10. セキュリティチェックリスト

### 開発時
- [ ] すべての入力をバリデーション
- [ ] パスワードはArgon2idでハッシュ化
- [ ] セッショントークンは暗号学的乱数で生成
- [ ] 権限チェックをすべての保護リソースに実装
- [ ] エラーメッセージで内部情報を漏らさない
- [ ] ログに機密情報を出力しない

### 運用時
- [ ] 定期的なバックアップ
- [ ] ログの監視
- [ ] 不審なログイン試行の確認
- [ ] パッチ・アップデートの適用
- [ ] 不要なアカウントの無効化
