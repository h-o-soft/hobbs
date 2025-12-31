# HOBBS - 機能仕様書: ファイル管理

## 1. 概要

ファイルのアップロード・ダウンロード機能。フォルダ単位で権限設定が可能。

## 2. フォルダ構造

```
ファイルライブラリ（ルート）
├─ 共有ファイル
│  ├─ ドキュメント
│  └─ 画像
├─ フリーソフト
│  ├─ ゲーム
│  └─ ユーティリティ
└─ 会員専用
```

- 階層構造をサポート
- 各フォルダに権限設定可能

## 3. フォルダの属性

| 属性 | 型 | 説明 |
|------|-----|------|
| id | INTEGER | フォルダID |
| name | TEXT | フォルダ名 |
| description | TEXT | 説明 |
| parent_id | INTEGER | 親フォルダ（NULLでルート） |
| permission | TEXT | 閲覧権限 |
| upload_perm | TEXT | アップロード権限 |
| order_num | INTEGER | 表示順 |

### 3.1 権限設定

| 設定 | 閲覧 | ダウンロード | アップロード |
|------|------|--------------|--------------|
| guest | ○ | × | × |
| member | ○ | ○ | △ |
| subop | ○ | ○ | ○ |
| sysop | ○ | ○ | ○ |

△ = フォルダの upload_perm 設定による

## 4. ファイルの属性

| 属性 | 型 | 説明 |
|------|-----|------|
| id | INTEGER | ファイルID |
| folder_id | INTEGER | 所属フォルダ |
| filename | TEXT | 元のファイル名 |
| stored_name | TEXT | 保存名（UUID.ext） |
| size | INTEGER | サイズ（バイト） |
| description | TEXT | ファイル説明 |
| uploader_id | INTEGER | アップロード者 |
| downloads | INTEGER | ダウンロード回数 |
| created_at | TEXT | アップロード日時 |

## 5. 機能詳細

### 5.1 フォルダ一覧

```
表示項目：
- フォルダ記号（[A], [B], ...）
- フォルダ名
- 説明（短縮）
- ファイル数

操作：
- 記号入力でフォルダ移動
- [U] 上の階層へ
```

### 5.2 ファイル一覧

```
表示項目：
- ファイル番号
- ファイル名
- サイズ
- ダウンロード回数
- アップロード日時

ソート：
- アップロード日時降順（デフォルト）
- ファイル名順
- サイズ順
- DL数順
```

### 5.3 ファイルダウンロード

```
手順：
1. ファイル番号を選択
2. ファイル情報表示
3. ダウンロード確認
4. テキストダンプで出力
5. ダウンロードカウント+1
```

**出力形式（テキストファイル）**
```
================================================================================
ファイル名: example.txt
サイズ: 1,234 bytes
アップロード: 2025/01/15 10:00:00 by たろう
説明: サンプルのテキストファイルです
================================================================================
--- ファイル内容 開始 ---

(ファイル内容がそのまま出力される)

--- ファイル内容 終了 ---
ダウンロード完了
```

### 5.4 ファイルアップロード

```
手順：
1. [U] アップロード選択
2. ファイル名入力
3. 説明入力（任意）
4. 本文入力（.で終了）
5. 確認
6. 保存

制限：
- テキストファイルのみ
- 最大サイズ: 設定による（デフォルト10MB）
```

### 5.5 ファイル削除

- アップロード者本人: 可能
- SubOp以上: 可能

## 6. ファイル保存

### 6.1 保存場所

```
data/files/
├─ aa/
│  └─ aabbccdd-1234-5678-90ab-cdef12345678.txt
├─ bb/
│  └─ bbccddee-2345-6789-01ab-cdef23456789.zip
...
```

- UUIDの先頭2文字でディレクトリ分割
- ファイル名は UUID.拡張子

### 6.2 保存処理

```rust
fn store_file(content: &[u8], original_name: &str) -> Result<String> {
    let uuid = Uuid::new_v4();
    let ext = Path::new(original_name)
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("bin");

    let stored_name = format!("{}.{}", uuid, ext);
    let dir = format!("data/files/{}", &uuid.to_string()[..2]);

    fs::create_dir_all(&dir)?;
    fs::write(format!("{}/{}", dir, stored_name), content)?;

    Ok(stored_name)
}
```

## 7. 操作フロー

### 7.1 ダウンロードフロー

```
メインメニュー
    ↓ [F]
ファイルライブラリ（ルート）
    ↓ [A]
フォルダ「共有ファイル」
    ↓ [1]
ファイル情報表示
    ↓ [D]
ダウンロード実行
    ↓
フォルダに戻る
```

### 7.2 アップロードフロー

```
フォルダ内
    ↓ [U]
ファイル名入力
    ↓
説明入力
    ↓
本文入力（.で終了）
    ↓
確認 [Y]
    ↓
アップロード完了
```

## 8. API仕様

```rust
pub trait FileService {
    /// フォルダ一覧取得
    async fn list_folders(&self, parent_id: Option<i64>, user: &User) -> Result<Vec<Folder>>;

    /// フォルダ取得
    async fn get_folder(&self, id: i64) -> Result<Folder>;

    /// ファイル一覧取得
    async fn list_files(&self, folder_id: i64, user: &User) -> Result<Vec<FileInfo>>;

    /// ファイル情報取得
    async fn get_file(&self, id: i64) -> Result<FileInfo>;

    /// ファイルダウンロード
    async fn download(&self, id: i64, user: &User) -> Result<Vec<u8>>;

    /// ファイルアップロード
    async fn upload(&self, data: UploadData, user: &User) -> Result<FileInfo>;

    /// ファイル削除
    async fn delete_file(&self, id: i64, user: &User) -> Result<()>;
}

#[derive(Debug)]
pub struct UploadData {
    pub folder_id: i64,
    pub filename: String,
    pub description: Option<String>,
    pub content: Vec<u8>,
}

#[derive(Debug)]
pub struct FileInfo {
    pub id: i64,
    pub folder_id: i64,
    pub filename: String,
    pub size: u64,
    pub description: Option<String>,
    pub uploader: User,
    pub downloads: u32,
    pub created_at: DateTime,
}
```

## 9. 制限事項

| 項目 | 制限値 |
|------|--------|
| ファイル名最大長 | 100文字 |
| 説明最大長 | 500文字 |
| 最大ファイルサイズ | 10MB（設定可能） |
| フォルダ階層最大 | 10階層 |

## 10. 将来拡張（検討）

- バイナリファイル対応（XMODEM/YMODEM/ZMODEM）
- 画像プレビュー（AAアート変換）
- ファイル圧縮対応
