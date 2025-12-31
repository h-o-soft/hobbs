# HOBBS - 機能仕様書: チャットルーム

## 1. 概要

リアルタイムで会話できるチャットルーム機能。単一ロビー形式で、接続中の会員全員が参加可能。

## 2. 基本仕様

| 項目 | 仕様 |
|------|------|
| ルーム数 | 1（単一ロビー） |
| 参加資格 | 会員のみ（ゲスト不可） |
| 同時参加 | 制限なし |
| ログ保存 | する（オプション） |

## 3. 画面構成

```
================================================================================
    チャットルーム                                             参加者: 4人
================================================================================

    [20:30:15] === たろう が入室しました ===
    [20:30:20] はなこ: こんばんは〜
    [20:30:25] たろう: こんばんは！
    [20:30:30] じろう: やあ
    [20:31:00] はなこ: 今日は寒いですね
    [20:31:15] じろう: うん、こっちは雪降ってるよ

    ...（スクロール領域）...

    現在の参加者: たろう, はなこ, じろう, さぶろう
--------------------------------------------------------------------------------
    [/quit] 退室    [/who] 参加者一覧    [/help] ヘルプ
--------------------------------------------------------------------------------
発言 >
```

## 4. メッセージ形式

### 4.1 システムメッセージ

```
[HH:MM:SS] === ユーザー名 が入室しました ===
[HH:MM:SS] === ユーザー名 が退室しました ===
[HH:MM:SS] === システムメッセージ ===
```

色: シアン（36）

### 4.2 通常発言

```
[HH:MM:SS] ユーザー名: メッセージ内容
```

- 時刻: 青（34）
- ユーザー名: マゼンタ（35）
- メッセージ: 白（デフォルト）

### 4.3 自分の発言

```
[HH:MM:SS] > メッセージ内容
```

色: 緑（32）で強調

## 5. コマンド

| コマンド | 説明 |
|----------|------|
| `/quit` または `/q` | チャットルームから退室 |
| `/who` または `/w` | 現在の参加者一覧を表示 |
| `/help` または `/h` または `/?` | ヘルプを表示 |
| `/me <アクション>` | アクション表示 |

### 5.1 /me コマンド

```
入力: /me は考え込んでいる
出力: [20:31:00] * たろう は考え込んでいる
```

## 6. 機能詳細

### 6.1 入室処理

```
1. チャットメニュー選択
2. 入室確認
3. 入室メッセージをブロードキャスト
4. 最新20件のログを表示
5. 発言待ち状態へ
```

### 6.2 発言処理

```
1. ユーザーがテキスト入力
2. 入力が/で始まる場合 → コマンド処理
3. それ以外 → 発言としてブロードキャスト
4. 全参加者の画面に表示
```

### 6.3 退室処理

```
1. /quit コマンドまたは切断
2. 退室メッセージをブロードキャスト
3. 参加者リストから削除
4. メインメニューへ戻る
```

### 6.4 リアルタイム表示

- 他者の発言はリアルタイムで表示
- 入力中でも新着メッセージを表示
- 画面は自動スクロール

## 7. 技術実装

### 7.1 ブロードキャスト

```rust
use tokio::sync::broadcast;

pub struct ChatRoom {
    tx: broadcast::Sender<ChatMessage>,
    participants: Arc<RwLock<HashMap<i64, Participant>>>,
}

impl ChatRoom {
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(100);
        Self {
            tx,
            participants: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn join(&self, user: &User) -> broadcast::Receiver<ChatMessage> {
        let mut participants = self.participants.write().unwrap();
        participants.insert(user.id, Participant::new(user));

        // 入室通知
        let _ = self.tx.send(ChatMessage::system(
            format!("{} が入室しました", user.nickname)
        ));

        self.tx.subscribe()
    }

    pub fn leave(&self, user_id: i64) {
        let mut participants = self.participants.write().unwrap();
        if let Some(p) = participants.remove(&user_id) {
            let _ = self.tx.send(ChatMessage::system(
                format!("{} が退室しました", p.nickname)
            ));
        }
    }

    pub fn send(&self, user: &User, message: &str) {
        let _ = self.tx.send(ChatMessage::new(user, message));
    }

    pub fn who(&self) -> Vec<String> {
        self.participants.read().unwrap()
            .values()
            .map(|p| p.nickname.clone())
            .collect()
    }
}
```

### 7.2 メッセージ構造

```rust
#[derive(Clone, Debug)]
pub struct ChatMessage {
    pub kind: MessageKind,
    pub timestamp: DateTime,
    pub sender: Option<String>,
    pub content: String,
}

#[derive(Clone, Debug)]
pub enum MessageKind {
    System,
    Chat,
    Action,
}
```

### 7.3 セッション処理

```rust
async fn chat_loop(session: &mut Session, room: &ChatRoom) -> Result<()> {
    let mut rx = room.join(&session.user);

    // 初期ログ表示
    for msg in room.recent_messages(20) {
        session.send(&msg.format())?;
    }

    loop {
        tokio::select! {
            // 新着メッセージを受信
            msg = rx.recv() => {
                if let Ok(msg) = msg {
                    session.send(&msg.format())?;
                }
            }
            // ユーザー入力を受信
            input = session.read_line() => {
                let input = input?;
                if input.starts_with('/') {
                    match handle_command(&input, session, room).await? {
                        CommandResult::Quit => break,
                        CommandResult::Continue => {}
                    }
                } else {
                    room.send(&session.user, &input);
                }
            }
        }
    }

    room.leave(session.user.id);
    Ok(())
}
```

## 8. ログ保存（オプション）

### 8.1 ログ形式

```
2025-01-15 20:30:15 [JOIN] たろう
2025-01-15 20:30:20 [CHAT] はなこ: こんばんは〜
2025-01-15 20:30:25 [CHAT] たろう: こんばんは！
2025-01-15 20:31:00 [ACTION] たろう は考え込んでいる
2025-01-15 20:35:00 [LEAVE] たろう
```

### 8.2 データベース保存

```sql
CREATE TABLE chat_logs (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id     INTEGER NOT NULL REFERENCES users(id),
    message     TEXT NOT NULL,
    kind        TEXT NOT NULL,  -- 'chat', 'action', 'system'
    created_at  TEXT NOT NULL DEFAULT (datetime('now'))
);
```

## 9. 制限事項

| 項目 | 制限値 |
|------|--------|
| 発言最大長 | 500文字 |
| ログ保持件数 | 直近1000件 |
| 連続発言制限 | 1秒間隔 |

## 10. 禁止事項・フィルタ（将来検討）

- 連続投稿制限
- NGワードフィルタ
- ミュート機能
