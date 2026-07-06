-- Phase 6 — calendar mirror + sync state
CREATE TABLE calendar_events (
    gcal_id TEXT PRIMARY KEY,
    summary TEXT NOT NULL,
    description TEXT,
    starts_at TEXT NOT NULL,
    ends_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    source TEXT NOT NULL DEFAULT 'google'
);
CREATE INDEX idx_calendar_starts ON calendar_events(starts_at);

CREATE TABLE calendar_sync_state (
    scope TEXT PRIMARY KEY,
    last_pulled_at TEXT,
    last_error TEXT
);