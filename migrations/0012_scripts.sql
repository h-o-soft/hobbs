-- Scripts table for Lua script metadata (file system-based)
CREATE TABLE scripts (
    id                      INTEGER PRIMARY KEY AUTOINCREMENT,
    file_path               TEXT NOT NULL UNIQUE,       -- Relative path from scripts directory
    name                    TEXT NOT NULL,              -- Display name (from metadata)
    slug                    TEXT NOT NULL UNIQUE,       -- URL-safe identifier
    description             TEXT,                       -- Description (from metadata)
    author                  TEXT,                       -- Author name (from metadata)
    file_hash               TEXT,                       -- File hash for change detection
    synced_at               TEXT,                       -- Last sync timestamp
    min_role                INTEGER NOT NULL DEFAULT 0, -- Minimum role to execute (0=Guest)
    enabled                 INTEGER NOT NULL DEFAULT 1, -- Whether the script is enabled
    max_instructions        INTEGER NOT NULL DEFAULT 1000000,
    max_memory_mb           INTEGER NOT NULL DEFAULT 10,
    max_execution_seconds   INTEGER NOT NULL DEFAULT 30
);

CREATE INDEX idx_scripts_enabled ON scripts(enabled);
CREATE INDEX idx_scripts_min_role ON scripts(min_role);
CREATE INDEX idx_scripts_file_path ON scripts(file_path);
CREATE INDEX idx_scripts_slug ON scripts(slug);
