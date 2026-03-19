ALTER TABLE short_url ADD COLUMN user_id BIGINT REFERENCES users(id);

-- index on owned, active short_urls
CREATE INDEX idx_short_url_user_id_active ON short_url(user_id) WHERE deleted_at IS NULL;