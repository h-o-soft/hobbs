-- Rename duplicate usernames (case-insensitive) by appending suffix
-- Keep the first one (lowest id), rename others with _1, _2, etc.
UPDATE users
SET username = username || '_' || (
    SELECT COUNT(*)
    FROM users u2
    WHERE LOWER(u2.username) = LOWER(users.username)
      AND u2.id < users.id
)
WHERE id IN (
    SELECT u1.id
    FROM users u1
    WHERE EXISTS (
        SELECT 1 FROM users u2
        WHERE LOWER(u2.username) = LOWER(u1.username)
          AND u2.id < u1.id
    )
);

-- Drop the old case-sensitive index
DROP INDEX IF EXISTS idx_users_username;

-- Create new case-insensitive unique index
CREATE UNIQUE INDEX idx_users_username_nocase ON users(username COLLATE NOCASE);
