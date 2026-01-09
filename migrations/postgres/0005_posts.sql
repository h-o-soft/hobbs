-- Posts table for both thread and flat boards
CREATE TABLE posts (
    id          BIGSERIAL PRIMARY KEY,
    board_id    BIGINT NOT NULL REFERENCES boards(id) ON DELETE CASCADE,
    thread_id   BIGINT REFERENCES threads(id) ON DELETE CASCADE,  -- NULL for flat boards
    author_id   BIGINT NOT NULL REFERENCES users(id),
    title       TEXT,                                               -- Used for flat boards
    body        TEXT NOT NULL,
    created_at  TIMESTAMP NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_posts_board_id ON posts(board_id);
CREATE INDEX idx_posts_thread_id ON posts(thread_id);
CREATE INDEX idx_posts_author_id ON posts(author_id);
CREATE INDEX idx_posts_created_at ON posts(created_at);
