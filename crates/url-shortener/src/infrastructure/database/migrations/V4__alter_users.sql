ALTER TABLE short_url DROP COLUMN IF EXISTS user_id;
ALTER TABLE short_url ADD COLUMN user_id UUID;

CREATE INDEX idx_short_url_user_id_active ON short_url(user_id) WHERE deleted_at IS NULL;

DROP TABLE IF EXISTS USERS;
