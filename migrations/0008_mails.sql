-- Mails table for internal mail system
CREATE TABLE mails (
    id                      INTEGER PRIMARY KEY AUTOINCREMENT,
    sender_id               INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    recipient_id            INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    subject                 TEXT NOT NULL,
    body                    TEXT NOT NULL,
    is_read                 INTEGER NOT NULL DEFAULT 0,
    is_deleted_by_sender    INTEGER NOT NULL DEFAULT 0,
    is_deleted_by_recipient INTEGER NOT NULL DEFAULT 0,
    created_at              TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_mails_sender_id ON mails(sender_id);
CREATE INDEX idx_mails_recipient_id ON mails(recipient_id);
CREATE INDEX idx_mails_created_at ON mails(created_at);
