-- Files table for file metadata
CREATE TABLE files (
    id           BIGSERIAL PRIMARY KEY,
    folder_id    BIGINT NOT NULL REFERENCES folders(id) ON DELETE CASCADE,
    filename     TEXT NOT NULL,           -- Display filename
    stored_name  TEXT NOT NULL,           -- Storage filename (UUID)
    size         BIGINT NOT NULL,         -- Bytes
    description  TEXT,
    uploader_id  BIGINT NOT NULL REFERENCES users(id),
    downloads    BIGINT NOT NULL DEFAULT 0,
    created_at   TEXT NOT NULL DEFAULT TO_CHAR(NOW(), 'YYYY-MM-DD HH24:MI:SS')
);

CREATE INDEX idx_files_folder ON files(folder_id);
CREATE INDEX idx_files_uploader ON files(uploader_id);
