CREATE TABLE users (
    id BIGSERIAL PRIMARY KEY,
    uuid UUID UNIQUE NOT NULL,
    username TEXT NOT NULL,
    email TEXT UNIQUE NOT NULL,
    password_hash TEXT NOT NULL,
    password_salt TEXT NOT NULL,
    active BOOLEAN NOT NULL DEFAULT TRUE,
    roles TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ DEFAULT now(),
    deleted_at TIMESTAMPTZ
);

-- index on non-deleted users
CREATE INDEX idx_users_email_active ON users(email) WHERE deleted_at IS NULL;

-- populate users table
INSERT INTO users (
        uuid,
        username,
        email,
        password_hash,
        password_salt,
        active,
        roles,
        created_at,
        updated_at,
        deleted_at
    )
VALUES (
        '0ca4906b-15f5-4365-841d-07e5eb431ef1',
        'admin',
        'admin@admin.com',
        -- pswd1234, sha256(pwd + salt) as hex lower-case
        'cf432b08ee63230b6831e2c86456e4a60beb65fa21b363c7a0e497fd29021597', -- 
        -- 32 u32 to utf8, range 32..126 (first 32 chars are unprintable) as char -> 96^32 possible salts 
        'g7YqA&^8Mz4tD3!sP*1r^0KcJ6f$mQh',
        'true',
        'admin',
        now(),
        now(),
        NULL
    );