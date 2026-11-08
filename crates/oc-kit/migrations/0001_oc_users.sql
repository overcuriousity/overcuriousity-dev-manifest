CREATE TABLE oc_users (
    id INTEGER PRIMARY KEY,
    username TEXT NOT NULL UNIQUE COLLATE NOCASE,
    display_name TEXT NOT NULL,
    is_admin INTEGER NOT NULL DEFAULT 0,
    password_hash TEXT,
    created_at TEXT NOT NULL,
    disabled_at TEXT
) STRICT;

CREATE TABLE oc_oidc_identities (
    issuer TEXT NOT NULL,
    subject TEXT NOT NULL,
    user_id INTEGER NOT NULL REFERENCES oc_users(id) ON DELETE CASCADE,
    PRIMARY KEY (issuer, subject)
) STRICT;

CREATE TABLE oc_passkeys (
    id INTEGER PRIMARY KEY,
    user_id INTEGER NOT NULL REFERENCES oc_users(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    credential TEXT NOT NULL,
    created_at TEXT NOT NULL,
    last_used_at TEXT
) STRICT;

CREATE TABLE oc_api_tokens (
    id TEXT PRIMARY KEY,
    user_id INTEGER NOT NULL REFERENCES oc_users(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    secret_hash TEXT NOT NULL,
    created_at TEXT NOT NULL,
    last_used_at TEXT,
    revoked_at TEXT
) STRICT;

CREATE TABLE oc_preferences (
    user_id INTEGER PRIMARY KEY REFERENCES oc_users(id) ON DELETE CASCADE,
    theme TEXT NOT NULL DEFAULT 'system' CHECK (theme IN ('system', 'light', 'dark')),
    density TEXT NOT NULL DEFAULT 'default' CHECK (density IN ('default', 'compact'))
) STRICT;
