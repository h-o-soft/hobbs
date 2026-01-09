-- Files table for file metadata
CREATE TABLE files (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    folder_id    INTEGER NOT NULL REFERENCES folders(id) ON DELETE CASCADE,
    filename     TEXT NOT NULL,           -- 表示用ファイル名
    stored_name  TEXT NOT NULL,           -- 保存時のファイル名（UUID）
    size         INTEGER NOT NULL,        -- バイト数
    description  TEXT,
    uploader_id  INTEGER NOT NULL REFERENCES users(id),
    downloads    INTEGER NOT NULL DEFAULT 0,
    created_at   TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_files_folder ON files(folder_id);
CREATE INDEX idx_files_uploader ON files(uploader_id);
