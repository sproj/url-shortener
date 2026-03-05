CREATE TABLE short_url (
    id  BIGSERIAL PRIMARY KEY,
    uuid UUID,
    code TEXT NOT NULL UNIQUE,
    long_url TEXT NOT NULL,
    expires_at TIMESTAMPTZ NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NULL,
    deleted_at TIMESTAMPTZ NULL
);

CREATE INDEX idx_short_url_long_url ON short_url(long_url);

CREATE UNIQUE INDEX short_url_code_active_uniq ON short_url(code) WHERE deleted_at IS NULL;