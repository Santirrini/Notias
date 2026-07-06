-- Phase 4 — tasks, SRS, quizzes, weekly plan
-- Schema follows spec §8 (SRS), §9 (IPC), §13 row 4 (exit criteria).

CREATE TABLE tasks (
    id TEXT PRIMARY KEY,
    note_id TEXT REFERENCES notes(id) ON DELETE SET NULL,
    title TEXT NOT NULL,
    priority INTEGER NOT NULL DEFAULT 0,    -- 0=low, 1=med, 2=high
    status TEXT NOT NULL DEFAULT 'todo',    -- todo / doing / done / cancelled
    due_at TEXT,                             -- ISO8601, nullable
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    done_at TEXT
);
CREATE INDEX idx_tasks_status ON tasks(status);
CREATE INDEX idx_tasks_due ON tasks(due_at);
CREATE INDEX idx_tasks_note ON tasks(note_id);

CREATE TABLE srs_cards (
    id TEXT PRIMARY KEY,
    note_id TEXT REFERENCES notes(id) ON DELETE SET NULL,
    front TEXT NOT NULL,
    back TEXT NOT NULL,
    ease REAL NOT NULL DEFAULT 2.5,
    interval_days INTEGER NOT NULL DEFAULT 0,
    repetitions INTEGER NOT NULL DEFAULT 0,
    due_at TEXT NOT NULL,                   -- ISO8601, used by queue query
    created_at TEXT NOT NULL,
    suspended_at TEXT                        -- soft-disable
);
-- ponytail: partial index keeps the queue query tight even when suspended cards pile up.
CREATE INDEX idx_srs_due_active ON srs_cards(due_at) WHERE suspended_at IS NULL;
CREATE UNIQUE INDEX idx_srs_nodupe ON srs_cards(note_id, front);

CREATE TABLE srs_reviews (
    id TEXT PRIMARY KEY,
    card_id TEXT NOT NULL REFERENCES srs_cards(id) ON DELETE CASCADE,
    reviewed_at TEXT NOT NULL,
    quality INTEGER NOT NULL,                -- 1=Again, 3=Hard, 4=Good, 5=Easy
    interval_after INTEGER NOT NULL,
    ease_after REAL NOT NULL
);
CREATE INDEX idx_srs_reviews_card ON srs_reviews(card_id);

CREATE TABLE study_sessions (
    id TEXT PRIMARY KEY,
    kind TEXT NOT NULL,                       -- 'plan' | 'srs' | 'quiz'
    started_at TEXT NOT NULL,
    ended_at TEXT,
    plan_json TEXT,                           -- the spec §8 shape, only when kind='plan'
    note TEXT
);

CREATE TABLE quiz_attempts (
    id TEXT PRIMARY KEY,
    session_id TEXT REFERENCES study_sessions(id) ON DELETE SET NULL,
    note_id TEXT REFERENCES notes(id) ON DELETE SET NULL,
    questions_json TEXT NOT NULL,
    answers_json TEXT,
    score INTEGER,
    total INTEGER NOT NULL,
    created_at TEXT NOT NULL,
    completed_at TEXT
);
CREATE INDEX idx_quiz_session ON quiz_attempts(session_id);
