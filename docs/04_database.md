# HOBBS - データベース設計

## 1. 概要

- **DBMS**: SQLite 3.35以上 または PostgreSQL 12以上（コンパイル時に選択）
- **ファイル（SQLite）**: `data/hobbs.db`
- **接続URL（PostgreSQL）**: `postgres://user:password@localhost/hobbs`
- **文字コード**: UTF-8（内部保存）
- **ライブラリ**: sqlx（非同期対応）

### 1.1 DBMS間の差異

| 項目 | SQLite | PostgreSQL |
|------|--------|------------|
| 自動採番 | `INTEGER PRIMARY KEY` | `BIGSERIAL PRIMARY KEY` |
| 現在日時 | `datetime('now')` | `TO_CHAR(NOW(), 'YYYY-MM-DD HH24:MI:SS')` |
| Boolean | INTEGER (0/1) | BOOLEAN |
| 大文字小文字無視 | `COLLATE NOCASE` | `LOWER()` + インデックス |

### 1.2 マイグレーション

マイグレーションファイルはDBMS別に管理：

```
migrations/
├── sqlite/
│   ├── 0001_initial_users.sql
│   ├── 0002_add_encoding.sql
│   └── ...
└── postgres/
    ├── 0001_initial_users.sql
    ├── 0002_add_encoding.sql
    └── ...
```

sqlx-cliで管理：
```bash
# SQLite
sqlx migrate run --source migrations/sqlite

# PostgreSQL
sqlx migrate run --source migrations/postgres
```

## 2. ER図

```
┌─────────────┐       ┌─────────────┐       ┌─────────────┐
│   users     │       │   boards    │       │   threads   │
├─────────────┤       ├─────────────┤       ├─────────────┤
│ id (PK)     │       │ id (PK)     │       │ id (PK)     │
│ username    │       │ name        │       │ board_id(FK)│──┐
│ password    │       │ description │       │ title       │  │
│ nickname    │       │ type        │       │ author_id   │──┼──┐
│ email       │   ┌──▶│ permission  │       │ created_at  │  │  │
│ role        │   │   │ created_at  │◀──────│ updated_at  │  │  │
│ created_at  │   │   │ order_num   │       │ post_count  │  │  │
│ last_login  │   │   └─────────────┘       └─────────────┘  │  │
│ is_active   │   │                                          │  │
└─────────────┘   │   ┌─────────────┐       ┌─────────────┐  │  │
      │           │   │   posts     │       │   mails     │  │  │
      │           │   ├─────────────┤       ├─────────────┤  │  │
      │           │   │ id (PK)     │       │ id (PK)     │  │  │
      │           │   │ board_id(FK)│───────│ from_id(FK) │──┘  │
      │           │   │ thread_id   │       │ to_id (FK)  │─────┘
      │           │   │ author_id   │───────│ subject     │
      │           │   │ content     │       │ body        │
      │           │   │ created_at  │       │ is_read     │
      │           │   └─────────────┘       │ created_at  │
      │           │                         └─────────────┘
      │           │
      │           │   ┌─────────────┐       ┌─────────────┐
      │           │   │  folders    │       │   files     │
      │           │   ├─────────────┤       ├─────────────┤
      │           └───│ id (PK)     │◀──────│ id (PK)     │
      │               │ name        │       │ folder_id   │
      └───────────────│ description │       │ filename    │
                      │ permission  │       │ size        │
                      │ upload_perm │       │ uploader_id │
                      │ parent_id   │       │ downloads   │
                      │ created_at  │       │ created_at  │
                      └─────────────┘       └─────────────┘

      ┌─────────────┐
      │  sessions   │
      ├─────────────┤
      │ id (PK)     │
      │ user_id(FK) │
      │ token       │
      │ ip_address  │
      │ created_at  │
      │ expires_at  │
      └─────────────┘
```

## 3. テーブル定義

### 3.1 users（ユーザー）

```sql
CREATE TABLE users (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    username    TEXT NOT NULL UNIQUE,
    password    TEXT NOT NULL,           -- Argon2ハッシュ
    nickname    TEXT NOT NULL,
    email       TEXT,
    role        TEXT NOT NULL DEFAULT 'member',  -- 'sysop', 'subop', 'member'
    profile     TEXT,                    -- 自己紹介
    terminal    TEXT NOT NULL DEFAULT 'standard',  -- 'standard', 'c64', 'c64_ansi'
    created_at  TEXT NOT NULL DEFAULT (datetime('now')),
    last_login  TEXT,
    is_active   INTEGER NOT NULL DEFAULT 1
);

CREATE INDEX idx_users_username ON users(username);
CREATE INDEX idx_users_role ON users(role);
```

| カラム | 型 | 説明 |
|--------|-----|------|
| id | INTEGER | 主キー |
| username | TEXT | ログインID（一意） |
| password | TEXT | パスワード（Argon2ハッシュ） |
| nickname | TEXT | 表示名・ハンドルネーム |
| email | TEXT | メールアドレス（任意） |
| role | TEXT | 権限（sysop/subop/member） |
| profile | TEXT | 自己紹介文 |
| terminal | TEXT | 端末タイプ（standard/c64/c64_ansi） |
| created_at | TEXT | 登録日時 |
| last_login | TEXT | 最終ログイン日時 |
| is_active | INTEGER | 有効フラグ（1=有効, 0=退会/停止） |

### 3.2 boards（掲示板）

```sql
CREATE TABLE boards (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    name        TEXT NOT NULL,
    description TEXT,
    board_type  TEXT NOT NULL DEFAULT 'thread',  -- 'thread', 'flat'
    permission  TEXT NOT NULL DEFAULT 'member',  -- 閲覧権限: 'guest', 'member', 'subop', 'sysop'
    post_perm   TEXT NOT NULL DEFAULT 'member',  -- 投稿権限
    order_num   INTEGER NOT NULL DEFAULT 0,
    is_active   INTEGER NOT NULL DEFAULT 1,
    created_at  TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_boards_order ON boards(order_num);
```

| カラム | 型 | 説明 |
|--------|-----|------|
| id | INTEGER | 主キー |
| name | TEXT | 掲示板名 |
| description | TEXT | 説明文 |
| board_type | TEXT | 形式（thread=スレッド, flat=フラット） |
| permission | TEXT | 閲覧に必要な権限 |
| post_perm | TEXT | 投稿に必要な権限 |
| order_num | INTEGER | 表示順 |
| is_active | INTEGER | 有効フラグ |
| created_at | TEXT | 作成日時 |

### 3.3 threads（スレッド）

```sql
CREATE TABLE threads (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    board_id    INTEGER NOT NULL REFERENCES boards(id) ON DELETE CASCADE,
    title       TEXT NOT NULL,
    author_id   INTEGER NOT NULL REFERENCES users(id),
    post_count  INTEGER NOT NULL DEFAULT 1,
    created_at  TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at  TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_threads_board ON threads(board_id, updated_at DESC);
CREATE INDEX idx_threads_author ON threads(author_id);
```

| カラム | 型 | 説明 |
|--------|-----|------|
| id | INTEGER | 主キー |
| board_id | INTEGER | 所属掲示板（外部キー） |
| title | TEXT | スレッドタイトル |
| author_id | INTEGER | 作成者（外部キー） |
| post_count | INTEGER | 投稿数 |
| created_at | TEXT | 作成日時 |
| updated_at | TEXT | 最終投稿日時 |

### 3.4 posts（投稿）

```sql
CREATE TABLE posts (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    board_id    INTEGER NOT NULL REFERENCES boards(id) ON DELETE CASCADE,
    thread_id   INTEGER REFERENCES threads(id) ON DELETE CASCADE,  -- NULL=フラット形式
    author_id   INTEGER NOT NULL REFERENCES users(id),
    title       TEXT,                    -- フラット形式用
    content     TEXT NOT NULL,
    created_at  TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_posts_board ON posts(board_id, created_at DESC);
CREATE INDEX idx_posts_thread ON posts(thread_id, created_at);
CREATE INDEX idx_posts_author ON posts(author_id);
```

| カラム | 型 | 説明 |
|--------|-----|------|
| id | INTEGER | 主キー |
| board_id | INTEGER | 所属掲示板（外部キー） |
| thread_id | INTEGER | 所属スレッド（NULLならフラット） |
| author_id | INTEGER | 投稿者（外部キー） |
| title | TEXT | タイトル（フラット形式用） |
| content | TEXT | 本文 |
| created_at | TEXT | 投稿日時 |

### 3.5 mails（内部メール）

```sql
CREATE TABLE mails (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    from_id     INTEGER NOT NULL REFERENCES users(id),
    to_id       INTEGER NOT NULL REFERENCES users(id),
    subject     TEXT NOT NULL,
    body        TEXT NOT NULL,
    is_read     INTEGER NOT NULL DEFAULT 0,
    is_deleted_by_sender   INTEGER NOT NULL DEFAULT 0,
    is_deleted_by_receiver INTEGER NOT NULL DEFAULT 0,
    created_at  TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_mails_to ON mails(to_id, created_at DESC);
CREATE INDEX idx_mails_from ON mails(from_id, created_at DESC);
```

| カラム | 型 | 説明 |
|--------|-----|------|
| id | INTEGER | 主キー |
| from_id | INTEGER | 送信者（外部キー） |
| to_id | INTEGER | 受信者（外部キー） |
| subject | TEXT | 件名 |
| body | TEXT | 本文 |
| is_read | INTEGER | 既読フラグ |
| is_deleted_by_sender | INTEGER | 送信者による削除 |
| is_deleted_by_receiver | INTEGER | 受信者による削除 |
| created_at | TEXT | 送信日時 |

### 3.6 folders（ファイルフォルダ）

```sql
CREATE TABLE folders (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    name        TEXT NOT NULL,
    description TEXT,
    parent_id   INTEGER REFERENCES folders(id) ON DELETE CASCADE,
    permission  TEXT NOT NULL DEFAULT 'member',  -- 閲覧権限
    upload_perm TEXT NOT NULL DEFAULT 'subop',   -- アップロード権限
    order_num   INTEGER NOT NULL DEFAULT 0,
    created_at  TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_folders_parent ON folders(parent_id, order_num);
```

| カラム | 型 | 説明 |
|--------|-----|------|
| id | INTEGER | 主キー |
| name | TEXT | フォルダ名 |
| description | TEXT | 説明 |
| parent_id | INTEGER | 親フォルダ（NULLならルート） |
| permission | TEXT | 閲覧権限 |
| upload_perm | TEXT | アップロード権限 |
| order_num | INTEGER | 表示順 |
| created_at | TEXT | 作成日時 |

### 3.7 files（ファイル）

```sql
CREATE TABLE files (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    folder_id    INTEGER NOT NULL REFERENCES folders(id) ON DELETE CASCADE,
    filename     TEXT NOT NULL,           -- 表示用ファイル名
    stored_name  TEXT NOT NULL,           -- 保存時のファイル名（UUID）
    size         INTEGER NOT NULL,        -- バイト数
    description  TEXT,
    uploader_id  INTEGER NOT NULL REFERENCES users(id),
    downloads    INTEGER NOT NULL DEFAULT 0,
    created_at   TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_files_folder ON files(folder_id);
CREATE INDEX idx_files_uploader ON files(uploader_id);
```

| カラム | 型 | 説明 |
|--------|-----|------|
| id | INTEGER | 主キー |
| folder_id | INTEGER | 所属フォルダ（外部キー） |
| filename | TEXT | 元のファイル名 |
| stored_name | TEXT | 保存名（UUID.拡張子） |
| size | INTEGER | ファイルサイズ（バイト） |
| description | TEXT | ファイル説明 |
| uploader_id | INTEGER | アップロード者（外部キー） |
| downloads | INTEGER | ダウンロード回数 |
| created_at | TEXT | アップロード日時 |

### 3.8 sessions（セッション）

```sql
CREATE TABLE sessions (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id     INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token       TEXT NOT NULL UNIQUE,
    ip_address  TEXT,
    created_at  TEXT NOT NULL DEFAULT (datetime('now')),
    expires_at  TEXT NOT NULL
);

CREATE INDEX idx_sessions_token ON sessions(token);
CREATE INDEX idx_sessions_user ON sessions(user_id);
CREATE INDEX idx_sessions_expires ON sessions(expires_at);
```

| カラム | 型 | 説明 |
|--------|-----|------|
| id | INTEGER | 主キー |
| user_id | INTEGER | ユーザー（外部キー） |
| token | TEXT | セッショントークン（UUID） |
| ip_address | TEXT | 接続元IPアドレス |
| created_at | TEXT | 作成日時 |
| expires_at | TEXT | 有効期限 |

### 3.9 chat_logs（チャットログ）- オプション

```sql
CREATE TABLE chat_logs (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id     INTEGER NOT NULL REFERENCES users(id),
    message     TEXT NOT NULL,
    kind        TEXT NOT NULL,  -- 'chat', 'action', 'system'
    created_at  TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_chat_logs_time ON chat_logs(created_at DESC);
```

| カラム | 型 | 説明 |
|--------|-----|------|
| id | INTEGER | 主キー |
| user_id | INTEGER | 発言者（外部キー） |
| message | TEXT | メッセージ内容 |
| kind | TEXT | 種別（chat=通常発言, action=/meコマンド, system=入退室等） |
| created_at | TEXT | 発言日時 |

### 3.10 read_positions（既読位置）

掲示板の未読管理用。ユーザーごとに各掲示板で「最後に読んだ投稿ID」を記録する。

```sql
CREATE TABLE read_positions (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id         INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    board_id        INTEGER NOT NULL REFERENCES boards(id) ON DELETE CASCADE,
    last_read_post_id INTEGER NOT NULL DEFAULT 0,  -- 最後に読んだ投稿ID
    updated_at      TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(user_id, board_id)
);

CREATE INDEX idx_read_positions_user ON read_positions(user_id);
CREATE INDEX idx_read_positions_board ON read_positions(board_id);
```

| カラム | 型 | 説明 |
|--------|-----|------|
| id | INTEGER | 主キー |
| user_id | INTEGER | ユーザー（外部キー） |
| board_id | INTEGER | 掲示板（外部キー） |
| last_read_post_id | INTEGER | 最後に読んだ投稿のID |
| updated_at | TEXT | 更新日時 |

**未読判定ロジック：**
```sql
-- 掲示板の未読件数を取得
SELECT COUNT(*) FROM posts
WHERE board_id = ? AND id > (
    SELECT COALESCE(
        (SELECT last_read_post_id FROM read_positions
         WHERE user_id = ? AND board_id = ?),
        0
    )
);

-- 未読記事を古い順に取得
SELECT * FROM posts
WHERE board_id = ? AND id > (
    SELECT COALESCE(
        (SELECT last_read_post_id FROM read_positions
         WHERE user_id = ? AND board_id = ?),
        0
    )
)
ORDER BY id ASC;
```

## 4. 初期データ

```sql
-- SysOpユーザー（パスワードはセットアップ時に設定）
INSERT INTO users (username, password, nickname, role)
VALUES ('sysop', '<hashed_password>', 'SysOp', 'sysop');

-- デフォルト掲示板
INSERT INTO boards (name, description, board_type, order_num)
VALUES
    ('お知らせ', 'システムからのお知らせ', 'flat', 1),
    ('雑談', '自由に語り合いましょう', 'thread', 2),
    ('質問', '質問・相談はこちら', 'thread', 3);

-- デフォルトファイルフォルダ
INSERT INTO folders (name, description, upload_perm, order_num)
VALUES
    ('共有ファイル', '会員向けファイル', 'subop', 1),
    ('フリーソフト', 'フリーソフトウェア', 'subop', 2);
```

## 5. マイグレーション

sqlxのマイグレーション機能を使用。起動時に自動適用される。

```
migrations/
├── sqlite/          # SQLite用マイグレーション
│   ├── 0001_initial_users.sql
│   ├── 0002_add_encoding.sql
│   └── ...（22ファイル）
└── postgres/        # PostgreSQL用マイグレーション
    ├── 0001_initial_users.sql
    ├── 0002_add_encoding.sql
    └── ...（22ファイル）
```

アプリケーション起動時に、選択されたDBMS用のマイグレーションが自動実行される。
バージョン管理は `_sqlx_migrations` テーブルで行われる。
