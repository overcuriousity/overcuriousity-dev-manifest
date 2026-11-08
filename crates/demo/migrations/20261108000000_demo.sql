CREATE TABLE cases (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    summary TEXT NOT NULL DEFAULT '',
    created_at TEXT NOT NULL
) STRICT;

CREATE TABLE events (
    id INTEGER PRIMARY KEY,
    case_id INTEGER NOT NULL REFERENCES cases(id) ON DELETE CASCADE,
    ts INTEGER NOT NULL,
    host TEXT NOT NULL,
    user TEXT NOT NULL,
    event_id INTEGER NOT NULL,
    level TEXT NOT NULL,
    source TEXT NOT NULL,
    process TEXT NOT NULL,
    pid INTEGER,
    parent TEXT,
    command_line TEXT,
    record INTEGER NOT NULL,
    integrity TEXT,
    logon_id TEXT,
    source_file TEXT NOT NULL,
    sha256 TEXT,
    message TEXT NOT NULL,
    verdict TEXT CHECK (verdict IN ('benign', 'suspicious', 'malicious')),
    pinned_at TEXT
) STRICT;

CREATE INDEX events_case_ts ON events (case_id, ts DESC, id DESC);
CREATE INDEX events_case_host ON events (case_id, host);
CREATE INDEX events_case_user ON events (case_id, user);
CREATE INDEX events_case_event_id ON events (case_id, event_id);

CREATE TABLE notes (
    id INTEGER PRIMARY KEY,
    case_id INTEGER REFERENCES cases(id) ON DELETE CASCADE,
    author_id INTEGER NOT NULL REFERENCES oc_users(id),
    title TEXT NOT NULL,
    body TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    deleted_at TEXT
) STRICT;

CREATE TABLE note_tags (
    note_id INTEGER NOT NULL REFERENCES notes(id) ON DELETE CASCADE,
    tag TEXT NOT NULL,
    PRIMARY KEY (note_id, tag)
) STRICT;
