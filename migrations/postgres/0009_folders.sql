-- Folders table for file management (hierarchical structure)
CREATE TABLE folders (
    id          BIGSERIAL PRIMARY KEY,
    name        TEXT NOT NULL,
    description TEXT,
    parent_id   BIGINT REFERENCES folders(id) ON DELETE CASCADE,
    permission  TEXT NOT NULL DEFAULT 'member',  -- Read permission
    upload_perm TEXT NOT NULL DEFAULT 'subop',   -- Upload permission
    order_num   INTEGER NOT NULL DEFAULT 0,
    created_at  TIMESTAMP NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_folders_parent ON folders(parent_id, order_num);
