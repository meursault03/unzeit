BEGIN IMMEDIATE;
CREATE TABLE study_intervals(
  id TEXT PRIMARY KEY,
  lesson_id INTEGER REFERENCES lessons(id) ON DELETE SET NULL,
  started_at REAL NOT NULL,
  ended_at REAL NOT NULL,
  seconds REAL NOT NULL CHECK(seconds>0 AND ended_at>started_at)
);
CREATE INDEX study_interval_time ON study_intervals(started_at,ended_at);
PRAGMA user_version=4;
COMMIT;
