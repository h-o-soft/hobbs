-- Folders table for file management (hierarchical structure)
CREATE TABLE folders (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    name        TEXT NOT NULL,
    description TEXT,
    parent_id   INTEGER REFERENCES folders(id) ON DELETE CASCADE,
    permission  TEXT NOT NULL DEFAULT 'member',  -- 閲覧権限
    upload_perm TEXT NOT NULL DEFAULT 'subop',   -- アップロード権限
    order_num   INTEGER NOT NULL DEFAULT 0,
    created_at  TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_folders_parent ON folders(parent_id, order_num);
