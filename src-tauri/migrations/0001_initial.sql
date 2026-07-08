-- Phase 0 baseline: just enough to prove migrations work.
-- Real tables arrive in later migrations.
-- ponytail: `schema_version` is bootstrapped in code (`migrations::run`) before this
-- migration runs, so we only declare the application tables here.

CREATE TABLE audit_log (
    id INTEGER PRIMARY KEY,
    ts TEXT NOT NULL,
    actor TEXT NOT NULL,
    action TEXT NOT NULL,
    payload_json TEXT
);