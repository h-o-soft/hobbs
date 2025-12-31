# HOBBS - 機能仕様書: テンプレート・カスタマイズ・国際化

## 1. 概要

HOBBSをBBSエンジンとして、様々な名前・デザインのBBSを運営できるようにする。
画面レイアウト、メッセージ、多言語対応を外部ファイルで管理する。

## 2. 設計方針

| 項目 | 決定 |
|------|------|
| テンプレートエンジン | 自前実装（軽量・依存少） |
| 言語設定 | システム全体で統一（config.tomlで指定） |
| テンプレートリロード | 起動時のみ読み込み |
| 初期対応言語 | 日本語（ja）・英語（en） |

**注意**: 国際化対応は初期実装から必須。全てのユーザー向けメッセージは言語リソースファイルから取得すること。

## 3. カスタマイズ可能な要素

| 要素 | カスタマイズ | 優先度 |
|------|-------------|--------|
| ウェルカム画面 | 完全カスタマイズ | 必須 |
| メインメニュー | 部分カスタマイズ | 必須 |
| ヘルプ画面 | 完全カスタマイズ | 必須 |
| メニュー項目 | 表示テキストのみ | 努力目標 |
| エラーメッセージ | 文字列リソース | 必須 |
| 成功メッセージ | 文字列リソース | 必須 |
| システムメッセージ | 文字列リソース | 必須 |

## 4. テンプレートシステム

### 4.1 ディレクトリ構成

```
templates/
├── 80/                  # 80カラム用（標準端末）
│   ├── welcome.txt      # ウェルカム画面
│   ├── main_menu.txt    # メインメニュー
│   └── help.txt         # ヘルプ画面
└── 40/                  # 40カラム用（C64等）
    ├── welcome.txt
    ├── main_menu.txt
    └── help.txt
```

端末プロファイルの `width` に応じて、適切なディレクトリからテンプレートを読み込む：
- 幅 > 60: `80/` を使用
- 幅 <= 60: `40/` を使用

### 4.2 テンプレート変数

テンプレート内で `{{変数名}}` 形式で動的値を埋め込む。

#### システム変数

| 変数 | 説明 | 例 |
|------|------|-----|
| `{{bbs.name}}` | BBS名 | レトロBBS ほっとステーション |
| `{{bbs.sysop}}` | SysOp名 | まさお |
| `{{bbs.description}}` | BBS説明 | 懐かしのパソコン通信BBS |
| `{{system.date}}` | 現在日付 | 2025/01/15 |
| `{{system.time}}` | 現在時刻 | 20:30:45 |
| `{{system.datetime}}` | 日時 | 2025/01/15 20:30:45 |
| `{{system.online}}` | 接続中ユーザー数 | 3 |

#### ユーザー変数（ログイン後のみ）

| 変数 | 説明 | 例 |
|------|------|-----|
| `{{user.name}}` | ユーザー名 | たろう |
| `{{user.nickname}}` | ニックネーム | たろう |
| `{{user.role}}` | 権限 | 会員 / SubOp / SysOp |
| `{{user.last_login}}` | 最終ログイン | 2025/01/14 18:20 |
| `{{user.unread_mail}}` | 未読メール数 | 2 |
| `{{user.unread_posts}}` | 未読投稿数 | 17 |
| `{{user.is_sysop}}` | SysOpか（条件用） | true / false |
| `{{user.is_subop}}` | SubOp以上か | true / false |

#### 翻訳参照

| 構文 | 説明 |
|------|------|
| `{{t "キー"}}` | 言語リソースから翻訳を取得 |

### 4.3 条件分岐

```
{{#if 条件}}
  条件が真の場合の内容
{{/if}}
```

**例：**
```
{{#if user.is_sysop}}
  [A] 管理メニュー
{{/if}}

{{#if user.unread_mail}}
  メール (未読: {{user.unread_mail}}通)
{{/if}}
```

### 4.4 テンプレート例

**templates/80/welcome.txt:**
```
================================================================================
     _   _  ___  ____  ____  ____
    | | | |/ _ \| __ )| __ )/ ___|
    | |_| | | | |  _ \|  _ \\___ \
    |  _  | |_| | |_) | |_) |___) |
    |_| |_|\___/|____/|____/|____/

                    {{bbs.name}}
================================================================================

    {{t "welcome.greeting"}}

    {{t "welcome.connect_time"}}: {{system.datetime}}

    {{bbs.description}}

================================================================================
    [L] {{t "menu.login"}}    [N] {{t "menu.register"}}    [G] {{t "menu.guest"}}    [Q] {{t "menu.quit"}}
--------------------------------------------------------------------------------
{{t "prompt.select"}} >
```

**templates/40/welcome.txt:**
```
========================================
    _  _  ___  ___  ___  ___
   | || |/ _ \| _ )| _ )/ __|
   | __ | (_) | _ \| _ \\__ \
   |_||_|\___/|___/|___/|___/

  {{bbs.name}}
========================================

  {{t "welcome.greeting"}}

  {{t "welcome.connect_time"}}:
  {{system.datetime}}

  {{bbs.description}}

========================================
[L]{{t "menu.login"}} [N]{{t "menu.register"}}
[G]{{t "menu.guest"}} [Q]{{t "menu.quit"}}
----------------------------------------
{{t "prompt.select"}} >
```

## 5. 言語リソースシステム

### 5.1 ディレクトリ構成

```
locales/
├── ja.toml              # 日本語（デフォルト）
└── en.toml              # 英語
```

### 5.2 言語ファイル形式（TOML）

```toml
[meta]
name = "日本語"
code = "ja"

[welcome]
greeting = "ようこそ！"
connect_time = "接続日時"

[menu]
login = "ログイン"
register = "新規登録"
guest = "ゲスト"
quit = "切断"
board = "掲示板"
chat = "チャット"
mail = "メール"
file = "ファイル"
profile = "プロフィール"
admin = "管理メニュー"
logout = "ログアウト"
back = "戻る"
next_page = "次ページ"
prev_page = "前ページ"

[prompt]
select = "選択"
input = "入力"
confirm = "確認"
yes = "はい"
no = "いいえ"

[error]
login_failed = "ユーザー名またはパスワードが間違っています"
permission_denied = "権限がありません"
not_found = "見つかりません"
invalid_input = "入力が正しくありません"
session_expired = "セッションが切れました"

[success]
login = "ログインしました"
logout = "ログアウトしました"
post_created = "投稿が完了しました"
mail_sent = "メールを送信しました"
registered = "登録が完了しました"

[format]
unread_mail = "未読: {count}通"
unread_posts = "未読: {count}件"
online_users = "現在 {count}人"
page_info = "ページ: {current}/{total}"
datetime = "{year}/{month}/{day} {hour}:{minute}"

[label]
username = "ユーザーID"
password = "パスワード"
nickname = "ニックネーム"
email = "メールアドレス"
```

### 5.3 プレースホルダー

言語リソース内で `{変数名}` 形式で動的値を埋め込む：

```toml
unread_mail = "未読: {count}通"
page_info = "ページ: {current}/{total}"
```

**使用例：**
```rust
// コード内での使用
i18n.translate_with("format.unread_mail", &[("count", "5")]);
// → "未読: 5通"

i18n.translate_with("format.page_info", &[("current", "1"), ("total", "5")]);
// → "ページ: 1/5"
```

## 6. 設定ファイル

### 6.1 config.toml

```toml
[bbs]
name = "レトロBBS ほっとステーション"
description = "懐かしのパソコン通信を再現したBBSです"
sysop_name = "まさお"

[locale]
language = "ja"          # 使用言語（ja / en）

[templates]
path = "templates"       # テンプレートディレクトリ
```

## 7. 実装仕様

### 7.1 構造体

```rust
/// テンプレートエンジン
pub struct TemplateEngine {
    templates: HashMap<(TerminalWidth, String), Template>,
    i18n: I18n,
}

/// 端末幅分類
pub enum TerminalWidth {
    Wide,   // 80カラム（幅 > 60）
    Narrow, // 40カラム（幅 <= 60）
}

impl From<&TerminalProfile> for TerminalWidth {
    fn from(profile: &TerminalProfile) -> Self {
        if profile.width > 60 {
            TerminalWidth::Wide
        } else {
            TerminalWidth::Narrow
        }
    }
}

/// テンプレート
pub struct Template {
    content: String,
}

/// 国際化
pub struct I18n {
    locales: HashMap<String, Locale>,
    current_locale: String,
}

/// 言語リソース
pub struct Locale {
    code: String,
    name: String,
    messages: HashMap<String, String>,
}

/// テンプレートコンテキスト
pub struct TemplateContext {
    pub bbs: BbsInfo,
    pub system: SystemInfo,
    pub user: Option<UserInfo>,
}

pub struct BbsInfo {
    pub name: String,
    pub sysop: String,
    pub description: String,
}

pub struct SystemInfo {
    pub date: String,
    pub time: String,
    pub datetime: String,
    pub online: u32,
}

pub struct UserInfo {
    pub name: String,
    pub nickname: String,
    pub role: String,
    pub last_login: String,
    pub unread_mail: u32,
    pub unread_posts: u32,
    pub is_sysop: bool,
    pub is_subop: bool,
}
```

### 7.2 API

```rust
impl TemplateEngine {
    /// テンプレートとロケールをロード
    pub fn load(
        templates_path: &Path,
        locales_path: &Path,
        locale: &str,
    ) -> Result<Self>;

    /// テンプレートを描画
    pub fn render(
        &self,
        name: &str,                    // "welcome", "main_menu", etc.
        profile: &TerminalProfile,     // 端末プロファイル
        context: &TemplateContext,     // 変数
    ) -> Result<String>;

    /// 翻訳を取得
    pub fn t(&self, key: &str) -> &str;

    /// 翻訳（プレースホルダー付き）
    pub fn t_with(&self, key: &str, vars: &[(&str, &str)]) -> String;
}

impl I18n {
    /// ロケールをロード
    pub fn load(path: &Path, locale: &str) -> Result<Self>;

    /// 翻訳を取得（キーがない場合はキー自体を返す）
    pub fn translate(&self, key: &str) -> &str;

    /// 翻訳（プレースホルダー置換）
    pub fn translate_with(&self, key: &str, vars: &[(&str, &str)]) -> String;
}
```

### 7.3 テンプレート描画フロー

```
1. テンプレート名と端末プロファイルから適切なテンプレートを選択
2. テンプレート内の変数を展開
   - {{変数名}} → コンテキストの値に置換
   - {{t "キー"}} → 言語リソースから翻訳を取得
3. 条件分岐を評価
   - {{#if 条件}}...{{/if}} を処理
4. 描画結果を返す
```

## 8. エラーハンドリング

| 状況 | 動作 |
|------|------|
| テンプレートが見つからない | デフォルトテンプレートを使用 |
| 変数が見つからない | 空文字に置換 |
| 翻訳キーが見つからない | キー自体を表示 |
| ロケールファイルが見つからない | デフォルト（ja）を使用 |

## 9. 制限事項

| 項目 | 制限 |
|------|------|
| テンプレートサイズ | 最大64KB |
| 言語リソースサイズ | 最大64KB |
| ネストした条件分岐 | 非サポート |
| ループ構文 | 非サポート |
