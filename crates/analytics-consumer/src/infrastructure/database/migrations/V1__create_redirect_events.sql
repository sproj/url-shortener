CREATE TABLE redirect_events (
    id BIGSERIAL PRIMARY KEY,
    code TEXT NOT NULL,
    long_url TEXT NOT NULL,
    redirect_type TEXT NOT NULL,
    timestamp TIMESTAMPTZ NOT NULL,
    recorded_at TIMESTAMPTZ NOT NULL DEFAULT now()
)