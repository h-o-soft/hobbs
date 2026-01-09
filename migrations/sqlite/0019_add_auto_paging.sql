-- Add auto_paging setting to users table
ALTER TABLE users ADD COLUMN auto_paging INTEGER NOT NULL DEFAULT 1;
