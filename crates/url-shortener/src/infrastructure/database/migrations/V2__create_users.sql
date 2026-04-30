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

--populate users table
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
        '019d0b7f-77c4-7fd2-acc2-d78789845da3',
        'admin',
        'admin@admin.com',
        -- pass1234, argon2id(pwd + salt)
        '$argon2id$v=19$m=19456,t=2,p=1$k3CGJ1BlOvUvJ6zGULarIQ$RLchvry0DuLIkBbhxt9vADBQq4WkXNNEgY02Awk/lrs',
        'k3CGJ1BlOvUvJ6zGULarIQ',
        'true',
        'user,admin',
        now(),
        now(),
        NULL
    );