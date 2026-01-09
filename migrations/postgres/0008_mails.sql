-- Mails table for internal mail system
CREATE TABLE mails (
    id                      BIGSERIAL PRIMARY KEY,
    sender_id               BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    recipient_id            BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    subject                 TEXT NOT NULL,
    body                    TEXT NOT NULL,
    is_read                 BOOLEAN NOT NULL DEFAULT FALSE,
    is_deleted_by_sender    BOOLEAN NOT NULL DEFAULT FALSE,
    is_deleted_by_recipient BOOLEAN NOT NULL DEFAULT FALSE,
    created_at              TEXT NOT NULL DEFAULT TO_CHAR(NOW(), 'YYYY-MM-DD HH24:MI:SS')
);

CREATE INDEX idx_mails_sender_id ON mails(sender_id);
CREATE INDEX idx_mails_recipient_id ON mails(recipient_id);
CREATE INDEX idx_mails_created_at ON mails(created_at);
