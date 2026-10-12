BEGIN IMMEDIATE;
CREATE INDEX jobs_status ON jobs(status);
PRAGMA user_version=6;
COMMIT;
