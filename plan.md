# 文字コード・端末まわりの整理（挙動を変えないリファクタリング）設計

## Purpose

### 目的
文字コード（encoding）・端末プロファイル（profile）・出力モード（output mode）・言語（language）の4つは、今は**別々の場所で、別々のタイミングで**決まっている。重複した処理もあちこちにある。これを**挙動を変えずに**整理し、この先の修正（C64 系の不具合修正、ログイン前の端末切り替え、プロファイルの整理）を安全に小さく入れられる土台を作る。

### 背景
- ユーザーの評価は「あっちこっち混乱している」「ユーザーが何を選べばいいか分からない」「設定がユーザーごとに保存されるので、別環境からつないだときに変える手段がない（ログイン前に変えられるべき）」。
- jterm40 の正体は、**ユーザーが Commodore 64 で自作した端末の特殊モード**（ShiftJIS で、ASCII も漢字も同じ幅の1マスに表示する）。これを後から足しつつ普通のモードも残したことが、複雑化の一因。
- 直前の調査で見つけた「おかしい所」7点と、重複（出力処理3か所、幅の計算4か所）が、この設計の軸。

### やらないこと（Non-goals）
- **挙動の変更**。バグを直すことや、ログイン前の端末切り替えの実装はしない。見つけたおかしな挙動は「現状の癖（Quirk）」として**あえてそのまま残し**、テストで固定する。直すのは後の別フェーズ（ユーザーの承認が要る）。
- DB スキーマの変更。
- プロファイルを減らす・名前を変えること（判断材料だけ出す）。
- Web UI 側（Web には文字コードや端末の概念がない。`src/web` に encoding/terminal の参照は無い）。

## Observed evidence

すべて main（78a7594）時点のコード。行番号は目安。

### E1. 4つの設定が決まる場所とタイミング
| 設定 | 保持している場所 | 決まるタイミング |
|---|---|---|
| encoding | `TelnetSession.encoding`（src/server/session.rs）と `SessionHandler.line_buffer` の encoding と `ScreenContext.line_buffer` の encoding（**3か所**） | 接続時は既定の ShiftJIS（`CharacterEncoding::default()`）。言語選択で E/J/U に応じて設定（session_handler.rs:355-403）。ログイン時に `user.encoding`（:516-518）。設定変更時にプロファイルの encoding（:763-773、profile.rs:443-444） |
| output_mode | `TelnetSession.output_mode` | **接続時の1回だけ**。`session.set_output_mode(self.profile.output_mode)`（session_handler.rs:146）。値は `config.terminal.default_profile` から来る。これ以外に set_output_mode を呼ぶ所は無い（grep で確認） |
| profile | `SessionHandler.profile`、それを clone した `ScreenContext.profile` | 接続時は `TerminalProfile::from_name(default_profile)`（:76）。ログイン時と設定変更時は `set_terminal_profile(name)`（:418-424）。ここで使うのは `from_name` で、カスタムプロファイルを**見ない** |
| language | `SessionHandler.i18n`、それを渡した `ScreenContext.i18n` | 接続時は `config.locale.language`。言語選択時、ログイン時（`user.language`）、設定変更時 |

- DB では `users.terminal`（既定 `'standard'`、migrations/sqlite/0001）と `users.encoding`（既定 `'shiftjis'`、0002）を**別々に**持っている（src/db/user.rs:108-111, 155-157, 177）。
- 登録時は `session.encoding()` と `i18n.locale()` を保存する。terminal は常に "standard"（session_handler.rs:677-680）。
- 設定画面ではプロファイル一覧に組み込みとカスタムが両方出る（`from_name_with_custom`、profile.rs:382-446）。保存されるのは terminal 名と、そのプロファイルの encoding。

### E2. 出力処理の重複（3か所）
1. `SessionHandler::send`（session_handler.rs:991-1000）: CRLF の正規化 → `process_output_mode` → `encode_for_client` → write。
2. `ScreenContext::send`（src/app/screens/common.rs:158-215）: CRLF の正規化 → 自動ページングがあれば行ごとに分割 → 各行を `process_output_mode` → `encode_for_client` → write。
3. `ScreenContext::send_raw`（common.rs:259-265）: **CRLF の正規化をしない** → `process_output_mode` → `encode_for_client`。「続きは ENTER」の表示に使う。

関連する処理:
- `convert_caret_escape`（`^[` を ESC に変換）は呼び出し側がそれぞれ個別に呼んでいる（session_handler.rs:432, common.rs:792, board.rs, mail.rs）。
- 入力のエコー書き込みも重複している。`ScreenContext` の `read_line`（:447-535）、`read_line_nonblocking`（:539-）、`finish_line_reading`（:605-）にほぼ同じ echo 分岐（Normal / Password / Masked）が3回あり、`SessionHandler::process_input_bytes` にも別に1つある。エコーは `encode_for_client` を通らず、生のバイトをそのまま返す。
- IAC の除去は `SessionHandler::read_line` だけが行っている（TelnetParser.parse、:1055）。ScreenContext の読み込み系は IAC を除去しない。

### E3. 幅の計算の重複（4か所）
すべて「ASCII は1、それ以外は cjk_width（1 または 2）」という同じ規則:
1. `TerminalProfile::display_width` と `truncate_to_width`（src/terminal/profile.rs:231-237, 266-286）
2. `template::display_width` と `truncate_to_width`（src/template/mod.rs:47-73）。テンプレートの `{{pad}}` が使う（renderer.rs:195-207）
3. `ScreenContext::word_wrap`（common.rs:302-420）。`c.is_ascii()` の判定を自前で持っている
4. `char_wrap_into`（common.rs:423-）

さらに、入力の BS エコー `LineBuffer::display_width_of_deleted`（src/server/input.rs:192-202）は**バイト数**で幅を決めている（2バイト以上なら2桁）。

### E4. 見つかった癖・不具合（今回は直さず、テストで固定する）
- Q1: output_mode がログイン後も設定変更後も変わらない（E1）。
- Q2: ログイン時と設定変更時の `set_terminal_profile` が `from_name` を使うので、カスタムプロファイルの幅と高さが効かない。未知の名前は standard 扱い（profile.rs:377）。
- Q3: `SessionHandler::create_context`（session_handler.rs:959-972）が `set_cjk_width` を呼ばない。そのため TemplateContext の既定値 2（template/mod.rs:247）で描画される（welcome、main menu、help）。ScreenContext 側は設定している（common.rs:141）。
- Q4: PETSCII のエンコーダ `unicode_to_petscii_byte`（encoding.rs:727-777）は `\r` と `\n` しか制御文字を持たない。ESC や PetsciiCtrl の制御文字は `?` になる。`\n` が 0x0D になるので、CRLF は 0x0D 0x0D になる。
- Q5: PetsciiCtrl は SGR のパラメータ全体を u8 として解釈するため、`1;33` のような複合指定は解釈できない（encoding.rs:927-）。
- Q6: 入力で ESC を捨てる（input.rs:323-327、`// TODO: Handle ANSI escape sequences`）。0x14（C64 の DEL）も捨てる（:334-338）。
- Q7: ScreenContext の入力は IAC を除去しない（E2）。
- Q8: ログイン前は `default_profile` の encoding を使わず、必ず ShiftJIS。
- Q9: BS の消去幅がバイト数で決まる（E3）。jterm40 で漢字を消すと2桁、UTF-8 の半角カナを消すと2桁消える。
- Q10: `send_raw` だけ CRLF を正規化しない（E2）。

### E5. 使われていないコード
- `src/screen`（AnsiScreen / PlainScreen）。`SessionHandler.screen` として生成しているだけで、出力には使っていない（session_handler.rs:50, 78, 422）。
- Cargo.toml の `petscii = "0.2"`（ソースから参照が無い）。
- `TerminalProfile.template_dir`（テンプレートの選択は幅だけで決まる。template/loader.rs:38-41）。
- `create_session_handler_with_profile`（app/mod.rs:122、呼び出し元なし）。
- `*_strict` / `*_detailed` 系の関数（本番コードからは使われていない。lib.rs と server/mod.rs で再エクスポートしているだけ）。

### E6. テストの現状（19ファイル）
- 文字コード系の E2E は `tests/e2e_encoding.rs`（10件）、`tests/encoding_e2e.rs`（14件）、`tests/e2e_profile_settings.rs`（4件）、`tests/e2e_login_settings.rs`（4件）。**ShiftJIS と UTF-8 しか扱っていない。**
- `tests/` に `c64`・`petscii`・`dos`・`cp437`・`jterm40`・`40col` の E2E は**1件も無い**（grep で確認）。
- テストクライアント `tests/common/mod.rs` の `recv` は文字列に復号してから返すので、**送信されたバイト列そのものは検証していない**。`send_raw` はある。
- 単体テストは多い（encoding.rs に106件、profile.rs に43件、input.rs に39件）。ただし変換関数の単体テストで、**出力経路全体を通したテストは無い**（Q4 が見逃された理由）。
- 今回は `cargo test` を実行していない（調査は読むだけの約束）。フェーズ0で基準値を取る。

## Change scope

### 目標の構造（案）
```
TerminalProfile   … プリセット（幅・高さ・cjk_width・ANSI の可否・既定 encoding・既定 output_mode）。変更なし（不要フィールドの扱いは Open decisions）
TerminalSettings  … 【新規】そのセッションで実際に効いている値の唯一の置き場所
                     { profile, encoding, output_mode, language }
                     （profile は「プリセット名＋幅・高さ・cjk_width」を持つ。後のフェーズ F3 で
                      プリセットを減らし、幅などを個別に設定できるようにするため、
                      プリセット名だけで値を引き直す作りにはしない）
settings::resolve … 【新規】「いつ・何から」各値を決めるかを1か所に集めた純粋関数群
                     （接続時 / 言語選択 / ログイン / 設定変更）。癖 Q1・Q2・Q8 はここに明示して残す
terminal::width   … 【新規】幅の計算の唯一の実装（char_width / display_width / truncate_to_width）
server::wire      … 【新規】出力の唯一の経路（text → CRLF → output_mode → encode → bytes）
                     と、エコー書き込みの唯一の経路
```

### ファイルごとの変更
- `src/terminal/width.rs`（新規）: 幅の関数。`TerminalProfile::display_width` と `truncate_to_width`、`template::display_width` と `truncate_to_width`、`word_wrap` と `char_wrap_into` の文字幅判定は、すべてここに委譲する。規則は現状どおり「ASCII は1、それ以外は cjk_width」。
- `src/server/wire.rs`（新規、または encoding.rs 内の新しい関数）: フェーズ2では `fn to_wire(text: &str, encoding: CharacterEncoding, output_mode: OutputMode, newline: NewlinePolicy) -> Vec<u8>` とし、今ある型だけに依存させる（フェーズ2の PR を単独でビルド・検証できるようにするため）。フェーズ3で `TerminalSettings` を入れたあと、呼び出し側が `settings.encoding` と `settings.output_mode` を渡す形に変える（関数の中身は変えない）。`NewlinePolicy::{Normalize, AsIs}` は Q10 を残すためのもの。`SessionHandler::send`、`ScreenContext::send`（行ごとの分割とページングはそのまま残し、1行ごとの処理だけ to_wire にする）、`send_raw` はこれを呼ぶ。エコーは `fn write_echo(session, echo: &[u8], mode: EchoMode)` に一本化し、4か所の分岐を置き換える（生のバイトを返す挙動は変えない）。
- `src/terminal/settings.rs`（新規）: `TerminalSettings` 構造体と `resolve` 関数群。
  - `on_connect(config)`: profile=default_profile、encoding=ShiftJIS（Q8）、output_mode=profile.output_mode、language=config.locale.language。
  - `on_language_selected(cur, choice)`: E/J/U/その他 → (language, encoding)。profile と output_mode はそのまま。
  - `on_login(cur, user)`: encoding=user.encoding、language=user.language、profile=`from_name(user.terminal)`（Q2）。**output_mode は変えない**（Q1）。
  - `on_settings_changed(cur, language, encoding, terminal)`: on_login と同じ規則（Q1・Q2）。
- `src/server/session.rs`: `TelnetSession` の `encoding` と `output_mode` を `TerminalSettings` に置き換える（または TerminalSettings を持たせ、今の getter と setter は委譲にして互換を保つ）。
- `src/app/session_handler.rs`: `profile`・`i18n`・`line_buffer` の encoding を個別に書き換えている4か所（:146、:355-403、:516-537、:763-773）を、`apply_settings(session, new_settings)` の1関数に置き換える。この関数が session・line_buffer・i18n（language → I18nManager で取得）・profile を**まとめて**更新する。`create_screen_context` は settings から作る。`create_context` が cjk を設定しない癖（Q3）は、明示的にコメントし、既定値 2 のままにする。
- `src/app/screens/common.rs`: send 系は wire に委譲する。幅は terminal::width に委譲する。エコーは write_echo を使う。
- `src/app/screens/profile.rs`: 返す `ScreenResult::SettingsChanged` はそのまま。受け取る側が `on_settings_changed` を通すようにする。
- 使われていないコードの削除（フェーズ4）: `src/screen`、`SessionHandler.screen`、`petscii` 依存、`create_session_handler_with_profile`。`*_strict` と `*_detailed` は Open decisions。
- ドキュメントの同期（フェーズ4）: CLAUDE.md のプロファイル表に jterm40 を追加する。docs/05_protocol.md のプロファイル一覧（6種 → 9種）と「NAWS/TTYPE 対応」の記述を実態に合わせる（「未実装」と書く）。config.rs:327 のコメント、migrations のコメントは触らない（migration は変更不可）。
- `tests/`: フェーズ0の特性テスト（後述）。

## Implementation steps

各フェーズを1つの PR にする（ブランチ `refactor-terminal-<n>-...`）。**各フェーズは、それより前のフェーズで入った型と関数だけに依存させる**（依存の順序: フェーズ0 テスト → 1 `terminal::width` → 2 `to_wire` と `write_echo`（既存の型だけを使う）→ 3 `TerminalSettings` と `resolve` → 4 削除）。**前のフェーズのテストがすべて緑で、ゴールデンが1バイトも変わっていないこと**を、次に進む条件にする。

### フェーズ0: 基準値と特性テスト（本番コードは変えない）
1. main で `cargo test` を全部実行し、件数と結果を PR に記録する（失敗が既にあれば、先に報告して止まる）。
2. `tests/common/mod.rs` に `recv_raw_timeout() -> Vec<u8>` を追加する（今の recv の生バイト版）。
3. **ゴールデン（送信バイト列のスナップショット）テスト** `tests/golden_terminal.rs` を追加する。
   - シナリオ × 条件で、サーバーが送ったバイト列をそのまま `tests/golden/<scenario>__<cond>.bin` と比較する。
   - `UPDATE_GOLDEN=1` のときは上書き保存する（自前の小さなヘルパー。新しい依存は足さない）。
   - 日時は `\d{4}/\d{2}/\d{2} \d{2}:\d{2}(:\d{2})?` を固定文字列に置き換えてから比較する。数字・`/`・`:` は PETSCII・CP437・SJIS・UTF-8 のどれでも ASCII と同じバイトになる。
   - 条件軸 A（接続時）: `default_profile` を9種それぞれにしたサーバーで、「ウェルカム画面」と「不正入力 → 再プロンプト」。
   - 条件軸 B（ログイン後）: `users.terminal` × `users.encoding` を代表的な組み合わせにしたユーザー（9プロファイル × それぞれの既定 encoding、それに食い違いの例として standard + utf8 と c64 + shiftjis）でログインし、「ログイン直後」「メインメニュー」「日本語タイトルの掲示板一覧（`{{pad}}` を含む）」「ヘルプ」。
   - 条件軸 C（設定変更）: standard でログイン → 設定画面で各プロファイルを選ぶ → メインメニュー。
   - 条件軸 D（ゲストと登録）: 言語選択 E/J/U/不正値のそれぞれ → メインメニュー。
   - 条件軸 E（入力エコー）: 各 encoding で「日本語2文字を打つ → BS → Enter」「PETSCII の 0x14」「ESC [ A」「IAC を含む入力」を、SessionHandler 側（ウェルカム）と ScreenContext 側（掲示板の入力）の両方で送り、エコーとして返ってくるバイト列。
   - 条件軸 E2（エコーモード）: `EchoMode::Normal` / `Password` / `Masked(c)` のそれぞれについて、「英数字・日本語を打つ → BS → Enter」を送ったときに返るバイト列を、置き換えの対象になる**4経路すべて**で固定する（`SessionHandler::process_input_bytes`、`ScreenContext::read_line`、`read_line_nonblocking`、`finish_line_reading`）。Password と Masked では、**入力した文字のバイトが1バイトもエコーに含まれない**ことを、ゴールデンとは別の明示的な assert でも確かめる（ログインのパスワード入力、登録時のパスワード入力を含む）。E2E で到達しにくい経路は、`write_echo` の置き換え前後で同じ結果になる単体テスト（LineBuffer と echo 分岐を直接呼ぶ）で補う。
   - 条件軸 F（ページング）: auto_paging が有効で、行数が閾値を超える一覧 → 「続きは」を含むバイト列。
4. 単体の特性テスト:
   - 幅の表テスト: 文字の種類（ASCII、漢字、半角カナ、é、─、絵文字、結合文字）× cjk_width 1/2。**4か所の実装がすべて同じ値を返す**ことを確認する。word_wrap と char_wrap の代表ケースも含める。
   - resolve の表テスト（フェーズ3で使う期待値表を、ここで E2E の観測から作る）。
5. ここで作ったゴールデンは「今の挙動そのもの」。**癖（Q1〜Q10）を含んだまま**コミットする。PR 本文に「このゴールデンは正しい挙動ではなく、現状の挙動」と明記する。

#### フェーズ0の実施結果（2026-09-27）
- 基準値（main 78a7594、`cargo test --no-fail-fast`、debug ビルド）: 成功 1562、失敗 33、無視 3。失敗はすべて #325 とは無関係な既存の問題なので、別の Issue にした。
  - Web API の統合テスト30件: `ConnectInfo` が無いため 500 になる（#326）。
  - Telnet E2E の3件: debug ビルドでは Argon2 が遅く、固定時間の待ちでは間に合わない（#327）。
  - **このため、各フェーズの合格条件は「失敗するテストの集合が基準値と同じ（増えない）」と「ゴールデンが1バイトも変わらない」の2つとする。**
- 追加したもの:
  - `tests/golden_terminal.rs`（E2E。条件軸 A〜F、ゴールデン44件）
  - `tests/golden_units.rs`（LineBuffer と ScreenContext のエコーを 3モード×4文字コードで記録、幅の計算の4実装が一致することの確認、出力の変換チェーン。ゴールデン5件）
  - `tests/golden_support/`（エスケープ表記、日時の正規化、比較と `UPDATE_GOLDEN`）
- E2E の待ち方:
  - 基本は、プロンプトの末尾（`": "`、`"> "`、`"? "`）が来てから、300ms 何も来なければそのステップの終わりとする。
  - プロンプトに区切りが無い画面（英語のウェルカムのプロンプトなど）は、出力が始まってから 1.5 秒何も来なければ終わりとする。
  - パスワードのハッシュを伴うステップは、プロンプトの末尾が来るまで待つ（最大60秒）。
  - UPDATE なしで2回続けて全件一致することを確認した。
- 条件軸 E のうち、画面を抜けてしまう入力（Ctrl+C、IAC を含む入力、CR LF など）は、ScreenContext 側では `golden_units` で確かめる。E2E ではメインメニュー（SessionHandler 側）でだけ送る。
- resolve の表テスト（手順4）は、resolve がまだ無いので、フェーズ3の PR で B・C・D のゴールデンから期待値を起こして作る。

### フェーズ1: 幅の計算を1か所にまとめる
1. `terminal::width` を追加し、4か所を委譲に置き換える。
2. フェーズ0の幅の表テストとゴールデンが変わらないことを確認する。
3. `LineBuffer::display_width_of_deleted`（Q9）は**変えない**（バイト数ベースのまま）。後のフェーズで width を使うように直す場所として、コメントで印を付ける。

### フェーズ2: 出力経路とエコーを1か所にまとめる
1. `to_wire(text, encoding, output_mode, newline)` を追加し、`SessionHandler::send`、`ScreenContext::send`、`send_raw` を置き換える（Q10 は `NewlinePolicy::AsIs` で残す）。引数は今の `session.encoding()` と `session.output_mode()` をそのまま渡す。フェーズ3で導入する型には依存しない。
2. `write_echo` を追加し、4か所のエコー分岐を置き換える。
3. 入力の読み込み（IAC を除去する / しない）は**この段階では統合しない**（Q7 を残すため）。二つの読み込み経路の違いをコメントで明示するにとどめる。
4. ゴールデンの A〜F がすべて一致することを確認する。

### フェーズ3: TerminalSettings と resolve を導入する
1. `TerminalSettings` と `resolve::*` を追加し、フェーズ0の表テストを通す。
2. `TelnetSession` に TerminalSettings を持たせる。`to_wire` の呼び出し側は、settings から encoding と output_mode を取り出して渡すように変える（`to_wire` 自体のシグネチャは変えない）。今の `encoding()` と `output_mode()` などは委譲にして互換を保つ。
3. `SessionHandler` の4か所を `apply_settings` にする。`line_buffer` の encoding、`profile`、`i18n` はすべて settings から導く。
4. ScreenContext はセッションごとに settings から作る（今の clone 渡しと同じ意味）。
5. Q1・Q2・Q3・Q8 は resolve の中で `// QUIRK(Qn): 現状維持。修正はフェーズF1` とコメントし、表テストで固定する。
6. ゴールデンが全部一致することを確認する。

### フェーズ4: 使われていないコードの削除とドキュメントの同期
1. E5 のうち、D4 で削除すると決めたものを削除する。
2. CLAUDE.md、docs/05_protocol.md、docs/operation_guide.md のプロファイル表と、NAWS/TTYPE の記述を実態に合わせる。
3. 設計メモ `docs/terminal_model.md` を追加する（4つの設定がどこで決まるかの表、Quirk の一覧、決定事項 D1〜D3 の要約）。

### （参考）この後の、挙動を変えるフェーズ（今回の範囲外。個別に承認をもらう）
- F1 不具合修正:
  - Q1（output_mode を反映）、Q2（カスタムプロファイルを反映）
  - Q4（PETSCII で ESC と制御文字を通し、CR の重複をやめる）、Q5
  - Q6（0x14 を BS として扱う）、Q7（全経路で IAC を除去する）、Q9（BS の幅を width に合わせる）
  - それぞれ、ゴールデンの差分を「意図した変更」として PR で見せる。
- F2 接続ごとの「接続方式」選択（決定事項 D2・D3）。変更箇所:
  - 接続直後、ウェルカム画面より前に ASCII だけの選択画面を出す。今の `show_language_selection`（session_handler.rs:355-403）を拡張して置き換え、L/R/G すべてに共通の最初の画面にする。
  - `resolve::on_connect_choice`（新規）と、`resolve::on_login` の優先規則（D2）。
  - `templates/{80,40}/welcome.txt` の案内。
  - フェーズ3で `resolve` と `apply_settings` の入口が1つになっているので、「接続時に選んだもの」を state として持つだけで済む形にしておく。**Issue #269（PETSCII からの復旧）もこれで解決する。**
- F3 プリセットの整理と設定画面の再設計（D1・D3）。廃止するプリセットの名前は、互換のための別名として残す（D1）。
- F4 入力のエスケープシーケンス（Issue #133）。TTYPE/NAWS による画面サイズの自動取得と、ANSI の自動判定（任意）。

## Edge cases and test plan

- **挙動が変わっていないことの確認（最重要）**:
  1. フェーズ0で作るゴールデン（送信バイト列の完全一致）が主な判定基準。文字列に復号してからの比較では、エンコード段階の違い（Q4 のような）を見逃すので、**生のバイトで比べる**。
  2. フェーズ1〜4の各 PR では `UPDATE_GOLDEN` を**使わない**。CI かレビューで `tests/golden/` に差分が無いことを確認する。差分が出たら、その PR は失敗扱い。
  3. 既存の全テスト（フェーズ0の基準値）の件数と結果が同じであること。
  4. 手動のスモークテスト: dunedin で `telnet dunedin 2323`（SJIS と UTF-8）。可能なら、ユーザーの C64 自作端末で jterm40 を確認する。実機での確認はユーザーに依頼する。
- **既存テスト19ファイルで足りるか**: 足りない。SJIS と UTF-8 以外の E2E がゼロ、生のバイトを検証する E2E もゼロ、ログイン前と `default_profile` の組み合わせも未検証。フェーズ0の追加が前提条件。
- **境界ケース**:
  - ShiftJIS の2バイト文字が read の境目で割れる場合（LineBuffer が生バイトをためておく今の挙動を維持。既存の単体テストあり）。
  - UTF-8 の4バイト文字（絵文字）、結合文字、半角カナ（SJIS では1バイト、UTF-8 では3バイト）。
  - 自動ページングで、最後の行に改行が無いとき（プロンプト）。
  - ゲストが言語選択で不正値を入れたとき（E 扱い）。
  - DB 上で terminal と encoding が食い違っているユーザー。
  - config に未知の default_profile やカスタムプロファイル名があるとき。
- **失敗時・ロールバック**: 各フェーズは独立した PR で、DB も設定ファイルも変えない。問題があれば、その PR を revert すれば前のフェーズの状態に戻る。フェーズ0のテストは残る。
- **性能**: `to_wire` は今と同じ処理を1か所に集めるだけで、行ごとの処理回数は変わらない。

## Open decisions

ユーザーの回答は「もう記憶にないので決められない。一般的なパソコン通信ホストと同じ動きにしてほしい。一般的で使いやすいとされているものに寄せてほしい」（2026-09-27）。
これを受けて、以下は**決定済み**とする。各項目の「理由」は、何が一般的だからそう決めたかを1行で書いたもの。
参照した「一般的な例」:
- 国内のパソコン通信ホスト: 接続時に漢字コードを選ばせる形が多い。
- 海外の現役 BBS ソフト（Synchronet、Mystic など）: 接続ごとに端末（ANSI の可否、CP437/UTF-8、PETSCII、画面サイズ）を判定するか、ユーザーに聞く。保存したユーザー設定は既定値として使う。
- C64 向けの BBS: PETSCII の文字と PETSCII の制御コードで色やカーソルを扱う（CCGMS などの C64 端末ソフトが前提としている形）。

### D1. 端末プリセットの整理（F3 で実施。今回のリファクタリングでは全部残す）
| 今の名前 | 決定 | 理由 |
|---|---|---|
| standard（SJIS 80×24） | 残す | 国内ホストの標準形（PC-98 などの SJIS 端末）。 |
| standard_utf8（UTF-8 80×24） | 残す | 今の端末エミュレータ（Mac、Linux、Windows Terminal）の既定は UTF-8。 |
| dos（CP437 80×25） | 残す | 海外 BBS の標準形（CP437 と ANSI）。 |
| c64_petscii | **"c64" として1つに統合し、F1 で直す** | C64 向けの BBS は「PETSCII の文字と制御コード」が標準。色は制御コードで送るのが一般的なので、これだけを正式な C64 用とする。 |
| c64（Plain、色なし） | 廃止して c64 に統合 | PETSCII の制御コードは、色を出せない C64 端末でも害が無いので、分けておく意味が無い。 |
| c64_ansi（PETSCII と ANSI の混在） | 廃止して c64 に統合 | PETSCII の文字に ANSI の ESC を混ぜる方式は、一般的な C64 端末ソフトの前提ではない（そもそも今は ESC が `?` になって動いていない）。 |
| 40col_sjis / 40col_utf8 | 残す。ただし、選択肢としては「幅 40」の設定として扱う（D3） | 画面の幅は、文字コードとは独立した端末の性質。一般的なホストは幅を別の項目として扱う。 |
| jterm40（SJIS、40桁、全文字1マス） | **組み込みから外す。config.toml.sample に、カスタムプロファイルの記入例として残す** | ユーザーが C64 で自作した特殊な端末用で、一般的な端末には無い形式。カスタムプロファイルの仕組み（Q2 を F1 で直せば完全に効く）で再現できるので、標準の一覧には置かない。 |

- **廃止する名前の互換**:
  - DB の `users.terminal` にある旧名は、別名として読み替える（c64 / c64_ansi / c64_petscii / petscii → c64）。
  - `jterm40` は、config にカスタムプロファイルとして定義されていればそれを使う。無ければ 40col_sjis として扱い、警告をログに出す。
  - 本番（oracle の beryl）で jterm40 を使っている人がいる可能性がある。そのため、F3 の PR には「config.toml に jterm40 の定義を足す手順」を書く。
- **消したと分かるように**: `docs/terminal_model.md` に「廃止したプリセット」の表を置く。何だったか（上の表の内容）と、代わりにどうすればよいかを書く。config.toml.sample の jterm40 の例にも同じ説明をコメントで付ける。

### D2. 接続時に選んだものと、保存してある設定のどちらを優先するか（F2 で実施）
- **決定: 「接続方式」（文字コードと、それに伴う出力方式）は、毎回の接続で選んだものを必ず使う。保存してある設定では上書きしない。**
  - 理由: 文字コードは「今使っている端末」の性質で、人ではなく機械で決まる。一般的なホストも、接続ごとに判定するか、接続ごとに聞く。
  - 理由: これで「別の環境からつなぐと設定を変えられない」問題と、Issue #269（PETSCII から戻れない）が同時に解決する。
- **言語**: ログイン後は、保存してある言語を使う。ただし、その文字コードで日本語を表示できない場合（CP437、PETSCII）は英語にする。
  - 理由: 言語は人の好みなので保存してある設定に従うのが一般的。ただし、表示できない言語を出すと画面が `?` だらけになる（現状の CP437 の問題）。
- **画面の幅・高さ、自動ページング**: 保存してある設定を使う（ログイン前は、接続方式ごとの既定値）。
  - 理由: 一般的なホストでも、ユーザー設定の既定値として保存されている項目。

### D3. ユーザーに何を選ばせるか（F2・F3 で実施）
- **決定: 選ぶ場所を2つに分ける。**
  1. **接続直後（ログイン前、毎回）**: ASCII だけで書いた短い一覧から「接続方式」を1つ選ぶ（Enter だけなら既定値）。
     ```
     1) Japanese  ShiftJIS
     2) Japanese  UTF-8
     3) English   UTF-8
     4) English   CP437 (DOS/ANSI)
     5) Commodore 64 (PETSCII)
     ```
     - 理由: 国内ホストの「漢字コードを選んでください」と同じ形。今の言語選択画面（E/J/U）を広げるだけなので、ユーザーの操作もほぼ変わらない。
  2. **ログイン後の設定画面（保存される）**: 言語、画面の幅（80/40）、高さ、色（ANSI）の有無、自動ページング。
     - 理由: 一般的なホストのユーザー設定と同じ粒度。「プリセットの名前を選ぶ」よりも、何が変わるのかが分かる。
- **文字コードを、接続方式と別に選ばせることはしない。**
  - 理由: 選択肢が増えるほど「何を選べばいいか分からない」が増える。変わった組み合わせが必要なら、カスタムプロファイルを使う。
- DB の `users.encoding` は、F3 で使わなくする（削除は別の migration で、互換を確認してから）。
- **自動判定**（TTYPE/NAWS による画面サイズの取得、応答による ANSI の判定）は F4 に回す（任意）。
  - 理由: 一般的なホストはやっているが、ShiftJIS と UTF-8 の区別は自動ではできない。まず手動の選択を確実にするのが先。

### D4. 使われていないコード（フェーズ4で実施）
- **決定: `src/screen`、`SessionHandler.screen`、`petscii` クレートへの依存、`create_session_handler_with_profile`、`*_strict` と `*_detailed` 系は削除する。**
  - 理由: どれも本番の経路から呼ばれていない。残しておくと「これも関係あるのか」と読む人を迷わせる。これは今回の混乱の一因。
- **決定: `template_dir` は、設定ファイルからは読むが使わない項目として残し、ドキュメントに「未使用」と明記する。**
  - 理由: 既存の config.toml に書かれていても起動が失敗しないようにするため（設定ファイルの互換）。

### D5. ゴールデンの仕組み
- **決定: 依存を増やさず、自前の小さなヘルパーにする**（`UPDATE_GOLDEN=1` のときだけ上書き保存）。
  - 理由: バイト列を比べるだけなので、大きな仕組みは要らない。CLAUDE.md の依存方針にも合う。

### D6. PR の単位
- **決定: フェーズ0〜4を、それぞれ別の PR にする（5本）。**
  - 理由: 「ゴールデンが1バイトも変わっていない」ことを、1つの種類の変更ごとに確かめられる。問題があっても、その PR だけ revert すれば戻せる。

### 残っている判断（実装の途中で決めてよい細かいもの）
- 接続方式の一覧の既定値（Enter だけ押したとき）を、config の `default_profile` から決めるのか、新しい設定項目にするのか。F2 の設計時に決める。

## Cross-review反映

### Round 1 — codex / gpt-6-astra
- Verdict: NEEDS_WORK (model=NEEDS_WORK; policy=strict; host_override=no)
- [adopted][medium] R1-F1 フェーズ2の `to_wire` が、フェーズ3で入る `TerminalSettings` に依存しており、フェーズ2の PR を単独でビルドできない → Change scope（wire.rs の項）、Implementation steps（冒頭の依存順序、フェーズ2の1、フェーズ3の2）。フェーズ2では既存の `CharacterEncoding` と `OutputMode` を引数にし、フェーズ3では呼び出し側だけを変える形に修正した。

### Round 2 — codex / gpt-6-astra
- Verdict: NEEDS_WORK (model=NEEDS_WORK; policy=strict; host_override=no)
- [adopted][medium] R2-F1 エコーの統合（`write_echo`）に対して、Password / Masked のエコーの回帰テストが無い → Implementation steps のフェーズ0に「条件軸 E2（エコーモード）」を追加した。4経路 × 3モードのエコーのバイト列を固定し、Password / Masked では入力した文字がエコーされないことを明示的に assert する。

### Round 3 — codex / gpt-6-astra
- Verdict: APPROVED (model=APPROVED; policy=strict; host_override=no)
- 指摘なし（R1-F1 と R2-F1 が解消されたことを確認）
