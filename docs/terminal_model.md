# 端末・文字コードのしくみ（現状）

Telnet 接続での「文字コード・端末プロファイル・出力モード・言語」の扱いを、コードの実態に合わせてまとめたもの。
Issue #325（挙動を変えないリファクタリング）の結果を反映している。設計の経緯は `plan.md` を参照。

> ここに書いてある「癖（Quirk）」は**現状の挙動**であり、正しい挙動ではない。
> 直すのは今後の挙動修正フェーズ（plan.md の F1〜F4）で行う。

## 1. 4つの設定と置き場所

| 設定 | 意味 | 置き場所 |
|---|---|---|
| encoding | 回線上の文字コード（ShiftJIS / UTF-8 / CP437 / PETSCII） | `TelnetSession` の `TerminalSettings.encoding` |
| output_mode | エスケープシーケンスの扱い（Ansi: そのまま送る / Plain: 消す / PetsciiCtrl: PETSCII の制御コードに変換） | `TerminalSettings.output_mode` |
| profile | 画面の幅・高さ、全角の幅（cjk_width）、テンプレートの組（80/40） | `TerminalSettings.profile` |
| language | 画面の言語（ja / en） | `TerminalSettings.language` |

- この4つは `src/terminal/settings.rs` の `TerminalSettings` にまとめてあり、`TelnetSession` が持つ。
- `profile.encoding` と `profile.output_mode` は「そのプロファイルの既定値」である。実際に回線で使う値は `TerminalSettings.encoding` と `TerminalSettings.output_mode` で、両者は食い違うことがある（下記の Q1、Q8）。
- `SessionHandler` は、入力の文字コード（`line_buffer`）と i18n を、`apply_settings` で settings から導く。
- output_mode は、常に encoding と合う値にする（`resolve` の `compatible_output_mode`）。PETSCII の制御コードは PETSCII でしか送らず、ANSI は PETSCII では送らない。

## 2. いつ・何から決まるか

値を決める規則は `terminal::settings::resolve` の純粋関数に集めてあり、`SessionHandler::apply_settings` が適用する。

| タイミング | 関数 | profile | encoding | output_mode | language |
|---|---|---|---|---|---|
| 接続直後 | `on_connect` | `terminal.default_profile`（組み込みのみ） | 変えない（新しいセッションは ShiftJIS） | プロファイルの値を文字コードに合わせたもの | `locale.language` |
| 接続方式の選択 | `on_connection_selected` | `default_profile` の文字コードが同じならそれ、違えば接続方式の組み込みプロファイル | 接続方式の文字コード | プロファイルの値を文字コードに合わせたもの | 1・2: ja、3・4・5: en |
| ログイン | `on_login` | `users.terminal`（カスタムプロファイルを先に探す）。ただし、その幅や配置が今の文字コードに合わなければ（例: PC からの接続に c64）、接続方式のプロファイル | **変えない**（接続方式のまま。`users.encoding` は使わない） | 同上 | `users.language`。ただし CP437・PETSCII なら en |
| 設定画面で保存 | `on_settings_changed` | 選んだ画面（80桁／40桁／文字コードに合うカスタムプロファイル。選ばなければ変えない） | **変えない**（接続方式のまま） | プロファイルの値を文字コードに合わせたもの | 選んだ言語（CP437・PETSCII なら en） |

- 接続したユーザーが見る流れ:
  1. 接続方式の選択（ASCII の大文字だけで40桁以内。C64 でも読める）。選べるのは、1: 日本語 ShiftJIS／2: 日本語 UTF-8／3: 英語 UTF-8／4: 英語 CP437／5: Commodore 64。Enter だけなら既定（`default_profile` の文字コードと `locale.language` から決まる）
  2. ウェルカム（選んだ文字コードと言語で表示）
  3. L（ログイン）/ R（登録）/ G（ゲスト）。以前あった言語選択（E/J/U）は無くした
  4. ログインしたら、保存してある言語と画面の配置に切り替わる。**文字コードは接続方式のまま**（別の端末からつないでも設定を変えずに使える。Issue #269）
- 登録すると、接続方式の文字コード・言語・プロファイル名を保存する。
- DB の `users.encoding` は、ログイン時には使わない（設定画面で保存はされる）。
- C64 の既定（大文字・グラフィック）モードでは、英字が大文字として送られる。ユーザー名は大文字小文字を区別しないが、パスワードは区別するため、小文字を含むパスワードは C64 からは入力できない（小文字モード 0x0E への対応は未実施）。
- Telnet の TTYPE / NAWS ネゴシエーションは**実装されていない**（定数だけある）。端末の自動判定はしていない。

## 3. 組み込みプロファイル

| 名前 | 幅×高 | cjk_width | encoding | output_mode | 備考 |
|---|---|---|---|---|---|
| standard | 80×24 | 2 | ShiftJIS | Ansi | |
| standard_utf8 | 80×24 | 2 | UTF-8 | Ansi | |
| dos | 80×25 | 1 | CP437 | Ansi | 日本語は `?` になる |
| c64 | 40×25 | 1 | PETSCII | PetsciiCtrl | ANSI の色やカーソル移動を PETSCII の制御コードに変換する。旧 `c64_petscii` / `c64_ansi` / `petscii` はこれの別名 |
| 40col_sjis | 40×25 | 2 | ShiftJIS | Ansi | |
| 40col_utf8 | 40×25 | 2 | UTF-8 | Ansi | |

- テンプレートの組は、幅だけで決まる（80以上なら `templates/80`、それ未満なら `templates/40`）。`template_dir` は**使われていない**（設定ファイルとの互換のために読み込みだけしている）。
- `ansi_enabled` を使っているのは、Lua スクリプトの `has_ansi` だけ。
- カスタムプロファイル（`[[terminal.profiles]]`）は、名前で探すときに組み込みより先に使われる（接続時の `default_profile`、ログイン時、設定画面）。設定画面では、接続の文字コードに合うものだけが選択肢に出る。

### 廃止したプリセット

| 名前 | 何だったか | 今の扱い |
|---|---|---|
| `c64_petscii` / `c64_ansi` / `petscii` | C64 向けの方式違い（制御コード方式、PETSCII と ANSI の混在） | `c64` の別名。保存してあるユーザーは `c64` として動く |
| `jterm40` | ユーザーが Commodore 64 で自作した端末のための特殊モード。ShiftJIS で、ASCII も漢字も同じ幅の1マスに表示する（cjk_width = 1、40×25） | 組み込みから外した。`config.toml.sample` の `[[terminal.profiles]]` の記入例を有効にすると、以前と同じに使える（ゴールデン `b_login__jterm40_custom` で、ログイン後の出力が以前と同じバイト列であることを確認済み）。定義しない場合は `40col_sjis`（漢字を2桁と数える）になり、ログに警告を出す |

## 4. 幅の計算

`src/terminal/width.rs` が唯一の実装。`TerminalProfile`、テンプレートの `{{pad}}`、`ScreenContext::word_wrap` はこれを使う。

- `cjk_width == 1`: すべての文字を1桁と数える
- それ以外: ASCII は1桁、ASCII 以外はすべて2桁と数える（半角カナ、é、罫線、絵文字も2桁）

## 5. 出力と入力

- **出力**: `src/server/wire.rs` の `to_wire` が唯一の経路。改行の正規化 → 出力モードの処理 → 文字コードの変換、の順に行う。
  - テンプレートと投稿の中の `^[` は、呼び出し側が `convert_caret_escape` で ESC に変換する。
- **入力**: `LineBuffer` が生のバイトをためておき、行が確定したらまとめてデコードする。そのため、マルチバイト文字が read の境目で割れても正しくデコードされる。
  - エコーの書き方は経路で違う（Q11）。ScreenContext はエコーモード（Normal / Password / Masked）で絞り込む（`write_screen_echo`）。SessionHandler は絞り込まずにそのまま書く。
  - 入力のエスケープシーケンス（矢印キーなど）には対応していない。ESC を捨てるので、`[A` が入力に混ざる（Issue #133）。

## 6. 既知の癖（Quirk）

| ID | 内容 |
|---|---|
| ~~Q1~~ | （解決済み）ログイン時と設定の保存時は、プロファイルの output_mode を適用する。ただし output_mode は、回線上の文字コードに合わせて調整する（PETSCII なら PetsciiCtrl、それ以外で PetsciiCtrl なら Ansi） |
| ~~Q2~~ | （解決済み）カスタムプロファイルを、接続時・ログイン時・設定画面で使う。未知の名前は standard になる |
| Q3 | ウェルカム、メインメニュー、ヘルプは、プロファイルに関係なく cjk_width=2 として描画される |
| ~~Q4~~ | （解決済み）PETSCII のエンコーダは PETSCII の制御コードをそのまま送り、CRLF を CR 1つにする。PETSCII に無い記号（`| _ \ ^ ~ { }` など）は近い字形に置き換える |
| ~~Q5~~ | （解決済み）PetsciiCtrl は複合の SGR（`1;33` など）、明るい色、背景色（色＋反転で表示）、カーソル移動の回数と位置指定を扱う |
| Q6 | 入力の ESC は捨てられる（Issue #133）。0x14（C64 の DEL）は、PETSCII のときに BS として扱い、消去のエコーも 0x14 で返すようにした（解決済み） |
| Q7 | ScreenContext の入力は IAC を取り除かない（IAC WILL SGA の 0x03 が Ctrl+C として扱われる） |
| ~~Q8~~ | （解決済み）接続直後に接続方式を選ぶので、ログイン前から選んだ文字コードで送る |
| Q9 | BS の消去幅はバイト数で決まる（jterm40 で漢字を消すと2桁、UTF-8 の半角カナでも2桁消える） |
| Q10 | `send_raw`（「続きは ENTER」の表示）だけ、改行を正規化しない |
| Q11 | エコーの書き方が、SessionHandler と ScreenContext で違う（5. 参照） |

## 7. 挙動が変わっていないことの確かめ方

- `tests/golden_terminal.rs` と `tests/golden_units.rs` は、サーバーが送る**生のバイト列**を `tests/golden/*.txt` と比較する（ゴールデンテスト）。
- 挙動を変えないつもりの変更では、`tests/golden/` に差分が出てはならない。
- 挙動を**意図して**変えるときは、`UPDATE_GOLDEN=1 cargo test --test golden_terminal --test golden_units` で更新する。その差分を PR で「意図した変更」として見せる。

## 8. 今後の予定（決定済み、未実装）

plan.md の決定事項 D1〜D3。一般的なパソコン通信ホストの動きに合わせる。

- **D1 プリセットの整理**:
  - ~~C64 系の3つは、`c64`（PETSCII の制御コード方式）1つにまとめて直す。~~（実施済み）
  - ~~`jterm40` は組み込みから外し、`config.toml.sample` のカスタムプロファイルの記入例として残す。~~（実施済み）
  - ~~旧名は別名として読み替える。~~（実施済み）
- ~~**D2 接続時の選択を優先する**~~（実施済み）: 文字コードは、毎回の接続で選んだものを使う。言語・幅は、保存してある設定を使う。ただし CP437 と PETSCII では英語にする。
- **D3 選ぶ場所を2つにする**:
  - ~~接続直後（ログイン前）: 「接続方式」を5択から選ぶ。~~（実施済み）
  - ログイン後の設定画面: 言語・画面（80桁／40桁／カスタム）・ページング（実施済み）。色（ANSI）の有無は未実施（DB に項目を足す必要がある）。
