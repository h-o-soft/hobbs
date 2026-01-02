# HOBBS スクリプトプラグイン機能 設計ドキュメント

## 1. 概要

### 1.1 目的

HOBBSにスクリプト実行機能を追加し、SysOp/SubOpがゲームや対話型コンテンツを作成できるようにする。昔のパソコン通信における「ドアゲーム」のような体験を提供する。

### 1.2 ユースケース

- **ドアゲーム**: テキストアドベンチャー、クイズ、じゃんけん、数当てゲーム
- **対話型ツール**: 占い、おみくじ、ダイス、電卓
- **カスタム機能**: アンケート、投票、自動応答Bot
- **SysOp拡張**: カスタムメニュー、統計表示、メンテナンスツール

---

## 2. 技術選定

### 2.1 組み込み言語の比較

| 言語 | クレート | メモリ | 非同期 | サンドボックス | Rust親和性 | 学習曲線 |
|------|----------|--------|--------|----------------|------------|----------|
| **Lua** | `mlua` | ~500KB | async対応 | 関数無効化 | ◯ | 低い |
| **Rhai** | `rhai` | ~1MB | async対応 | 完全分離 | ◎ | 低い |
| JavaScript | `rquickjs` | ~2MB | △ | ◯ | ◯ | 低い |
| Python | `pyo3` | ~50MB | × | × | △ | 低い |
| WASM | `wasmtime` | ~10MB | 対応 | ◎ | △ | 高い |

### 2.2 採用: Lua (`mlua`)

**選定理由:**
1. **軽量**: メモリフットプリントが小さい（サーバー向き）
2. **実績**: 30年以上の歴史、ゲーム組み込みの業界標準
3. **シンプル**: 学習コストが低い、ドキュメント豊富
4. **非同期対応**: `mlua` の async feature で tokio と統合可能
5. **サンドボックス**: `os`, `io`, `loadfile` 等を無効化可能

### 2.3 依存クレート

```toml
[dependencies]
mlua = { version = "0.10", features = ["lua54", "async", "serialize"] }
```

---

## 3. システムアーキテクチャ

### 3.1 全体構成

```
┌─────────────────────────────────────────────────────────┐
│                     SessionHandler                       │
│  ┌─────────────────────────────────────────────────┐    │
│  │              MenuAction::Script                  │    │
│  └──────────────────────┬──────────────────────────┘    │
│                         │                                │
│  ┌──────────────────────▼──────────────────────────┐    │
│  │              ScriptScreen                        │    │
│  │  - スクリプト一覧表示                            │    │
│  │  - スクリプト選択・実行                          │    │
│  │  - 管理機能（SubOp/SysOp向け）                   │    │
│  └──────────────────────┬──────────────────────────┘    │
│                         │                                │
│  ┌──────────────────────▼──────────────────────────┐    │
│  │              ScriptEngine                        │    │
│  │  ┌─────────────────────────────────────────┐    │    │
│  │  │            Lua Runtime (mlua)            │    │    │
│  │  │  ┌─────────────────────────────────┐    │    │    │
│  │  │  │         Sandbox Layer           │    │    │    │
│  │  │  │  - os, io, loadfile 無効化      │    │    │    │
│  │  │  │  - 命令数制限                   │    │    │    │
│  │  │  │  - メモリ制限                   │    │    │    │
│  │  │  └─────────────────────────────────┘    │    │    │
│  │  │  ┌─────────────────────────────────┐    │    │    │
│  │  │  │          BBS API Layer          │    │    │    │
│  │  │  │  - print(), input()             │    │    │    │
│  │  │  │  - get_user(), get_time()       │    │    │    │
│  │  │  │  - random(), sleep()            │    │    │    │
│  │  │  │  - db.get(), db.set()           │    │    │    │
│  │  │  └─────────────────────────────────┘    │    │    │
│  │  └─────────────────────────────────────────┘    │    │
│  └─────────────────────────────────────────────────┘    │
│                                                          │
│  ┌─────────────────────────────────────────────────┐    │
│  │              ScriptService                       │    │
│  │  - list_scripts()                                │    │
│  │  - get_script()                                  │    │
│  │  - create_script() (SubOp+)                      │    │
│  │  - update_script() (SubOp+)                      │    │
│  │  - delete_script() (SubOp+)                      │    │
│  └──────────────────────┬──────────────────────────┘    │
│                         │                                │
│  ┌──────────────────────▼──────────────────────────┐    │
│  │              ScriptRepository                    │    │
│  │              ScriptDataRepository                │    │
│  └─────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────┘
```

### 3.2 コンポーネント責務

| コンポーネント | 責務 |
|---------------|------|
| **ScriptScreen** | UI層。スクリプト一覧表示、選択、管理画面 |
| **ScriptEngine** | Lua実行環境。サンドボックス、BBS API提供 |
| **ScriptService** | ビジネスロジック。権限チェック、CRUD操作 |
| **ScriptRepository** | DBアクセス。スクリプトメタデータ管理 |
| **ScriptDataRepository** | DBアクセス。スクリプト固有データ（セーブデータ等） |

---

## 4. BBS API 設計

### 4.1 基本API（Lua関数）

```lua
-- === 入出力 ===
bbs.print(text)              -- テキスト出力（改行なし）
bbs.println(text)            -- テキスト出力（改行あり）
bbs.input(prompt)            -- ユーザー入力を取得
bbs.input_number(prompt)     -- 数値入力（バリデーション付き）
bbs.input_yn(prompt)         -- Y/N入力
bbs.clear()                  -- 画面クリア（ANSIの場合）
bbs.pause()                  -- Press Enter to continue...

-- === ユーザー情報 ===
bbs.get_user()               -- { id, username, nickname, role } を返す
bbs.is_guest()               -- ゲストかどうか
bbs.is_sysop()               -- SysOpかどうか

-- === ユーティリティ ===
bbs.random(min, max)         -- 乱数生成
bbs.sleep(seconds)           -- 待機（最大5秒）
bbs.get_time()               -- 現在時刻
bbs.get_date()               -- 現在日付

-- === 永続化（スクリプト固有データ） ===
bbs.data.get(key)            -- データ取得
bbs.data.set(key, value)     -- データ保存
bbs.data.delete(key)         -- データ削除

-- === ユーザー固有データ ===
bbs.user_data.get(key)       -- ユーザー別データ取得
bbs.user_data.set(key, value)-- ユーザー別データ保存

-- === 端末情報 ===
bbs.terminal.width           -- 端末幅
bbs.terminal.height          -- 端末高さ
bbs.terminal.has_ansi        -- ANSI対応かどうか
```

### 4.2 API使用例

```lua
-- じゃんけんゲーム
bbs.println("=== じゃんけんゲーム ===")
bbs.println("")

local user = bbs.get_user()
bbs.println("こんにちは、" .. user.nickname .. "さん！")
bbs.println("")

-- 戦績読み込み
local wins = bbs.user_data.get("wins") or 0
local losses = bbs.user_data.get("losses") or 0
bbs.println("戦績: " .. wins .. "勝 " .. losses .. "敗")
bbs.println("")

-- ゲームループ
while true do
    bbs.println("[1] グー  [2] チョキ  [3] パー  [Q] 終了")
    local choice = bbs.input("> ")

    if choice:upper() == "Q" then
        break
    end

    local player = tonumber(choice)
    if player and player >= 1 and player <= 3 then
        local cpu = bbs.random(1, 3)
        local hands = {"グー", "チョキ", "パー"}

        bbs.println("あなた: " .. hands[player])
        bbs.println("CPU: " .. hands[cpu])

        if player == cpu then
            bbs.println("あいこ！")
        elseif (player == 1 and cpu == 2) or
               (player == 2 and cpu == 3) or
               (player == 3 and cpu == 1) then
            bbs.println("あなたの勝ち！")
            wins = wins + 1
            bbs.user_data.set("wins", wins)
        else
            bbs.println("あなたの負け...")
            losses = losses + 1
            bbs.user_data.set("losses", losses)
        end
    end
    bbs.println("")
end

bbs.println("またね！")
```

---

## 5. サンドボックス設計

### 5.1 無効化する標準ライブラリ

```rust
// 危険な関数を無効化
lua.globals().set("os", mlua::Nil)?;           // OSアクセス
lua.globals().set("io", mlua::Nil)?;           // ファイルI/O
lua.globals().set("loadfile", mlua::Nil)?;     // ファイル読み込み
lua.globals().set("dofile", mlua::Nil)?;       // ファイル実行
lua.globals().set("load", mlua::Nil)?;         // 動的コード実行（制限付き）
lua.globals().set("require", mlua::Nil)?;      // モジュール読み込み
lua.globals().set("package", mlua::Nil)?;      // パッケージシステム
lua.globals().set("debug", mlua::Nil)?;        // デバッグライブラリ
```

### 5.2 リソース制限

| 制限項目 | デフォルト値 | 設定可能範囲 |
|---------|-------------|-------------|
| 命令数上限 | 1,000,000 | 100,000 - 10,000,000 |
| メモリ上限 | 10MB | 1MB - 100MB |
| 実行時間上限 | 30秒 | 5秒 - 300秒 |
| sleep最大秒数 | 5秒 | - |
| データサイズ上限 | 1MB/スクリプト | - |

### 5.3 制限の実装

```rust
// 命令数制限（Hookを使用）
lua.set_hook(
    mlua::HookTriggers::every_nth_instruction(10000),
    |_lua, _debug| {
        // カウンター確認、上限超過でエラー
        Ok(())
    },
)?;

// メモリ制限
lua.set_memory_limit(10 * 1024 * 1024)?; // 10MB
```

---

## 6. スクリプト管理方式

### 6.1 ファイルシステムベース

スクリプトはファイルシステム上のディレクトリに配置し、起動時に自動的にスキャン・同期する。

```
scripts/                          # スクリプトディレクトリ（config.tomlで設定）
├── janken.lua                   # じゃんけんゲーム
├── number_guess.lua             # 数当てゲーム
├── omikuji.lua                  # おみくじ
└── quiz/                        # サブディレクトリも可
    └── trivia.lua
```

### 6.2 スクリプトファイルフォーマット

各Luaファイルの先頭にメタデータをコメントとして記述：

```lua
-- @name じゃんけん
-- @description じゃんけんゲーム。勝敗記録付き。
-- @author SysOp
-- @min_role 0
-- @enabled true

bbs.println("=== じゃんけんゲーム ===")
-- ...
```

### 6.3 scripts テーブル（メタデータキャッシュ）

```sql
CREATE TABLE scripts (
    id INTEGER PRIMARY KEY,
    file_path TEXT NOT NULL UNIQUE,      -- ファイルパス（相対パス）
    name TEXT NOT NULL,                   -- スクリプト名（メタデータから）
    slug TEXT NOT NULL UNIQUE,           -- URL-safe識別子（ファイル名から生成）
    description TEXT,                     -- 説明文（メタデータから）
    author TEXT,                          -- 作成者（メタデータから）
    file_hash TEXT,                       -- ファイルハッシュ（変更検知用）
    synced_at TIMESTAMP,                  -- 最終同期日時
    min_role INTEGER DEFAULT 0,          -- 実行に必要な権限
    enabled BOOLEAN DEFAULT 1,           -- 有効/無効

    -- リソース制限（個別設定）
    max_instructions INTEGER DEFAULT 1000000,
    max_memory_mb INTEGER DEFAULT 10,
    max_execution_seconds INTEGER DEFAULT 30
);

CREATE INDEX idx_scripts_enabled ON scripts(enabled);
CREATE INDEX idx_scripts_min_role ON scripts(min_role);
```

### 6.4 同期の仕組み

1. **起動時スキャン**: スクリプトディレクトリを再帰的にスキャン
2. **変更検知**: ファイルハッシュを比較し、変更があればメタデータを再解析
3. **新規ファイル**: DBに追加
4. **削除されたファイル**: DBから削除（または無効化）
5. **メニューからの再同期**: SysOpが手動で同期を実行可能

### 6.5 script_data テーブル（スクリプト固有データ）

```sql
CREATE TABLE script_data (
    id INTEGER PRIMARY KEY,
    script_id INTEGER NOT NULL,
    user_id INTEGER,                     -- NULL = 全体データ、非NULL = ユーザー固有
    key TEXT NOT NULL,
    value TEXT NOT NULL,                 -- JSON形式で保存
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,

    FOREIGN KEY(script_id) REFERENCES scripts(id) ON DELETE CASCADE,
    FOREIGN KEY(user_id) REFERENCES users(id) ON DELETE CASCADE,

    UNIQUE(script_id, user_id, key)
);

CREATE INDEX idx_script_data_script ON script_data(script_id);
CREATE INDEX idx_script_data_user ON script_data(user_id);
```

### 6.6 script_logs テーブル（実行ログ、オプション）

```sql
CREATE TABLE script_logs (
    id INTEGER PRIMARY KEY,
    script_id INTEGER NOT NULL,
    user_id INTEGER,
    executed_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    execution_time_ms INTEGER,
    success BOOLEAN,
    error_message TEXT,

    FOREIGN KEY(script_id) REFERENCES scripts(id) ON DELETE CASCADE,
    FOREIGN KEY(user_id) REFERENCES users(id) ON DELETE SET NULL
);

CREATE INDEX idx_script_logs_script ON script_logs(script_id);
CREATE INDEX idx_script_logs_executed_at ON script_logs(executed_at);
```

---

## 7. 画面フロー

### 7.1 メインメニューからの遷移

```
Main Menu
    │
    └─ [S] Scripts/Games
           │
           ├─ スクリプト一覧表示
           │   ├─ [1] じゃんけん
           │   ├─ [2] 数当てゲーム
           │   ├─ [3] おみくじ
           │   └─ [Q] 戻る
           │
           ├─ 番号選択 → スクリプト実行
           │   └─ 実行終了後 → 一覧に戻る
           │
           └─ (SubOp/SysOp) [A] 管理
                   │
                   ├─ [1] スクリプト再同期
                   ├─ [2] 有効/無効切替
                   ├─ [3] スクリプト配置ガイド表示
                   └─ [Q] 戻る
```

### 7.2 スクリプト実行画面

```
=== じゃんけん ===
作者: SysOp  |  実行回数: 1,234回

[R] 実行  [B] 戻る
> R

--- スクリプト実行開始 ---

こんにちは、ユーザーさん！
戦績: 5勝 3敗

[1] グー  [2] チョキ  [3] パー  [Q] 終了
> 1
あなた: グー
CPU: チョキ
あなたの勝ち！

[1] グー  [2] チョキ  [3] パー  [Q] 終了
> Q
またね！

--- スクリプト実行終了 ---
Press Enter to continue...
```

---

## 8. アプリケーション例

### 8.1 数当てゲーム

```lua
bbs.println("=== 数当てゲーム ===")
bbs.println("1から100までの数を当ててください")
bbs.println("")

local answer = bbs.random(1, 100)
local attempts = 0

while true do
    local guess = bbs.input_number("予想: ")
    if not guess then
        bbs.println("数字を入力してください")
    else
        attempts = attempts + 1

        if guess < answer then
            bbs.println("もっと大きい！")
        elseif guess > answer then
            bbs.println("もっと小さい！")
        else
            bbs.println("正解！ " .. attempts .. "回で当たりました！")

            -- ハイスコア更新
            local best = bbs.user_data.get("best_score")
            if not best or attempts < best then
                bbs.user_data.set("best_score", attempts)
                bbs.println("新記録！")
            end
            break
        end
    end
end
```

### 8.2 おみくじ

```lua
bbs.println("=== おみくじ ===")
bbs.println("")

-- 今日既に引いたか確認
local today = bbs.get_date()
local last_date = bbs.user_data.get("last_omikuji_date")

if last_date == today then
    local result = bbs.user_data.get("last_omikuji_result")
    bbs.println("今日はもう引きました")
    bbs.println("結果: " .. result)
else
    bbs.println("おみくじを引きますか？ [Y/N]")
    if bbs.input_yn("> ") then
        local fortunes = {
            "大吉", "吉", "中吉", "小吉", "末吉", "凶"
        }
        local weights = {10, 20, 25, 25, 15, 5}

        -- 重み付き抽選
        local total = 0
        for _, w in ipairs(weights) do
            total = total + w
        end
        local roll = bbs.random(1, total)
        local cumulative = 0
        local result = fortunes[1]
        for i, w in ipairs(weights) do
            cumulative = cumulative + w
            if roll <= cumulative then
                result = fortunes[i]
                break
            end
        end

        bbs.println("")
        bbs.println("  ┏━━━━━━━━━┓")
        bbs.println("  ┃ " .. result .. " ┃")
        bbs.println("  ┗━━━━━━━━━┛")
        bbs.println("")

        -- 保存
        bbs.user_data.set("last_omikuji_date", today)
        bbs.user_data.set("last_omikuji_result", result)
    end
end
```

### 8.3 テキストアドベンチャー（簡易版）

```lua
bbs.println("=== 冒険の始まり ===")
bbs.println("")

-- セーブデータ読み込み
local scene = bbs.user_data.get("current_scene") or "start"
local inventory = bbs.user_data.get("inventory") or {}

local scenes = {
    start = {
        text = [[
あなたは古い城の入り口に立っている。
重い木の扉が目の前にある。

[1] 扉を開ける
[2] 周りを調べる
]],
        choices = {
            ["1"] = "hall",
            ["2"] = "entrance_search"
        }
    },
    entrance_search = {
        text = [[
入り口の周りを調べると、石の下に鍵を見つけた！
（鍵を手に入れた）

[1] 扉を開ける
]],
        on_enter = function()
            table.insert(inventory, "key")
            bbs.user_data.set("inventory", inventory)
        end,
        choices = {
            ["1"] = "hall"
        }
    },
    hall = {
        text = [[
広いホールに入った。
正面に階段、左に廊下、右に扉がある。

[1] 階段を上る
[2] 左の廊下へ
[3] 右の扉を開ける
]],
        choices = {
            ["1"] = "upstairs",
            ["2"] = "corridor",
            ["3"] = "right_room"
        }
    },
    -- ... 他のシーン ...
}

-- ゲームループ
while scene ~= "end" do
    local s = scenes[scene]
    if s.on_enter then s.on_enter() end

    bbs.println(s.text)
    local choice = bbs.input("> ")

    if s.choices[choice] then
        scene = s.choices[choice]
        bbs.user_data.set("current_scene", scene)
    else
        bbs.println("その選択肢はありません")
    end
    bbs.println("")
end

bbs.println("=== GAME OVER ===")
bbs.user_data.delete("current_scene")
```

### 8.4 アンケート/投票

```lua
bbs.println("=== 今月のアンケート ===")
bbs.println("好きなプログラミング言語は？")
bbs.println("")

-- 既に投票済みか確認
local voted = bbs.user_data.get("voted_202501")
if voted then
    bbs.println("既に投票済みです（" .. voted .. "）")
else
    bbs.println("[1] Rust")
    bbs.println("[2] Python")
    bbs.println("[3] JavaScript")
    bbs.println("[4] その他")

    local choice = bbs.input("> ")
    local options = {
        ["1"] = "Rust",
        ["2"] = "Python",
        ["3"] = "JavaScript",
        ["4"] = "その他"
    }

    if options[choice] then
        local key = "votes_" .. options[choice]
        local count = bbs.data.get(key) or 0
        bbs.data.set(key, count + 1)
        bbs.user_data.set("voted_202501", options[choice])
        bbs.println("投票しました！")
    end
end

-- 結果表示
bbs.println("")
bbs.println("=== 現在の結果 ===")
for _, lang in ipairs({"Rust", "Python", "JavaScript", "その他"}) do
    local count = bbs.data.get("votes_" .. lang) or 0
    bbs.println(lang .. ": " .. count .. "票")
end
```

---

## 9. セキュリティ考慮事項

### 9.1 リスクと対策

| リスク | 対策 |
|-------|------|
| 無限ループ | 命令数制限、タイムアウト |
| メモリ枯渇 | メモリ上限設定 |
| ファイルアクセス | io, os ライブラリ無効化 |
| 外部通信 | socket無効化、HTTP API なし |
| コードインジェクション | load/loadstring 無効化 |
| データ破壊 | スクリプト毎にデータ分離 |
| DoS攻撃 | sleep制限、実行時間制限 |
| 他ユーザーデータアクセス | user_data は自分のみ |

### 9.2 権限モデル

```
Guest     → min_role=0 のスクリプトのみ実行可能
Member    → min_role≤1 のスクリプト実行可能
SubOp     → min_role≤2 のスクリプト実行可能、スクリプト作成・編集・削除可能
SysOp     → 全スクリプト実行可能、全管理機能、他ユーザーのスクリプトも編集可能
```

**決定事項**: スクリプト作成権限は **SubOp以上** に付与

### 9.3 入力サニタイズ

- ユーザー入力は自動的にトリム
- ANSIエスケープシーケンスは出力時にフィルタリング（オプション）
- 長すぎる入力は切り詰め

---

## 10. 実装フェーズ

### Phase 1: 基盤構築（MVP）

**目標**: 最小限のスクリプト実行機能

1. `mlua` クレート追加
2. `src/script/` モジュール作成
   - `mod.rs`
   - `engine.rs` - ScriptEngine（サンドボックス付きLua実行）
   - `repository.rs` - ScriptRepository
   - `service.rs` - ScriptService
3. DBマイグレーション（scripts テーブル）
4. 基本BBS API実装
   - `bbs.print()`, `bbs.println()`, `bbs.input()`
   - `bbs.get_user()`, `bbs.random()`
5. `ScriptScreen` 基本実装（一覧・実行）
6. メインメニューに [S] 追加

**成果物**: 簡単なスクリプト（Hello World、じゃんけん）が動作

### Phase 2: 永続化と管理機能

**目標**: データ保存とSubOp/SysOp管理画面

1. `script_data` テーブル追加
2. BBS API拡張
   - `bbs.data.get/set/delete()`
   - `bbs.user_data.get/set/delete()`
3. ScriptScreen管理機能
   - スクリプト再同期
   - 有効/無効切替
   - 配置ガイド表示
4. i18nメッセージ追加

**成果物**: セーブ機能付きゲームが動作、SubOp/SysOpが管理可能

### Phase 3: 拡張API と安定化

**目標**: 機能拡充とセキュリティ強化

1. BBS API拡張
   - `bbs.input_number()`, `bbs.input_yn()`
   - `bbs.clear()`, `bbs.pause()`
   - `bbs.sleep()`, `bbs.get_time()`, `bbs.get_date()`
   - `bbs.terminal.*`
2. リソース制限強化
   - 命令数Hook実装
   - メモリ制限
   - スクリプト毎の個別設定
3. 実行ログ（オプション）
4. エラーハンドリング改善

**成果物**: 本格的なゲームが作成可能、安定運用

### Phase 4: 応用機能（オプション）

**目標**: 高度な機能

1. スクリプトテンプレート（サンプル集）
2. スクリプト間連携（オプション）
3. 統計・ランキング機能

---

## 11. ファイル構成（予定）

```
src/
├── script/
│   ├── mod.rs              # pub use
│   ├── engine.rs           # ScriptEngine（Lua実行環境）
│   ├── api.rs              # BBS API実装
│   ├── sandbox.rs          # サンドボックス設定
│   ├── loader.rs           # ScriptLoader（ファイルスキャン、同期）
│   ├── repository.rs       # ScriptRepository
│   ├── data_repository.rs  # ScriptDataRepository
│   ├── service.rs          # ScriptService
│   └── types.rs            # Script, ScriptData型
│
├── app/screens/
│   └── script.rs           # ScriptScreen（新規）
│
├── db/
│   └── schema.rs           # マイグレーション追加
│
└── lib.rs                  # pub mod script 追加

scripts/                      # スクリプト配置ディレクトリ
├── janken.lua               # サンプル: じゃんけん
├── number_guess.lua         # サンプル: 数当て
└── omikuji.lua              # サンプル: おみくじ
```

---

## 12. 参考情報

### mlua ドキュメント
- https://docs.rs/mlua/latest/mlua/
- https://github.com/mlua-rs/mlua

### Lua 5.4 リファレンス
- https://www.lua.org/manual/5.4/

### 類似実装
- MUD/MOO のスクリプトシステム
- IRC Bot のプラグインシステム
