# HOBBS 運用ガイド

このドキュメントでは、HOBBSの運用に必要な手順とトラブルシューティングについて説明します。

## 目次

1. [サーバー運用手順](#サーバー運用手順)
2. [端末プロファイル設定](#端末プロファイル設定)
3. [Web UI設定](#web-ui設定)
4. [バックアップ・リストア](#バックアップリストア)
5. [トラブルシューティング](#トラブルシューティング)
6. [セキュリティ](#セキュリティ)

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

## 端末プロファイル設定

HOBBSは様々な端末タイプをサポートしています。

### 組み込みプロファイル

| プロファイル | 幅 | 高 | エンコーディング | 出力モード | 想定クライアント |
|--------------|----|----|------------------|------------|------------------|
| `standard` | 80 | 24 | ShiftJIS | ANSI | TeraTerm, PuTTY等（日本語） |
| `standard_utf8` | 80 | 24 | UTF-8 | ANSI | TeraTerm, PuTTY等（UTF-8） |
| `dos` | 80 | 25 | CP437 | ANSI | DOS端末、IBM PC互換機 |
| `c64` | 40 | 25 | PETSCII | Plain | C64（ANSI非対応） |
| `c64_petscii` | 40 | 25 | PETSCII | PetsciiCtrl | C64（PETSCII制御コード使用） |
| `c64_ansi` | 40 | 25 | PETSCII | ANSI | C64（ANSI対応エミュレータ） |

### 設定項目

`config.toml` の `[terminal]` セクション:

```toml
[terminal]
# デフォルトの端末プロファイル
default_profile = "standard"

# 自動ページングを有効にする（スクロール機能のない端末向け）
auto_paging = true

# ページング前に表示する行数（0 = 端末高さ - 4）
paging_lines = 0
```

### カスタムプロファイル

独自の端末プロファイルを定義できます：

```toml
# PC-98用プロファイルの例
[[terminal.profiles]]
name = "pc98"
width = 80
height = 25
cjk_width = 2
ansi_enabled = true
encoding = "shiftjis"
output_mode = "ansi"
template_dir = "80"

# MSX用プロファイルの例
[[terminal.profiles]]
name = "msx"
width = 40
height = 24
cjk_width = 1
ansi_enabled = true
encoding = "shiftjis"
output_mode = "ansi"
template_dir = "40"
```

#### カスタムプロファイルの設定項目

| 項目 | 必須 | デフォルト | 説明 |
|------|------|------------|------|
| `name` | ○ | - | プロファイル名 |
| `width` | - | 80 | 画面幅（カラム数） |
| `height` | - | 24 | 画面高（行数） |
| `cjk_width` | - | 2 | 全角文字の幅（1 or 2） |
| `ansi_enabled` | - | true | ANSIエスケープシーケンス対応 |
| `encoding` | - | "shiftjis" | 文字エンコーディング |
| `output_mode` | - | "ansi" | 出力モード |
| `template_dir` | - | "80" | テンプレートディレクトリ |

#### エンコーディング値

| 値 | 説明 |
|----|------|
| `shiftjis` | 日本語Shift_JIS |
| `utf8` | UTF-8 |
| `cp437` | IBM PC Code Page 437 |
| `petscii` | Commodore PETSCII |

#### 出力モード値

| 値 | 説明 |
|----|------|
| `ansi` | ANSIエスケープシーケンスをそのまま出力 |
| `plain` | ANSIエスケープシーケンスを除去 |
| `petscii_ctrl` | ANSIをPETSCII制御コードに変換 |

カスタムプロファイルは、Telnetログイン時の端末選択画面に組み込みプロファイルと共に表示されます。

---

## Web UI設定

HOBBSはTelnetに加えて、REST API + SPAベースのWeb UIを提供します。

### 設定項目

`config.toml` の `[web]` セクション:

```toml
[web]
# Web UIを有効にする
enabled = true

# バインドするホストアドレス
host = "0.0.0.0"

# Web APIのポート番号
port = 8080

# CORS許可オリジン（開発環境用）
cors_origins = ["http://localhost:5173"]

# JWT秘密鍵（必須、環境変数で上書き可能）
jwt_secret = ""

# アクセストークンの有効期限（秒）
jwt_access_token_expiry_secs = 900  # 15分

# リフレッシュトークンの有効期限（日）
jwt_refresh_token_expiry_days = 7

# 静的ファイル配信を有効にする
serve_static = true

# 静的ファイルのパス
static_path = "web/dist"

# ログインのレート制限（回/分）
login_rate_limit = 5

# 一般APIのレート制限（回/分）
api_rate_limit = 100
```

### 環境変数

以下の環境変数で設定を上書きできます（推奨）：

| 環境変数 | 説明 |
|----------|------|
| `HOBBS_JWT_SECRET` | JWT秘密鍵（本番環境では必須） |

### 本番環境での設定

1. **JWT秘密鍵の設定**（必須）

   JWT秘密鍵は環境変数で設定することを推奨します：

   ```bash
   # ランダムな秘密鍵を生成
   openssl rand -base64 32

   # 環境変数として設定
   export HOBBS_JWT_SECRET="生成した秘密鍵"
   ```

   systemdサービスの場合:
   ```ini
   [Service]
   Environment="HOBBS_JWT_SECRET=your-secret-key"
   ```

2. **CORS設定**

   本番環境では、正しいオリジンのみを許可：
   ```toml
   cors_origins = ["https://your-domain.com"]
   ```

3. **HTTPSの設定**

   本番環境ではリバースプロキシ（nginx/Caddy）でHTTPSを終端することを推奨します。

   nginx設定例:
   ```nginx
   server {
       listen 443 ssl;
       server_name your-domain.com;

       ssl_certificate /path/to/cert.pem;
       ssl_certificate_key /path/to/key.pem;

       location /api/ {
           proxy_pass http://127.0.0.1:8080;
           proxy_set_header Host $host;
           proxy_set_header X-Real-IP $remote_addr;
           proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
           proxy_set_header X-Forwarded-Proto $scheme;
       }

       location / {
           root /opt/hobbs/web/dist;
           try_files $uri $uri/ /index.html;
       }
   }
   ```

### セキュリティ機能

Web UIには以下のセキュリティ機能が組み込まれています：

1. **レート制限**
   - ログイン: 5回/分（デフォルト）
   - 一般API: 100回/分（デフォルト）
   - IPアドレスベースで制限

2. **セキュリティヘッダー**
   - `X-Content-Type-Options: nosniff`
   - `X-Frame-Options: DENY`
   - `Referrer-Policy: strict-origin-when-cross-origin`
   - `Cache-Control: no-store, max-age=0`

3. **JWT認証**
   - アクセストークン（短期、15分）
   - リフレッシュトークン（長期、7日）
   - トークンの自動更新

### ヘルスチェック

Web APIのヘルスチェックエンドポイント：

```bash
curl http://localhost:8080/health
# レスポンス: OK
```

### ログ

Web API関連のログは標準のログファイルに出力されます：

```bash
# リアルタイム監視
tail -f logs/hobbs.log | grep "web\|api\|http"
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

# === SQLite版の場合 ===
# データベースをSQLiteのバックアップ機能でコピー
sqlite3 "$HOBBS_DIR/data/hobbs.db" ".backup '$BACKUP_DIR/hobbs_$DATE.db'"

# === PostgreSQL版の場合 ===
# pg_dump -U hobbs -h localhost hobbs > "$BACKUP_DIR/hobbs_$DATE.sql"
# または圧縮形式で:
# pg_dump -U hobbs -h localhost -Fc hobbs > "$BACKUP_DIR/hobbs_$DATE.dump"

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

#### SQLite版

```bash
#!/bin/bash
# restore_sqlite.sh

BACKUP_FILE="$1"
HOBBS_DIR="/opt/hobbs"

if [ -z "$BACKUP_FILE" ]; then
    echo "Usage: restore_sqlite.sh <backup_db_file>"
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

#### PostgreSQL版

```bash
#!/bin/bash
# restore_postgres.sh

BACKUP_FILE="$1"
DB_NAME="hobbs"
DB_USER="hobbs"

if [ -z "$BACKUP_FILE" ]; then
    echo "Usage: restore_postgres.sh <backup_file.sql or .dump>"
    exit 1
fi

# サーバー停止
sudo systemctl stop hobbs

# データベースを再作成
dropdb -U "$DB_USER" "$DB_NAME"
createdb -U "$DB_USER" "$DB_NAME"

# リストア（.sql形式）
# psql -U "$DB_USER" "$DB_NAME" < "$BACKUP_FILE"

# リストア（.dump形式）
pg_restore -U "$DB_USER" -d "$DB_NAME" "$BACKUP_FILE"

# サーバー起動
sudo systemctl start hobbs

echo "Restore completed"
```

### オンラインバックアップ

#### SQLite版

SQLiteのWALモードを使用しているため、サーバー稼働中でもバックアップ可能です：

```bash
sqlite3 data/hobbs.db ".backup data/hobbs_backup.db"
```

#### PostgreSQL版

PostgreSQLは標準でオンラインバックアップに対応しています：

```bash
pg_dump -U hobbs -h localhost hobbs > hobbs_backup.sql
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

#### SQLite版

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

#### PostgreSQL版

1. **接続エラー**
   ```bash
   # PostgreSQLサービスが起動しているか確認
   sudo systemctl status postgresql

   # 接続テスト
   psql -U hobbs -h localhost -d hobbs -c "SELECT 1;"
   ```

2. **認証エラー**
   - `pg_hba.conf` の設定を確認
   - パスワード認証が許可されているか確認

3. **接続数超過**
   ```bash
   # 現在の接続数を確認
   psql -U hobbs -c "SELECT count(*) FROM pg_stat_activity WHERE datname='hobbs';"
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
