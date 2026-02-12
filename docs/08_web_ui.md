# Web UI 設計

## 1. 概要

HOBBSにREST API + SPA形式のWeb UIを追加し、ブラウザからもBBSにアクセス可能にする。

### 1.1 目標

- Telnet UIとWeb UIの両方から同じBBSを利用可能
- 既存のビジネスロジック層を共有
- モダンなSPA体験を提供
- チャット機能のリアルタイム対応

### 1.2 技術スタック

| 項目 | 選定技術 | バージョン | 理由 |
|------|----------|------------|------|
| Web Framework | Axum | 0.7.x | tokioネイティブ、WebSocket標準対応 |
| 認証 | JWT | - | ステートレス、SPA親和性 |
| リアルタイム通信 | WebSocket | - | チャット要件、既存broadcast統合 |
| Frontend Framework | SolidJS | 1.x | 軽量、高パフォーマンス |
| Build Tool | Vite | 5.x | 高速開発サーバー |
| CSS Framework | TailwindCSS | 3.x | 迅速なスタイリング |

---

## 2. アーキテクチャ

### 2.1 全体構成

```
┌─────────────────────────────────────────────────────────────────────┐
│                          HOBBS Server                                │
│                                                                      │
│  ┌───────────────────────────────────────────────────────────────┐  │
│  │                    Web API Layer (Axum)                        │  │
│  │  ┌─────────────┐  ┌─────────────┐  ┌──────────────────────┐  │  │
│  │  │   Router    │  │   JWT Auth  │  │   CORS Middleware    │  │  │
│  │  │   Handlers  │  │  Middleware │  │   (tower-http)       │  │  │
│  │  └─────────────┘  └─────────────┘  └──────────────────────┘  │  │
│  │                                                                │  │
│  │  ┌─────────────────────────────────────────────────────────┐  │  │
│  │  │                    WebSocket Hub                         │  │  │
│  │  │          (Chat, Notifications)                           │  │  │
│  │  └─────────────────────────────────────────────────────────┘  │  │
│  └───────────────────────────────────────────────────────────────┘  │
│                              │                                       │
│  ┌───────────────────────────────────────────────────────────────┐  │
│  │                    Telnet Layer (既存)                         │  │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐   │  │
│  │  │   Listener  │  │  Session    │  │  ShiftJIS            │   │  │
│  │  │   (tokio)   │  │  Handler    │  │  Encoder/Decoder     │   │  │
│  │  └─────────────┘  └─────────────┘  └─────────────────────┘   │  │
│  └───────────────────────────────────────────────────────────────┘  │
│                              │                                       │
│  ┌───────────────────────────────────────────────────────────────┐  │
│  │              Application Layer (共通ビジネスロジック)           │  │
│  │  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐         │  │
│  │  │ AuthSvc  │ │ BoardSvc │ │ MailSvc  │ │ ChatMgr  │         │  │
│  │  └──────────┘ └──────────┘ └──────────┘ └──────────┘         │  │
│  │  ┌──────────┐ ┌──────────┐ ┌──────────┐                      │  │
│  │  │ FileSvc  │ │ AdminSvc │ │ RssSvc   │                      │  │
│  │  └──────────┘ └──────────┘ └──────────┘                      │  │
│  └───────────────────────────────────────────────────────────────┘  │
│                              │                                       │
│  ┌───────────────────────────────────────────────────────────────┐  │
│  │                    Data Layer (共通)                           │  │
│  │  ┌─────────────────────┐  ┌─────────────────────────┐         │  │
│  │  │   SQLite Database   │  │    File Storage         │         │  │
│  │  │   (rusqlite)        │  │    (filesystem)         │         │  │
│  │  └─────────────────────┘  └─────────────────────────┘         │  │
│  └───────────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────┘
          │                                      │
          │ Telnet (TCP:23)                      │ HTTP/WS (:8080)
          ▼                                      ▼
    ┌───────────┐                         ┌──────────────┐
    │  Telnet   │                         │  SPA Client  │
    │  Client   │                         │  (Browser)   │
    └───────────┘                         └──────────────┘
```

### 2.2 ディレクトリ構成

```
hobbs/
├── src/
│   ├── web/                      # Web API層
│   │   ├── mod.rs                # モジュール定義
│   │   ├── server.rs             # Axumサーバー起動
│   │   ├── router.rs             # ルーティング定義
│   │   ├── middleware/           # ミドルウェア
│   │   │   ├── mod.rs
│   │   │   ├── auth.rs           # JWT認証ミドルウェア
│   │   │   └── cors.rs           # CORS設定
│   │   ├── handlers/             # APIハンドラー
│   │   │   ├── mod.rs
│   │   │   ├── auth.rs           # 認証エンドポイント
│   │   │   ├── board.rs          # 掲示板エンドポイント
│   │   │   ├── mail.rs           # メールエンドポイント
│   │   │   ├── chat.rs           # チャットエンドポイント
│   │   │   ├── file.rs           # ファイルエンドポイント
│   │   │   ├── admin.rs          # 管理エンドポイント
│   │   │   ├── user.rs           # ユーザーエンドポイント
│   │   │   └── rss.rs            # RSSエンドポイント
│   │   ├── ws/                   # WebSocket
│   │   │   ├── mod.rs
│   │   │   ├── hub.rs            # 接続管理ハブ
│   │   │   └── chat.rs           # チャットWS処理
│   │   ├── dto/                  # データ転送オブジェクト
│   │   │   ├── mod.rs
│   │   │   ├── request.rs        # リクエストDTO
│   │   │   └── response.rs       # レスポンスDTO
│   │   └── error.rs              # APIエラーハンドリング
│   └── lib.rs                    # web モジュールを追加
├── web/                          # SPAフロントエンド
│   ├── package.json
│   ├── vite.config.ts
│   ├── tsconfig.json
│   ├── index.html
│   └── src/
│       ├── main.tsx              # エントリポイント
│       ├── App.tsx               # ルートコンポーネント
│       ├── api/                  # APIクライアント
│       │   ├── client.ts         # HTTP/WSクライアント
│       │   ├── auth.ts           # 認証API
│       │   ├── board.ts          # 掲示板API
│       │   └── ...
│       ├── components/           # UIコンポーネント
│       │   ├── common/           # 共通コンポーネント
│       │   ├── layout/           # レイアウト
│       │   └── ...
│       ├── pages/                # ページコンポーネント
│       │   ├── Login.tsx
│       │   ├── MainMenu.tsx
│       │   ├── Board.tsx
│       │   └── ...
│       ├── stores/               # 状態管理
│       │   ├── auth.ts           # 認証状態
│       │   └── ...
│       └── styles/               # スタイル
│           └── index.css
└── config.toml                   # [web] セクション追加
```

---

## 3. 設定

### 3.1 config.toml

```toml
[web]
# Web API を有効化
enabled = true

# バインドアドレス
host = "0.0.0.0"
port = 8080

# CORS許可オリジン（開発時）
cors_origins = ["http://localhost:5173"]

# JWT設定
jwt_secret = "your-256-bit-secret-key-here"
jwt_access_token_expiry_secs = 900    # 15分
jwt_refresh_token_expiry_days = 7     # 7日

# 静的ファイル配信
serve_static = true
static_path = "web/dist"
```

### 3.2 WebConfig構造体

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct WebConfig {
    pub enabled: bool,
    pub host: String,
    pub port: u16,
    pub cors_origins: Vec<String>,
    pub jwt_secret: String,
    pub jwt_access_token_expiry_secs: u64,
    pub jwt_refresh_token_expiry_days: u64,
    pub serve_static: bool,
    pub static_path: String,
}

impl Default for WebConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            host: "0.0.0.0".to_string(),
            port: 8080,
            cors_origins: vec![],
            jwt_secret: "".to_string(),
            jwt_access_token_expiry_secs: 900,
            jwt_refresh_token_expiry_days: 7,
            serve_static: false,
            static_path: "web/dist".to_string(),
        }
    }
}
```

---

## 4. REST API 仕様

### 4.1 共通仕様

#### ベースURL
```
http://{host}:{port}/api
```

#### リクエストヘッダー
```
Content-Type: application/json
Authorization: Bearer {access_token}  # 認証が必要なエンドポイント
```

#### レスポンス形式

**成功時:**
```json
{
  "data": { ... },
  "meta": {
    "page": 1,
    "per_page": 20,
    "total": 100
  }
}
```

**エラー時:**
```json
{
  "error": {
    "code": "UNAUTHORIZED",
    "message": "認証が必要です"
  }
}
```

#### エラーコード

| コード | HTTPステータス | 説明 |
|--------|---------------|------|
| `BAD_REQUEST` | 400 | リクエスト不正 |
| `UNAUTHORIZED` | 401 | 認証が必要 |
| `FORBIDDEN` | 403 | 権限不足 |
| `NOT_FOUND` | 404 | リソースが見つからない |
| `CONFLICT` | 409 | 競合（重複など） |
| `UNPROCESSABLE_ENTITY` | 422 | バリデーションエラー |
| `INTERNAL_ERROR` | 500 | サーバーエラー |

### 4.2 認証 API

#### POST /api/auth/login
ログイン

**リクエスト:**
```json
{
  "username": "testuser",
  "password": "password123"
}
```

**レスポンス:**
```json
{
  "data": {
    "access_token": "eyJhbGciOiJIUzI1NiIs...",
    "refresh_token": "eyJhbGciOiJIUzI1NiIs...",
    "expires_in": 900,
    "user": {
      "id": 1,
      "username": "testuser",
      "nickname": "テストユーザー",
      "role": "member"
    }
  }
}
```

#### POST /api/auth/logout
ログアウト（リフレッシュトークン無効化）

**リクエスト:**
```json
{
  "refresh_token": "eyJhbGciOiJIUzI1NiIs..."
}
```

#### POST /api/auth/refresh
トークン更新

**リクエスト:**
```json
{
  "refresh_token": "eyJhbGciOiJIUzI1NiIs..."
}
```

**レスポンス:**
```json
{
  "data": {
    "access_token": "eyJhbGciOiJIUzI1NiIs...",
    "expires_in": 900
  }
}
```

#### POST /api/auth/register
新規会員登録

**リクエスト:**
```json
{
  "username": "newuser",
  "password": "password123",
  "nickname": "新規ユーザー",
  "email": "new@example.com"
}
```

#### GET /api/auth/me
現在のユーザー情報取得

**レスポンス:**
```json
{
  "data": {
    "id": 1,
    "username": "testuser",
    "nickname": "テストユーザー",
    "role": "member",
    "email": "test@example.com",
    "unread_mail_count": 3,
    "created_at": "2024-01-01T00:00:00Z",
    "last_login_at": "2024-12-01T12:00:00Z"
  }
}
```

### 4.3 掲示板 API

#### GET /api/boards
掲示板一覧

**クエリパラメータ:**
- `page` (optional): ページ番号（デフォルト: 1）
- `per_page` (optional): 1ページあたりの件数（デフォルト: 20）

**レスポンス:**
```json
{
  "data": [
    {
      "id": 1,
      "name": "雑談掲示板",
      "description": "自由に雑談してください",
      "board_type": "thread",
      "post_count": 150,
      "unread_count": 5,
      "last_post_at": "2024-12-01T12:00:00Z"
    }
  ],
  "meta": { "page": 1, "per_page": 20, "total": 5 }
}
```

#### GET /api/boards/:id
掲示板詳細

#### GET /api/boards/:id/threads
スレッド一覧（thread形式掲示板）

**クエリパラメータ:**
- `page`, `per_page`

**レスポンス:**
```json
{
  "data": [
    {
      "id": 1,
      "title": "はじめまして",
      "author": {
        "id": 1,
        "nickname": "テストユーザー"
      },
      "post_count": 10,
      "created_at": "2024-12-01T00:00:00Z",
      "last_post_at": "2024-12-01T12:00:00Z"
    }
  ]
}
```

#### POST /api/boards/:id/threads
スレッド作成

**リクエスト:**
```json
{
  "title": "新しいスレッド",
  "content": "本文です"
}
```

#### GET /api/boards/:id/posts
投稿一覧（flat形式掲示板）

#### POST /api/boards/:id/posts
投稿作成（flat形式掲示板）

#### GET /api/threads/:id
スレッド詳細

#### GET /api/threads/:id/posts
スレッド内投稿一覧

**レスポンス:**
```json
{
  "data": [
    {
      "id": 1,
      "content": "投稿内容",
      "author": {
        "id": 1,
        "nickname": "テストユーザー"
      },
      "created_at": "2024-12-01T00:00:00Z"
    }
  ]
}
```

#### POST /api/threads/:id/posts
スレッドに返信

**リクエスト:**
```json
{
  "content": "返信内容"
}
```

#### DELETE /api/posts/:id
投稿削除（作成者またはSubOp以上）

### 4.4 メール API

#### GET /api/mail/inbox
受信トレイ

**クエリパラメータ:**
- `page`, `per_page`

**レスポンス:**
```json
{
  "data": [
    {
      "id": 1,
      "subject": "件名",
      "sender": {
        "id": 2,
        "nickname": "送信者"
      },
      "is_read": false,
      "sent_at": "2024-12-01T12:00:00Z"
    }
  ]
}
```

#### GET /api/mail/sent
送信済みメール

#### GET /api/mail/:id
メール詳細（未読の場合は既読に更新）

**レスポンス:**
```json
{
  "data": {
    "id": 1,
    "subject": "件名",
    "body": "本文",
    "sender": {
      "id": 2,
      "nickname": "送信者"
    },
    "is_read": true,
    "sent_at": "2024-12-01T12:00:00Z"
  }
}
```

#### POST /api/mail
メール送信

**リクエスト:**
```json
{
  "recipient_id": 2,
  "subject": "件名",
  "body": "本文"
}
```

#### DELETE /api/mail/:id
メール削除

#### GET /api/mail/unread-count
未読件数

**レスポンス:**
```json
{
  "data": {
    "count": 3
  }
}
```

### 4.5 チャット API

#### GET /api/chat/rooms
ルーム一覧

**レスポンス:**
```json
{
  "data": [
    {
      "id": "lobby",
      "name": "ロビー",
      "participant_count": 5
    }
  ]
}
```

#### GET /api/chat/rooms/:id
ルーム詳細（参加者一覧）

### 4.6 ファイル API

#### GET /api/folders
フォルダ一覧

#### GET /api/folders/:id
フォルダ詳細

#### GET /api/folders/:id/files
フォルダ内ファイル一覧

**レスポンス:**
```json
{
  "data": [
    {
      "id": 1,
      "filename": "readme.txt",
      "description": "説明文",
      "size": 1024,
      "download_count": 10,
      "uploader": {
        "id": 1,
        "nickname": "テストユーザー"
      },
      "uploaded_at": "2024-12-01T00:00:00Z"
    }
  ]
}
```

#### POST /api/folders/:id/files
ファイルアップロード（multipart/form-data）

**フォームフィールド:**
- `file`: ファイル本体
- `description`: 説明文（オプション）

#### GET /api/files/:id
ファイルメタデータ

#### GET /api/files/:id/download
ファイルダウンロード

#### DELETE /api/files/:id
ファイル削除（アップロード者またはSubOp以上）

### 4.7 ユーザー API

#### GET /api/users
会員一覧（SubOp以上）

#### GET /api/users/:id
会員詳細

#### PUT /api/users/:id/profile
プロフィール更新（本人のみ）

**リクエスト:**
```json
{
  "nickname": "新しいニックネーム",
  "email": "new@example.com"
}
```

#### PUT /api/users/:id/password
パスワード変更（本人のみ）

**リクエスト:**
```json
{
  "current_password": "old_password",
  "new_password": "new_password"
}
```

### 4.8 管理 API

**すべてのエンドポイントはSubOp以上の権限が必要**

#### GET /api/admin/users
ユーザー管理一覧

#### PUT /api/admin/users/:id
ユーザー情報更新

#### PUT /api/admin/users/:id/role
権限変更（SysOpのみ）

**リクエスト:**
```json
{
  "role": "subop"
}
```

#### PUT /api/admin/users/:id/status
有効/無効切替

**リクエスト:**
```json
{
  "is_active": false
}
```

#### GET /api/admin/boards
掲示板管理一覧

#### POST /api/admin/boards
掲示板作成

**リクエスト:**
```json
{
  "name": "新しい掲示板",
  "description": "説明",
  "board_type": "thread",
  "required_role": "member"
}
```

#### PUT /api/admin/boards/:id
掲示板更新

#### DELETE /api/admin/boards/:id
掲示板削除

#### GET /api/admin/folders
フォルダ管理一覧

#### POST /api/admin/folders
フォルダ作成

#### PUT /api/admin/folders/:id
フォルダ更新

#### DELETE /api/admin/folders/:id
フォルダ削除

### 4.9 RSS API

#### GET /api/rss/feeds
フィード一覧

**レスポンス:**
```json
{
  "data": [
    {
      "id": 1,
      "title": "テックニュース",
      "url": "https://example.com/rss",
      "item_count": 50,
      "last_fetched_at": "2024-12-01T12:00:00Z"
    }
  ]
}
```

#### GET /api/rss/feeds/:id/items
フィードアイテム一覧

#### GET /api/rss/items/:id
アイテム詳細

#### POST /api/admin/rss/feeds
フィード追加（管理者）

#### DELETE /api/admin/rss/feeds/:id
フィード削除（管理者）

---

## 5. 認証フロー

### 5.1 JWT構造

#### アクセストークン
```json
{
  "sub": 1,                    // user_id
  "username": "testuser",
  "role": "member",
  "iat": 1701388800,           // 発行日時
  "exp": 1701389700,           // 有効期限（15分後）
  "jti": "uuid-v4"             // トークンID
}
```

#### リフレッシュトークン
```json
{
  "sub": 1,
  "iat": 1701388800,
  "exp": 1701993600,           // 有効期限（7日後）
  "jti": "uuid-v4"
}
```

### 5.2 認証フロー図

```
┌─────────┐                 ┌─────────┐                 ┌─────────┐
│ Client  │                 │ Server  │                 │   DB    │
└────┬────┘                 └────┬────┘                 └────┬────┘
     │                           │                           │
     │  POST /api/auth/login     │                           │
     │  {username, password}     │                           │
     │─────────────────────────> │                           │
     │                           │  verify password          │
     │                           │─────────────────────────> │
     │                           │                           │
     │                           │  store refresh_token      │
     │                           │─────────────────────────> │
     │                           │                           │
     │  {access_token,           │                           │
     │   refresh_token}          │                           │
     │<───────────────────────── │                           │
     │                           │                           │
     │  GET /api/boards          │                           │
     │  Authorization: Bearer    │                           │
     │─────────────────────────> │                           │
     │                           │  verify JWT               │
     │                           │                           │
     │  {boards: [...]}          │                           │
     │<───────────────────────── │                           │
     │                           │                           │
     │  POST /api/auth/refresh   │                           │
     │  {refresh_token}          │                           │
     │─────────────────────────> │                           │
     │                           │  check refresh_token      │
     │                           │─────────────────────────> │
     │                           │                           │
     │  {access_token}           │                           │
     │<───────────────────────── │                           │
```

### 5.3 リフレッシュトークンのDB保存

```sql
CREATE TABLE refresh_tokens (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash TEXT NOT NULL,  -- SHA256ハッシュ
    expires_at DATETIME NOT NULL,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    revoked_at DATETIME,
    UNIQUE(token_hash)
);

CREATE INDEX idx_refresh_tokens_user ON refresh_tokens(user_id);
CREATE INDEX idx_refresh_tokens_hash ON refresh_tokens(token_hash);
```

---

## 6. WebSocket 仕様

### 6.1 接続

WebSocket接続には、セキュリティのためワンタイムトークンを使用します。

#### 接続フロー

1. **ワンタイムトークンを取得**

```http
POST /api/auth/one-time-token
Authorization: Bearer {access_token}
Content-Type: application/json

{
  "purpose": "websocket"
}
```

**レスポンス:**
```json
{
  "data": {
    "token": "abc123...",
    "purpose": "websocket",
    "expires_in": 30
  }
}
```

2. **WebSocketに接続（30秒以内）**

```
ws://{host}:{port}/api/chat/ws?token={one_time_token}
```

**注意事項:**
- ワンタイムトークンは1回限り有効
- 有効期限は30秒
- 期限切れまたは使用済みのトークンでは接続できない

### 6.2 メッセージ形式

#### クライアント → サーバー

**ルーム参加:**
```json
{
  "type": "join",
  "room_id": "lobby"
}
```

**ルーム退出:**
```json
{
  "type": "leave",
  "room_id": "lobby"
}
```

**メッセージ送信:**
```json
{
  "type": "message",
  "room_id": "lobby",
  "content": "こんにちは"
}
```

**アクション送信:**
```json
{
  "type": "action",
  "room_id": "lobby",
  "content": "考え中..."
}
```

**Ping:**
```json
{
  "type": "ping"
}
```

#### サーバー → クライアント

**チャットメッセージ:**
```json
{
  "type": "chat",
  "room_id": "lobby",
  "sender": "テストユーザー",
  "content": "こんにちは",
  "timestamp": "2024-12-01T12:00:00Z"
}
```

**アクション:**
```json
{
  "type": "action",
  "room_id": "lobby",
  "sender": "テストユーザー",
  "content": "考え中...",
  "timestamp": "2024-12-01T12:00:00Z"
}
```

**ユーザー参加:**
```json
{
  "type": "user_joined",
  "room_id": "lobby",
  "user": "新規ユーザー"
}
```

**ユーザー退出:**
```json
{
  "type": "user_left",
  "room_id": "lobby",
  "user": "退出ユーザー"
}
```

**システムメッセージ:**
```json
{
  "type": "system",
  "room_id": "lobby",
  "content": "サーバーメンテナンスのお知らせ"
}
```

**エラー:**
```json
{
  "type": "error",
  "message": "ルームが見つかりません"
}
```

**Pong:**
```json
{
  "type": "pong"
}
```

### 6.3 Telnetとの相互運用

Web UIとTelnet UIのチャットは相互に通信可能。

```
┌──────────────────────────────────────────────────────────────┐
│                    ChatRoomManager                            │
│                                                               │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │              ChatRoom (e.g., "lobby")                    │ │
│  │                                                          │ │
│  │  participants: Vec<ChatParticipant>                      │ │
│  │    - TelnetParticipant { session_id, tx }               │ │
│  │    - WebParticipant { connection_id, ws_tx }            │ │
│  │                                                          │ │
│  │  broadcast_message()                                     │ │
│  │    → 全参加者にメッセージ配信（Telnet/Web両対応）          │ │
│  └─────────────────────────────────────────────────────────┘ │
└──────────────────────────────────────────────────────────────┘
```

---

## 7. フロントエンド構成

### 7.1 ページ構成

| パス | ページ | 認証 |
|------|--------|------|
| `/` | ログイン | 不要 |
| `/register` | 新規登録 | 不要 |
| `/menu` | メインメニュー | 必要 |
| `/boards` | 掲示板一覧 | 必要 |
| `/boards/:id` | 掲示板詳細 | 必要 |
| `/boards/:id/new` | スレッド作成 | 必要 |
| `/threads/:id` | スレッド詳細 | 必要 |
| `/mail` | メール受信トレイ | 必要 |
| `/mail/sent` | 送信済みメール | 必要 |
| `/mail/compose` | メール作成 | 必要 |
| `/mail/:id` | メール詳細 | 必要 |
| `/chat` | チャットルーム一覧 | 必要 |
| `/chat/:id` | チャットルーム | 必要 |
| `/files` | ファイル一覧 | 必要 |
| `/files/:id` | フォルダ詳細 | 必要 |
| `/profile` | プロフィール | 必要 |
| `/admin` | 管理メニュー | SubOp以上 |
| `/admin/users` | ユーザー管理 | SubOp以上 |
| `/admin/boards` | 掲示板管理 | SubOp以上 |
| `/admin/folders` | フォルダ管理 | SubOp以上 |

### 7.2 デザインテーマ

レトロBBSの雰囲気を再現：

- モノスペースフォント使用
- ダークテーマ（黒背景、緑/アンバー文字）
- 枠線はASCII風（─│┌┐└┘等）
- アニメーションは控えめ

---

## 8. セキュリティ

### 8.1 認証・認可

- JWTによるステートレス認証
- リフレッシュトークンはDB管理（revoke可能）
- 権限チェックはミドルウェアで一元管理

### 8.2 入力検証

- すべてのリクエストボディを検証
- SQLインジェクション対策（パラメータ化クエリ）
- XSS対策（Content-Type: application/json）

### 8.3 レート制限

- ログイン: 5回/分
- API全般: 100回/分

### 8.4 CORS

- 許可オリジンを設定ファイルで管理
- 本番環境では明示的に指定

---

## 9. 依存クレート

```toml
# Cargo.toml に追加
axum = "0.7"
axum-extra = { version = "0.9", features = ["typed-header", "cookie"] }
tower = "0.4"
tower-http = { version = "0.5", features = ["cors", "trace", "fs"] }
jsonwebtoken = "9"
utoipa = { version = "4", features = ["axum_extras"] }
utoipa-swagger-ui = { version = "7", features = ["axum"] }
```
