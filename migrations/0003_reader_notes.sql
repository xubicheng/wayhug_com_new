-- 读者读书笔记提交与采纳后元信息。可重复执行。

CREATE TABLE IF NOT EXISTS note_submissions (
  id            INTEGER PRIMARY KEY AUTOINCREMENT,
  receipt       TEXT NOT NULL UNIQUE,
  occurred_at   TEXT NOT NULL DEFAULT '',
  weekday       INTEGER,
  place         TEXT NOT NULL DEFAULT '',
  weather       TEXT NOT NULL DEFAULT '',
  temp_c        TEXT NOT NULL DEFAULT '',
  book_title    TEXT NOT NULL DEFAULT '',
  isbn          TEXT NOT NULL DEFAULT '',
  title         TEXT NOT NULL,
  body          TEXT NOT NULL DEFAULT '',
  question      TEXT NOT NULL DEFAULT '',
  color         TEXT NOT NULL DEFAULT '',
  flag          TEXT NOT NULL DEFAULT '',
  links         TEXT NOT NULL DEFAULT '',
  lang          TEXT NOT NULL DEFAULT 'zh',
  status        TEXT NOT NULL DEFAULT 'pending',
  ip_hash       TEXT NOT NULL DEFAULT '',
  ip_region     TEXT NOT NULL DEFAULT '',
  created_at    TEXT NOT NULL DEFAULT '',
  reviewed_at   TEXT,
  reviewer      TEXT,
  reject_reason TEXT,
  review_note   TEXT,
  note_id       INTEGER REFERENCES notes(id),
  original_json TEXT NOT NULL DEFAULT '',
  edited_at     TEXT,
  edited_by     TEXT
);

CREATE INDEX IF NOT EXISTS idx_note_submissions_status
  ON note_submissions(status, created_at);
CREATE INDEX IF NOT EXISTS idx_note_submissions_ip
  ON note_submissions(ip_hash, created_at);

CREATE TABLE IF NOT EXISTS reader_note_meta (
  note_id       INTEGER PRIMARY KEY REFERENCES notes(id) ON DELETE CASCADE,
  submission_id INTEGER NOT NULL UNIQUE REFERENCES note_submissions(id),
  occurred_at   TEXT NOT NULL DEFAULT '',
  weekday       INTEGER,
  place         TEXT NOT NULL DEFAULT '',
  weather       TEXT NOT NULL DEFAULT '',
  temp_c        TEXT NOT NULL DEFAULT '',
  book_title    TEXT NOT NULL DEFAULT '',
  isbn          TEXT NOT NULL DEFAULT '',
  question      TEXT NOT NULL DEFAULT '',
  color         TEXT NOT NULL DEFAULT '',
  flag          TEXT NOT NULL DEFAULT '',
  links         TEXT NOT NULL DEFAULT '',
  created_at    TEXT NOT NULL DEFAULT ''
);
