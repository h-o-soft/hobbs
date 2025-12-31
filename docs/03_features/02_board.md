# HOBBS - 機能仕様書: 掲示板

## 1. 概要

スレッド形式・フラット形式の両方に対応した掲示板機能。掲示板は動的に追加・削除・設定変更が可能。

## 2. 掲示板形式

### 2.1 スレッド形式

- トピック（スレッド）を立てて、その中にレスをつける
- スレッドごとに話題がまとまる
- 最新レスがあったスレッドが上に表示される（age方式）

```
掲示板
└─ スレッド1「最近買ったもの」
│  ├─ レス1
│  ├─ レス2
│  └─ レス3
└─ スレッド2「今日の晩ごはん」
   ├─ レス1
   └─ レス2
```

### 2.2 フラット形式

- 記事が時系列に並ぶ
- 従来のBBS形式
- シンプルで見やすい

```
掲示板
├─ 記事1（最新）
├─ 記事2
├─ 記事3
└─ 記事4（古い）
```

## 3. 掲示板の属性

| 属性 | 型 | 説明 |
|------|-----|------|
| id | INTEGER | 掲示板ID |
| name | TEXT | 掲示板名 |
| description | TEXT | 説明文 |
| board_type | TEXT | 形式（thread/flat） |
| permission | TEXT | 閲覧権限 |
| post_perm | TEXT | 投稿権限 |
| order_num | INTEGER | 表示順 |
| is_active | INTEGER | 有効フラグ |

### 3.1 権限設定

閲覧権限・投稿権限はそれぞれ以下から選択：

- `guest`: ゲストでも可（投稿権限をguestにした場合、未登録ユーザーも投稿可能）
- `member`: 会員のみ
- `subop`: SubOp以上
- `sysop`: SysOpのみ

**ゲスト投稿の用途例**：
- 匿名掲示板
- 問い合わせフォーム的な用途
- 自由参加型の雑談板

## 4. スレッド（スレッド形式用）

| 属性 | 型 | 説明 |
|------|-----|------|
| id | INTEGER | スレッドID |
| board_id | INTEGER | 所属掲示板 |
| title | TEXT | スレッドタイトル |
| author_id | INTEGER | 作成者 |
| post_count | INTEGER | レス数 |
| created_at | TEXT | 作成日時 |
| updated_at | TEXT | 最終投稿日時 |

## 5. 投稿（記事・レス）

| 属性 | 型 | 説明 |
|------|-----|------|
| id | INTEGER | 投稿ID |
| board_id | INTEGER | 所属掲示板 |
| thread_id | INTEGER | 所属スレッド（NULLならフラット） |
| author_id | INTEGER | 投稿者 |
| title | TEXT | タイトル（フラット形式用） |
| content | TEXT | 本文 |
| created_at | TEXT | 投稿日時 |

## 6. 機能詳細

### 6.1 掲示板一覧

```
表示項目：
- 掲示板番号
- 掲示板名
- 説明（短縮表示）
- 記事/スレッド数
- 未読件数（★マーク付き）
- 最新投稿日時
```

### 6.2 スレッド一覧（スレッド形式）

```
表示項目：
- スレッド番号
- タイトル
- 作成者
- レス数
- 最終投稿日時

ソート：
- 最終投稿日時降順（デフォルト）
```

### 6.3 記事一覧（フラット形式）

```
表示項目：
- 記事番号
- タイトル
- 投稿者
- 投稿日時

ソート：
- 投稿日時降順（デフォルト）
```

### 6.4 記事詳細表示

```
表示項目：
- 掲示板名 > スレッドタイトル（スレッド形式時）
- 投稿者名
- 投稿日時
- 本文
- レス番号（スレッド形式時）
```

### 6.5 新規投稿

**スレッド形式 - 新規スレッド**
```
入力項目：
- スレッドタイトル（1-50文字）
- 本文（必須）

処理：
1. スレッド作成
2. 最初のレスとして本文を投稿
```

**スレッド形式 - レス投稿**
```
入力項目：
- 本文のみ

処理：
1. スレッドにレス追加
2. スレッドのupdated_atを更新
3. post_countをインクリメント
```

**フラット形式 - 新規記事**
```
入力項目：
- タイトル（1-50文字）
- 本文（必須）

処理：
1. 記事を追加
```

### 6.6 投稿の削除

- 投稿者本人: 可能
- SubOp以上: 可能
- 削除後は「この投稿は削除されました」表示

### 6.7 ページング

- 1ページあたり表示件数: 20件
- ページ移動: [N]次ページ / [P]前ページ

## 7. 操作フロー

### 7.1 掲示板閲覧フロー

```
メインメニュー
    ↓ [B]
掲示板一覧
    ↓ [番号]
スレッド一覧 / 記事一覧
    ↓ [番号]
記事詳細
    ↓ [Q]
スレッド一覧 / 記事一覧
    ↓ [Q]
掲示板一覧
    ↓ [Q]
メインメニュー
```

### 7.2 投稿フロー

```
スレッド一覧
    ↓ [W]
新規スレッド作成
    ↓
タイトル入力
    ↓
本文入力（.で終了）
    ↓
確認
    ↓ [Y]
投稿完了
    ↓
スレッド一覧に戻る
```

## 8. API仕様

```rust
pub trait BoardService {
    /// 掲示板一覧取得
    async fn list_boards(&self, user: &User) -> Result<Vec<Board>>;

    /// 掲示板取得
    async fn get_board(&self, id: i64) -> Result<Board>;

    /// スレッド一覧取得
    async fn list_threads(&self, board_id: i64, page: u32) -> Result<Page<Thread>>;

    /// スレッド取得
    async fn get_thread(&self, id: i64) -> Result<Thread>;

    /// 投稿一覧取得（フラット形式）
    async fn list_posts_flat(&self, board_id: i64, page: u32) -> Result<Page<Post>>;

    /// 投稿一覧取得（スレッド内）
    async fn list_posts_in_thread(&self, thread_id: i64, page: u32) -> Result<Page<Post>>;

    /// 投稿取得
    async fn get_post(&self, id: i64) -> Result<Post>;

    /// スレッド作成
    async fn create_thread(&self, board_id: i64, author_id: i64, title: &str, content: &str) -> Result<Thread>;

    /// 投稿作成（レス/記事）
    async fn create_post(&self, data: CreatePost) -> Result<Post>;

    /// 投稿削除
    async fn delete_post(&self, post_id: i64, user: &User) -> Result<()>;
}

#[derive(Debug)]
pub struct CreatePost {
    pub board_id: i64,
    pub thread_id: Option<i64>,  // None = フラット形式
    pub author_id: i64,
    pub title: Option<String>,   // フラット形式時
    pub content: String,
}

#[derive(Debug)]
pub struct Page<T> {
    pub items: Vec<T>,
    pub page: u32,
    pub total_pages: u32,
    pub total_items: u64,
}
```

## 9. 制限事項

| 項目 | 制限値 |
|------|--------|
| タイトル最大長 | 50文字 |
| 本文最大長 | 10,000文字 |
| 1ページ表示件数 | 20件 |
| スレッド最大レス数 | 1,000件 |

## 10. 未読管理機能

### 10.1 概要

ユーザーごとに「最後に読んだ投稿ID」を記録するポインタ方式で未読を管理。
新規投稿がそのIDより大きければ未読として扱う。

### 10.2 未読表示

**掲示板一覧での表示：**
```
No. 掲示板名          記事  未読   最新
 1  お知らせ            15    2*  01/15
 2  雑談               234   15*  01/15
 3  質問                89    0   01/15
```
- `*` マークで未読があることを示す
- 未読0件の場合はマークなし

**スレッド一覧での表示：**
```
No. タイトル          投稿者 Res  未読
 1  最近買ったもの    はなこ  45    3*
 2  今日の晩ごはん    たろう  23    0
```

### 10.3 未読一気読み機能

2つのレベルで未読一気読みが可能：

**A) 全掲示板の未読一気読み（掲示板一覧から）**
- コマンド: `[U]` 未読一気読み
- 全掲示板の未読記事を古い順に連続表示
- 掲示板をまたいで時系列で表示

**B) 単一掲示板の未読一気読み（掲示板内から）**
- コマンド: `[U]` 未読一気読み
- その掲示板の未読記事のみを古い順に連続表示

### 10.4 未読一気読みフロー

```
1. [U] 未読一気読み選択
2. 「未読 XX件 を読みます」確認
3. 未読記事を1件ずつ表示
   ┌─────────────────────────────┐
   │ 掲示板: 雑談                │
   │ スレッド: 最近買ったもの    │
   │ 投稿者: はなこ  01/15 20:15 │
   │ ─────────────────────────── │
   │ （本文）                    │
   │ ─────────────────────────── │
   │ [Enter]次 [S]スキップ [Q]終了│
   │ (残り 14件)                 │
   └─────────────────────────────┘
4. Enter → 次の未読へ、既読位置を更新
5. S → この掲示板をスキップして次の掲示板へ
6. Q → 一気読み終了（ここまでを既読に）
7. 全て読了 → 「未読を全て読みました」
```

### 10.5 既読位置の更新タイミング

| 操作 | 更新タイミング |
|------|---------------|
| 記事詳細を開く | その記事まで既読に |
| 未読一気読みでEnter | その記事まで既読に |
| 未読一気読みでQ終了 | 最後に表示した記事まで既読に |
| 掲示板を「全て既読」 | 最新記事まで既読に |

### 10.6 追加コマンド

**掲示板一覧：**
- `[U]` 全掲示板の未読一気読み
- `[A]` 全掲示板を既読にする

**掲示板内（スレッド/記事一覧）：**
- `[U]` この掲示板の未読一気読み
- `[A]` この掲示板を既読にする

### 10.7 ゲストの場合

ゲストは未読管理対象外。常に全記事を閲覧可能だが、未読情報は保持されない。

### 10.8 API拡張

```rust
pub trait BoardService {
    // ... 既存のメソッド ...

    /// 掲示板一覧取得（未読件数付き）
    async fn list_boards_with_unread(&self, user: &User) -> Result<Vec<BoardWithUnread>>;

    /// 未読件数取得（単一掲示板）
    async fn get_unread_count(&self, user_id: i64, board_id: i64) -> Result<u32>;

    /// 未読件数取得（全掲示板合計）
    async fn get_total_unread_count(&self, user_id: i64) -> Result<u32>;

    /// 未読記事取得（単一掲示板、古い順）
    async fn get_unread_posts(&self, user_id: i64, board_id: i64) -> Result<Vec<Post>>;

    /// 未読記事取得（全掲示板、古い順）
    async fn get_all_unread_posts(&self, user_id: i64) -> Result<Vec<PostWithBoard>>;

    /// 既読位置を更新
    async fn mark_as_read(&self, user_id: i64, board_id: i64, post_id: i64) -> Result<()>;

    /// 掲示板を全て既読にする
    async fn mark_board_as_read(&self, user_id: i64, board_id: i64) -> Result<()>;

    /// 全掲示板を既読にする
    async fn mark_all_as_read(&self, user_id: i64) -> Result<()>;
}

#[derive(Debug)]
pub struct BoardWithUnread {
    pub board: Board,
    pub unread_count: u32,
}

#[derive(Debug)]
pub struct PostWithBoard {
    pub post: Post,
    pub board_name: String,
    pub thread_title: Option<String>,
}
```
