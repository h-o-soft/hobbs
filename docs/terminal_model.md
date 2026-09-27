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
| 接続時 | `on_connect` | `terminal.default_profile`（組み込みのみ） | 変えない（新しいセッションは ShiftJIS）**Q8** | 接続時のプロファイルの値 | `locale.language` |
| 言語選択（登録・ゲスト） | `on_language_selected` | 変えない | E/1: UTF-8、J/2: ShiftJIS、U/3: UTF-8、それ以外: UTF-8 | 変えない | E/1・それ以外: en、J/2・U/3: ja |
| ログイン | `on_login` | `users.terminal` を `from_name` で引く **Q2** | `users.encoding` | プロファイルの値 | `users.language` |
| 設定画面で保存 | `on_settings_changed` | 選んだプロファイル（`from_name`。選ばなければ変えない）**Q2** | 選んだプロファイルの encoding（選ばなければ今の値） | プロファイルの値 | 選んだ言語 |

- DB では `users.terminal` と `users.encoding` を別々に保存している。そのため「standard なのに UTF-8」のような食い違いが起こりうる。
- 接続したユーザーが見る流れ:
  1. ウェルカム（ASCII、ShiftJIS）
  2. L（ログイン）/ R（登録）/ G（ゲスト）
  3. R と G のときは、言語選択（E/J/U。CP437 と PETSCII は選べない）
  4. ログインしたら、保存してある設定に切り替わる
- Telnet の TTYPE / NAWS ネゴシエーションは**実装されていない**（定数だけある）。端末の自動判定はしていない。

## 3. 組み込みプロファイル

| 名前 | 幅×高 | cjk_width | encoding | output_mode | 備考 |
|---|---|---|---|---|---|
| standard | 80×24 | 2 | ShiftJIS | Ansi | |
| standard_utf8 | 80×24 | 2 | UTF-8 | Ansi | |
| dos | 80×25 | 1 | CP437 | Ansi | 日本語は `?` になる |
| c64 | 40×25 | 1 | PETSCII | PetsciiCtrl | ANSI の色やカーソル移動を PETSCII の制御コードに変換する。旧 `c64_petscii` / `c64_ansi` / `petscii` はこれの別名 |
| 40col_sjis | 40×25 | 2 | ShiftJIS | Ansi | |
| jterm40 | 40×25 | **1** | ShiftJIS | Ansi | C64 用の自作端末向けの特殊モード。ASCII も漢字も同じ幅の1マスに表示する端末を想定している |
| 40col_utf8 | 40×25 | 2 | UTF-8 | Ansi | |

- テンプレートの組は、幅だけで決まる（80以上なら `templates/80`、それ未満なら `templates/40`）。`template_dir` は**使われていない**（設定ファイルとの互換のために読み込みだけしている）。
- `ansi_enabled` を使っているのは、Lua スクリプトの `has_ansi` だけ。
- カスタムプロファイル（`[[terminal.profiles]]`）は設定画面の一覧に出る。ただし、ログイン時と設定の保存時は `from_name` で引くため、**幅や高さは効かず、効くのは encoding だけ**（Q2）。

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
| Q2 | ログイン時と設定の保存時は、カスタムプロファイルを見ない。未知の名前は standard になる |
| Q3 | ウェルカム、メインメニュー、ヘルプは、プロファイルに関係なく cjk_width=2 として描画される |
| ~~Q4~~ | （解決済み）PETSCII のエンコーダは PETSCII の制御コードをそのまま送り、CRLF を CR 1つにする。PETSCII に無い記号（`| _ \ ^ ~ { }` など）は近い字形に置き換える |
| ~~Q5~~ | （解決済み）PetsciiCtrl は複合の SGR（`1;33` など）、明るい色、背景色（色＋反転で表示）、カーソル移動の回数と位置指定を扱う |
| Q6 | 入力の ESC は捨てられる（Issue #133）。0x14（C64 の DEL）は、PETSCII のときに BS として扱い、消去のエコーも 0x14 で返すようにした（解決済み） |
| Q7 | ScreenContext の入力は IAC を取り除かない（IAC WILL SGA の 0x03 が Ctrl+C として扱われる） |
| Q8 | 接続時は、プロファイルの encoding を使わず ShiftJIS のまま |
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
  - `jterm40` は組み込みから外し、`config.toml.sample` のカスタムプロファイルの記入例として残す。
  - 旧名は別名として読み替える。
- **D2 接続時の選択を優先する**: 文字コードは、毎回の接続で選んだものを使う。言語・幅・ページングは、保存してある設定を使う。ただし CP437 と PETSCII では英語にする。
- **D3 選ぶ場所を2つにする**:
  - 接続直後（ログイン前）: 「接続方式」を5択から選ぶ（日本語 SJIS／日本語 UTF-8／英語 UTF-8／英語 CP437／C64）。
  - ログイン後の設定画面: 言語・幅・色・ページング。
