-- wayhug.com 新版数据结构（SQLite）
-- 语言是一等字段：内容主体与译文分表，新增语言只加数据行。

CREATE TABLE IF NOT EXISTS notes (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    slug            TEXT NOT NULL UNIQUE,
    lang            TEXT NOT NULL DEFAULT 'zh',
    status          TEXT NOT NULL DEFAULT 'draft',      -- draft | published
    published_at    TEXT,
    created_at      TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at      TEXT NOT NULL DEFAULT (datetime('now')),
    guest_author_id INTEGER REFERENCES guest_authors(id),
    license         TEXT,
    tags            TEXT NOT NULL DEFAULT ''
);
CREATE INDEX IF NOT EXISTS idx_notes_published ON notes(status, published_at DESC);

CREATE TABLE IF NOT EXISTS note_i18n (
    note_id  INTEGER NOT NULL REFERENCES notes(id) ON DELETE CASCADE,
    lang     TEXT NOT NULL,
    title    TEXT NOT NULL,
    summary  TEXT NOT NULL DEFAULT '',
    body     TEXT NOT NULL DEFAULT '',
    PRIMARY KEY (note_id, lang)
);

CREATE TABLE IF NOT EXISTS subscribers (
    id                 INTEGER PRIMARY KEY AUTOINCREMENT,
    email              TEXT NOT NULL UNIQUE,
    lang               TEXT NOT NULL DEFAULT 'zh',
    source             TEXT NOT NULL DEFAULT '',
    status             TEXT NOT NULL DEFAULT 'pending', -- pending | confirmed | unsubscribed
    created_at         TEXT NOT NULL DEFAULT (datetime('now')),
    confirmed_at       TEXT,
    unsubscribed_at    TEXT,
    confirm_token_hash TEXT,
    confirm_expires_at TEXT,
    manage_token_hash  TEXT,
    last_mail_at       TEXT,
    ip_hash            TEXT
);
CREATE INDEX IF NOT EXISTS idx_subscribers_status ON subscribers(status);

CREATE TABLE IF NOT EXISTS guest_authors (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    name          TEXT NOT NULL,
    bio           TEXT NOT NULL DEFAULT '',
    contact_email TEXT,
    status        TEXT NOT NULL DEFAULT 'invited',      -- invited | active | revoked
    created_at    TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS invites (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    guest_author_id INTEGER NOT NULL REFERENCES guest_authors(id) ON DELETE CASCADE,
    code_hash       TEXT NOT NULL,
    expires_at      TEXT NOT NULL,
    used_at         TEXT,
    created_at      TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_invites_hash ON invites(code_hash);

CREATE TABLE IF NOT EXISTS submissions (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    guest_author_id INTEGER REFERENCES guest_authors(id),
    name            TEXT NOT NULL,
    email           TEXT,
    lang            TEXT NOT NULL DEFAULT 'zh',
    title           TEXT NOT NULL,
    body            TEXT NOT NULL,
    license         TEXT NOT NULL DEFAULT '',
    status          TEXT NOT NULL DEFAULT 'pending',    -- pending | approved | rejected
    created_at      TEXT NOT NULL DEFAULT (datetime('now')),
    reviewed_at     TEXT,
    note_id         INTEGER REFERENCES notes(id)
);
CREATE INDEX IF NOT EXISTS idx_submissions_status ON submissions(status);

CREATE TABLE IF NOT EXISTS products (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    slug        TEXT NOT NULL UNIQUE,
    price_cents INTEGER NOT NULL,
    currency    TEXT NOT NULL DEFAULT 'CNY',
    stock       TEXT NOT NULL DEFAULT 'in_stock',       -- in_stock | preorder | sold_out
    url         TEXT,
    created_at  TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at  TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS product_i18n (
    product_id  INTEGER NOT NULL REFERENCES products(id) ON DELETE CASCADE,
    lang        TEXT NOT NULL,
    name        TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    revenue_note TEXT NOT NULL DEFAULT '',
    PRIMARY KEY (product_id, lang)
);

CREATE TABLE IF NOT EXISTS sessions (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    token_hash   TEXT NOT NULL UNIQUE,
    csrf         TEXT NOT NULL,
    created_at   TEXT NOT NULL DEFAULT (datetime('now')),
    expires_at   TEXT NOT NULL,
    last_seen_at TEXT,
    ip           TEXT
);

CREATE TABLE IF NOT EXISTS audit_log (
    id     INTEGER PRIMARY KEY AUTOINCREMENT,
    at     TEXT NOT NULL DEFAULT (datetime('now')),
    actor  TEXT NOT NULL,
    action TEXT NOT NULL,
    target TEXT,
    detail TEXT,
    ip     TEXT
);
CREATE INDEX IF NOT EXISTS idx_audit_at ON audit_log(at DESC);

CREATE TABLE IF NOT EXISTS mail_outbox (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    to_email   TEXT NOT NULL,
    subject    TEXT NOT NULL,
    body       TEXT NOT NULL,
    kind       TEXT NOT NULL,
    status     TEXT NOT NULL DEFAULT 'pending',         -- pending | sent | failed | logged
    error      TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    sent_at    TEXT
);
CREATE INDEX IF NOT EXISTS idx_outbox_created ON mail_outbox(created_at DESC);

CREATE TABLE IF NOT EXISTS settings (
    k TEXT PRIMARY KEY,
    v TEXT NOT NULL
);
