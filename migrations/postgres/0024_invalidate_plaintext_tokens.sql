-- Invalidate all existing refresh tokens.
-- This migration is required because we changed from storing plaintext tokens
-- to storing SHA256 hashes. Existing plaintext tokens won't match the new
-- hash-based validation, so we need to revoke them all.
-- Users will need to re-login after this migration.

UPDATE refresh_tokens SET revoked_at = TO_CHAR(NOW(), 'YYYY-MM-DD HH24:MI:SS') WHERE revoked_at IS NULL;
