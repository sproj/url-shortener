CREATE TABLE short_url (
    id  BIGSERIAL PRIMARY KEY,
    code TEXT NOT NULL UNIQUE,
    long_url TEXT NOT NULL,
    expires_at TIMESTAMPTZ NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NULL,
    deleted_at TIMESTAMPTZ NULL
);

CREATE INDEX idx_short_url_long_url ON short_url(long_url);