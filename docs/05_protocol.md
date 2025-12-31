# HOBBS - プロトコル仕様

## 1. 通信仕様

### 1.1 基本仕様

| 項目 | 仕様 |
|------|------|
| プロトコル | Telnet (RFC 854) |
| トランスポート | TCP |
| ポート | 設定可能（デフォルト: 2323） |
| 文字コード | ShiftJIS（送受信） |
| 改行コード | CR+LF (0x0D 0x0A) |

### 1.2 Telnetオプション

最低限のTelnetオプションネゴシエーションをサポート：

| オプション | コード | 対応 |
|------------|--------|------|
| ECHO | 1 | サーバ側で制御 |
| SUPPRESS-GO-AHEAD | 3 | 有効化推奨 |
| TERMINAL-TYPE | 24 | 対応（任意） |
| NAWS (ウィンドウサイズ) | 31 | 対応（任意） |

```
接続時のネゴシエーション例：
Server -> Client: IAC WILL ECHO        (255 251 1)
Server -> Client: IAC WILL SGA         (255 251 3)
Client -> Server: IAC DO ECHO          (255 253 1)
Client -> Server: IAC DO SGA           (255 253 3)
```

### 1.3 タイムアウト

| 種類 | 時間 | 説明 |
|------|------|------|
| 接続タイムアウト | 30秒 | 接続後、入力がなければ切断 |
| アイドルタイムアウト | 5分 | 操作なしで自動ログアウト |
| セッション最大時間 | 24時間 | 強制ログアウト |

## 2. 文字コード処理

### 2.1 ShiftJIS変換

```
[クライアント] <-- ShiftJIS --> [HOBBS Server] <-- UTF-8 --> [Database]
```

- 受信: ShiftJIS → UTF-8 に変換して内部処理
- 送信: UTF-8 → ShiftJIS に変換して送信
- 変換ライブラリ: `encoding_rs`

### 2.2 制御文字

| 文字 | コード | 処理 |
|------|--------|------|
| CR | 0x0D | 改行（LFと組み合わせ） |
| LF | 0x0A | 改行 |
| BS | 0x08 | バックスペース（1文字削除） |
| DEL | 0x7F | バックスペース扱い |
| Ctrl+C | 0x03 | 現在の操作をキャンセル |
| Ctrl+D | 0x04 | EOF（ログアウト確認） |
| ESC | 0x1B | ANSIシーケンス開始 |

### 2.3 入力処理

```rust
// 行バッファリングモード
loop {
    let byte = read_byte().await?;
    match byte {
        0x0D | 0x0A => {
            // 改行 → 行を処理
            process_line(&buffer);
            buffer.clear();
        }
        0x08 | 0x7F => {
            // バックスペース
            buffer.pop();
            send("\x08 \x08")?; // カーソル戻す＋消す＋戻す
        }
        0x03 => {
            // Ctrl+C → キャンセル
            return Action::Cancel;
        }
        b => {
            buffer.push(b);
            echo(b)?; // エコーバック
        }
    }
}
```

## 3. ANSIエスケープシーケンス

### 3.1 基本構文

```
ESC [ <パラメータ> <コマンド>
0x1B 0x5B ...
```

### 3.2 カーソル制御

| シーケンス | 説明 |
|------------|------|
| `\x1b[H` | カーソルをホーム位置(1,1)へ |
| `\x1b[<row>;<col>H` | カーソルを指定位置へ |
| `\x1b[<n>A` | カーソルをn行上へ |
| `\x1b[<n>B` | カーソルをn行下へ |
| `\x1b[<n>C` | カーソルをn列右へ |
| `\x1b[<n>D` | カーソルをn列左へ |
| `\x1b[s` | カーソル位置を保存 |
| `\x1b[u` | カーソル位置を復元 |

### 3.3 画面制御

| シーケンス | 説明 |
|------------|------|
| `\x1b[2J` | 画面クリア |
| `\x1b[K` | 行末までクリア |
| `\x1b[1K` | 行頭までクリア |
| `\x1b[2K` | 行全体をクリア |

### 3.4 色・装飾

| シーケンス | 説明 |
|------------|------|
| `\x1b[0m` | リセット（全属性解除） |
| `\x1b[1m` | 太字/高輝度 |
| `\x1b[4m` | 下線 |
| `\x1b[5m` | 点滅 |
| `\x1b[7m` | 反転 |

**前景色（文字色）**

| コード | 色 | 高輝度 |
|--------|-----|--------|
| 30 | 黒 | 90 |
| 31 | 赤 | 91 |
| 32 | 緑 | 92 |
| 33 | 黄 | 93 |
| 34 | 青 | 94 |
| 35 | マゼンタ | 95 |
| 36 | シアン | 96 |
| 37 | 白 | 97 |

**背景色**

| コード | 色 | 高輝度 |
|--------|-----|--------|
| 40 | 黒 | 100 |
| 41 | 赤 | 101 |
| 42 | 緑 | 102 |
| 43 | 黄 | 103 |
| 44 | 青 | 104 |
| 45 | マゼンタ | 105 |
| 46 | シアン | 106 |
| 47 | 白 | 107 |

**使用例**

```rust
// 赤い太字テキスト
"\x1b[1;31mエラー\x1b[0m: 入力が不正です"

// 青い背景に白い文字
"\x1b[44;37m ヘッダー \x1b[0m"

// シアンのタイトル
"\x1b[36m=== メインメニュー ===\x1b[0m"
```

### 3.5 Rustでの実装

```rust
pub struct Ansi;

impl Ansi {
    pub const RESET: &'static str = "\x1b[0m";
    pub const BOLD: &'static str = "\x1b[1m";
    pub const CLEAR: &'static str = "\x1b[2J\x1b[H";

    pub fn fg(color: u8) -> String {
        format!("\x1b[{}m", color)
    }

    pub fn bg(color: u8) -> String {
        format!("\x1b[{}m", color)
    }

    pub fn goto(row: u16, col: u16) -> String {
        format!("\x1b[{};{}H", row, col)
    }

    pub fn color_text(text: &str, fg: u8) -> String {
        format!("\x1b[{}m{}\x1b[0m", fg, text)
    }
}

// 色定数
pub mod colors {
    pub const BLACK: u8 = 30;
    pub const RED: u8 = 31;
    pub const GREEN: u8 = 32;
    pub const YELLOW: u8 = 33;
    pub const BLUE: u8 = 34;
    pub const MAGENTA: u8 = 35;
    pub const CYAN: u8 = 36;
    pub const WHITE: u8 = 37;
}
```

## 4. 端末プロファイル

HOBBSは複数の端末タイプをサポートする。特にCommodore 64のような特殊な端末にも対応。

### 4.1 対応端末タイプ

| プロファイル | 幅 | 高 | 全角幅 | ANSI | 想定クライアント |
|--------------|----|----|--------|------|------------------|
| `standard` | 80 | 24 | 2 | ○ | TeraTerm, PuTTY等 |
| `c64` | 40 | 25 | 1 | × | C64（現状） |
| `c64_ansi` | 40 | 25 | 1 | ○ | C64（ANSI対応後） |

### 4.2 端末プロファイル構造

```rust
#[derive(Debug, Clone)]
pub struct TerminalProfile {
    pub name: String,         // プロファイル名
    pub width: u16,           // 画面幅（カラム数）
    pub height: u16,          // 画面高（行数）
    pub cjk_width: u8,        // 全角文字の幅（1 or 2）
    pub ansi_enabled: bool,   // ANSIエスケープシーケンス対応
}

impl TerminalProfile {
    /// 標準端末（80x24、全角2幅、ANSI対応）
    pub fn standard() -> Self {
        Self {
            name: "standard".to_string(),
            width: 80,
            height: 24,
            cjk_width: 2,
            ansi_enabled: true,
        }
    }

    /// Commodore 64（40x25、全角1幅、ANSIなし）
    pub fn c64() -> Self {
        Self {
            name: "c64".to_string(),
            width: 40,
            height: 25,
            cjk_width: 1,
            ansi_enabled: false,
        }
    }

    /// Commodore 64 ANSI対応版（将来用）
    pub fn c64_ansi() -> Self {
        Self {
            name: "c64_ansi".to_string(),
            width: 40,
            height: 25,
            cjk_width: 1,
            ansi_enabled: true,
        }
    }

    /// 文字列の表示幅を計算
    pub fn display_width(&self, s: &str) -> usize {
        if self.cjk_width == 1 {
            // C64モード: 全ての文字が1幅
            s.chars().count()
        } else {
            // 標準モード: 全角2幅、半角1幅
            s.chars().map(|c| {
                if c.is_ascii() { 1 } else { 2 }
            }).sum()
        }
    }

    /// 指定幅に収まるように文字列を切り詰め
    pub fn truncate_to_width(&self, s: &str, max_width: usize) -> String {
        let mut width = 0;
        let mut result = String::new();
        for c in s.chars() {
            let char_width = if self.cjk_width == 1 || c.is_ascii() { 1 } else { 2 };
            if width + char_width > max_width {
                break;
            }
            width += char_width;
            result.push(c);
        }
        result
    }
}
```

### 4.3 端末タイプ選択

端末タイプは以下のタイミングで選択・変更できる：

| タイミング | 対象 | 保存先 |
|------------|------|--------|
| 新規会員登録時 | 会員 | ユーザー設定（DB） |
| プロフィール編集 | 会員 | ユーザー設定（DB） |
| ゲストログイン時 | ゲスト | セッションのみ |

**会員の場合**：登録時に選択した端末タイプがDBに保存され、次回ログイン時に自動適用される。プロフィール編集画面からいつでも変更可能。

**ゲストの場合**：ゲストモード選択時に端末タイプを選択。セッション終了時に設定は破棄される。

```
端末タイプを選択してください:

[1] 標準端末 (80x24)
[2] Commodore 64 (40x25)
[3] Commodore 64 ANSI (40x25)

選択 >
```

選択された端末タイプはセッションに保存され、以降の画面描画に使用される。

### 4.4 NAWSネゴシエーション

標準端末ではNAWSオプションでクライアントからサイズを取得可能：

```
Server -> Client: IAC DO NAWS    (255 253 31)
Client -> Server: IAC WILL NAWS  (255 251 31)
Client -> Server: IAC SB NAWS <width-hi> <width-lo> <height-hi> <height-lo> IAC SE
```

### 4.5 画面描画の分岐

端末プロファイルに応じて描画を切り替え：

```rust
pub trait ScreenRenderer {
    fn render(&self, profile: &TerminalProfile) -> String;
}

impl ScreenRenderer for MainMenu {
    fn render(&self, profile: &TerminalProfile) -> String {
        if profile.ansi_enabled {
            self.render_ansi(profile.width)
        } else {
            self.render_plain(profile.width)
        }
    }
}
```

### 4.6 ANSI対応時とプレーンテキスト時の違い

| 要素 | ANSI対応時 | プレーンテキスト時 |
|------|------------|-------------------|
| タイトル | シアン色 | `===` で囲む |
| エラー | 赤色 `[エラー]` | `[!]` または `*エラー*` |
| 強調 | 太字/色付き | `*強調*` または `>強調<` |
| 罫線 | `─` (装飾文字) | `-` (ハイフン) |
| 選択項目 | 黄色 `[B]` | `[B]` そのまま |
| 未読マーク | 赤丸または色付き | `*` または `●` |

**プレーンテキストでのエラー表示例：**

```
*エラー* ユーザー名またはパスワードが間違っています
```

**ANSI対応時のエラー表示例：**

```
\x1b[31m[エラー]\x1b[0m ユーザー名またはパスワードが間違っています
```

## 5. ユーザー操作インターフェース

### 5.1 メニュー方式

基本的にメニュー番号または記号を入力して操作：

```
=== メインメニュー ===

[B] 掲示板
[C] チャット
[M] メール
[F] ファイル
[P] プロフィール
[Q] ログアウト

選択 >
```

### 5.2 ページング

長い表示にはページング処理：

```
-- More -- (Enter: 続き / Q: 終了)
```

### 5.3 入力プロンプト

| 種類 | プロンプト | 説明 |
|------|------------|------|
| コマンド入力 | `> ` | 一般的なコマンド待ち |
| パスワード入力 | `Password: ` | エコーなし |
| 本文入力 | `.` | ピリオド1文字で終了 |
| 確認 | `(Y/N) ` | Yes/No確認 |

### 5.4 本文入力モード

投稿やメール本文の入力：

```
本文を入力してください（終了は「.」のみの行）:
---
こんにちは。
これは投稿の本文です。

複数行入力できます。
.
---
投稿しますか？ (Y/N) >
```

## 6. ファイル転送

### 6.1 プロトコル

シンプルなテキストダンプ方式（バイナリ非対応）：

**ダウンロード**
```
ファイル名: example.txt
サイズ: 1234 bytes
---
(ファイル内容がそのまま出力)
---
ダウンロード完了
```

**アップロード**
```
ファイル名を入力: example.txt
本文を入力（終了は Ctrl+D または「.」のみの行）:
---
(ユーザーが内容を入力)
.
---
アップロード完了 (1234 bytes)
```

### 6.2 将来拡張（検討）

- XMODEM/YMODEM/ZMODEM対応
- バイナリファイル対応

## 7. エラーメッセージ

```rust
// エラー表示フォーマット
fn error(msg: &str) -> String {
    format!("\x1b[31m[エラー]\x1b[0m {}\r\n", msg)
}

// 警告表示フォーマット
fn warning(msg: &str) -> String {
    format!("\x1b[33m[警告]\x1b[0m {}\r\n", msg)
}

// 情報表示フォーマット
fn info(msg: &str) -> String {
    format!("\x1b[36m[情報]\x1b[0m {}\r\n", msg)
}
```

## 8. 接続シーケンス

```
1. TCP接続確立

2. Telnetネゴシエーション
   Server: IAC WILL ECHO, IAC WILL SGA
   Client: IAC DO ECHO, IAC DO SGA

3. ウェルカム画面表示
   - BBS名
   - 接続日時
   - 注意事項

4. ログインプロンプト
   - ユーザー名入力
   - パスワード入力（エコーなし）
   - または「new」で新規登録
   - または「guest」でゲストアクセス

5. 認証成功 → メインメニュー
   認証失敗 → 3回までリトライ → 切断

6. メインメニューからの操作

7. ログアウト → ウェルカム画面に戻る
   または切断
```
