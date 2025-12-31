# HOBBS 運用ガイド

このドキュメントでは、HOBBSの運用に必要な手順とトラブルシューティングについて説明します。

## 目次

1. [サーバー運用手順](#サーバー運用手順)
2. [バックアップ・リストア](#バックアップリストア)
3. [トラブルシューティング](#トラブルシューティング)
4. [セキュリティ](#セキュリティ)

---

## サーバー運用手順

### 起動

```bash
# 通常起動
./hobbs

# バックグラウンド起動（Linux/macOS）
nohup ./hobbs > /dev/null 2>&1 &

# systemdサービスとして起動（推奨）
sudo systemctl start hobbs
```

### 停止

サーバーを安全に停止するには、以下の方法があります：

1. **Ctrl+C**: フォアグラウンドで実行中の場合
2. **SIGTERMシグナル**: `kill <PID>`
3. **systemctl**: `sudo systemctl stop hobbs`

### 再起動

```bash
# systemdを使用している場合
sudo systemctl restart hobbs
```

### ステータス確認

```bash
# プロセス確認
ps aux | grep hobbs

# ポート確認
lsof -i :2323

# systemdステータス
sudo systemctl status hobbs
```

### systemdサービス設定（例）

`/etc/systemd/system/hobbs.service`:

```ini
[Unit]
Description=HOBBS - Hobbyist Bulletin Board System
After=network.target

[Service]
Type=simple
User=hobbs
Group=hobbs
WorkingDirectory=/opt/hobbs
ExecStart=/opt/hobbs/hobbs
Restart=on-failure
RestartSec=5

[Install]
WantedBy=multi-user.target
```

有効化:

```bash
sudo systemctl daemon-reload
sudo systemctl enable hobbs
sudo systemctl start hobbs
```

---

## バックアップ・リストア

### バックアップ対象

| ファイル/ディレクトリ | 内容 | 重要度 |
|----------------------|------|--------|
| `data/hobbs.db` | データベース | 最重要 |
| `data/files/` | アップロードファイル | 重要 |
| `config.toml` | 設定ファイル | 重要 |
| `locales/` | 言語リソース（カスタマイズ時） | 任意 |
| `templates/` | テンプレート（カスタマイズ時） | 任意 |

### バックアップ手順

#### 手動バックアップ

```bash
#!/bin/bash
# backup.sh

BACKUP_DIR="/backup/hobbs"
HOBBS_DIR="/opt/hobbs"
DATE=$(date +%Y%m%d_%H%M%S)

# バックアップディレクトリ作成
mkdir -p "$BACKUP_DIR"

# データベースをSQLiteのバックアップ機能でコピー
sqlite3 "$HOBBS_DIR/data/hobbs.db" ".backup '$BACKUP_DIR/hobbs_$DATE.db'"

# ファイルストレージをコピー
tar -czf "$BACKUP_DIR/files_$DATE.tar.gz" -C "$HOBBS_DIR/data" files/

# 設定ファイルをコピー
cp "$HOBBS_DIR/config.toml" "$BACKUP_DIR/config_$DATE.toml"

# 古いバックアップを削除（30日以上前）
find "$BACKUP_DIR" -type f -mtime +30 -delete

echo "Backup completed: $DATE"
```

#### cronによる定期バックアップ

```bash
# /etc/cron.d/hobbs-backup
0 3 * * * hobbs /opt/hobbs/scripts/backup.sh >> /var/log/hobbs-backup.log 2>&1
```

### リストア手順

```bash
#!/bin/bash
# restore.sh

BACKUP_FILE="$1"
HOBBS_DIR="/opt/hobbs"

if [ -z "$BACKUP_FILE" ]; then
    echo "Usage: restore.sh <backup_db_file>"
    exit 1
fi

# サーバー停止
sudo systemctl stop hobbs

# 既存DBをバックアップ
mv "$HOBBS_DIR/data/hobbs.db" "$HOBBS_DIR/data/hobbs.db.old"

# リストア
cp "$BACKUP_FILE" "$HOBBS_DIR/data/hobbs.db"

# サーバー起動
sudo systemctl start hobbs

echo "Restore completed"
```

### オンラインバックアップ

SQLiteのWALモードを使用しているため、サーバー稼働中でもバックアップ可能です：

```bash
sqlite3 data/hobbs.db ".backup data/hobbs_backup.db"
```

---

## トラブルシューティング

### 接続できない

1. **サーバーが起動しているか確認**
   ```bash
   ps aux | grep hobbs
   ```

2. **ポートがリッスンしているか確認**
   ```bash
   lsof -i :2323
   # または
   ss -tlnp | grep 2323
   ```

3. **ファイアウォール設定を確認**
   ```bash
   # iptables
   sudo iptables -L -n | grep 2323

   # ufw
   sudo ufw status

   # firewalld
   sudo firewall-cmd --list-all
   ```

4. **ログを確認**
   ```bash
   tail -f logs/hobbs.log
   ```

### データベースエラー

1. **データベースのロック**
   - 複数プロセスが同時にアクセスしている可能性
   - `lsof data/hobbs.db` で確認

2. **データベースの破損**
   ```bash
   # 整合性チェック
   sqlite3 data/hobbs.db "PRAGMA integrity_check;"

   # 修復を試みる
   sqlite3 data/hobbs.db ".recover" | sqlite3 data/hobbs_recovered.db
   ```

3. **WALファイルの問題**
   ```bash
   # WALチェックポイント
   sqlite3 data/hobbs.db "PRAGMA wal_checkpoint(TRUNCATE);"
   ```

### メモリ使用量が高い

1. **接続数を確認**
   - 管理画面でアクティブセッション数を確認
   - `max_connections` 設定を調整

2. **アイドルタイムアウトを短くする**
   ```toml
   [server]
   idle_timeout_secs = 180  # 5分から3分に短縮
   ```

### 文字化け

1. **クライアントの文字コード設定を確認**
   - Tera Term: 設定 → 端末 → 漢字（受信）→ SJIS
   - PuTTY: 設定 → ウィンドウ → 変換 → Shift_JIS

2. **ターミナルタイプを確認**
   - ANSIエスケープシーケンス対応クライアントを使用

### ログの確認方法

```bash
# リアルタイム監視
tail -f logs/hobbs.log

# エラーのみ抽出
grep -i error logs/hobbs.log

# 特定ユーザーの操作を追跡
grep "username" logs/hobbs.log
```

### デバッグモード

詳細なログを出力するには：

```bash
RUST_LOG=debug ./hobbs
```

ログレベル：
- `error`: エラーのみ
- `warn`: 警告以上
- `info`: 通常運用（デフォルト）
- `debug`: 詳細情報
- `trace`: 最大詳細

---

## セキュリティ

### 推奨設定

1. **専用ユーザーで実行**
   ```bash
   sudo useradd -r -s /bin/false hobbs
   sudo chown -R hobbs:hobbs /opt/hobbs
   ```

2. **ファイル権限**
   ```bash
   chmod 600 config.toml     # 設定ファイル
   chmod 600 data/hobbs.db   # データベース
   chmod 700 data/files      # ファイルストレージ
   ```

3. **ファイアウォール**
   ```bash
   # 特定IPのみ許可
   sudo ufw allow from 192.168.1.0/24 to any port 2323

   # 全て許可
   sudo ufw allow 2323/tcp
   ```

### パスワードポリシー

- 最小長: 8文字
- 最大長: 128文字
- ハッシュ: Argon2id

### セッション管理

- デフォルトタイムアウト: 5分
- 同一ユーザーの複数セッション: 許可
- ログイン試行制限: 5回失敗で一時ロック

### 定期メンテナンス

1. **古いセッションの削除**（自動）
2. **ログローテーション**
   ```bash
   # logrotateの設定例
   /opt/hobbs/logs/*.log {
       daily
       rotate 30
       compress
       delaycompress
       missingok
       notifempty
   }
   ```

3. **データベースの最適化**
   ```bash
   sqlite3 data/hobbs.db "VACUUM;"
   sqlite3 data/hobbs.db "ANALYZE;"
   ```

---

## 監視

### ヘルスチェック

```bash
#!/bin/bash
# healthcheck.sh

PORT=2323
HOST="localhost"

if nc -z "$HOST" "$PORT"; then
    echo "HOBBS is running"
    exit 0
else
    echo "HOBBS is NOT running"
    exit 1
fi
```

### メトリクス

ログから以下の情報を収集できます：
- 接続数
- ログイン成功/失敗数
- エラー発生数

---

## 更新手順

1. サーバー停止
2. バックアップ取得
3. 新しいバイナリをデプロイ
4. 設定ファイル確認（新しい設定項目がないか）
5. サーバー起動
6. 動作確認

```bash
# 更新スクリプト例
sudo systemctl stop hobbs
./scripts/backup.sh
cp hobbs-new /opt/hobbs/hobbs
sudo systemctl start hobbs
./scripts/healthcheck.sh
```
