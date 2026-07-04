-- ponytail: embeddings table is write-only here; populated by Phase 2's RAG pipeline.
CREATE TABLE notes (
    id TEXT PRIMARY KEY,
    path TEXT NOT NULL UNIQUE,
    title TEXT NOT NULL,
    created TEXT NOT NULL,
    updated TEXT NOT NULL,
    deleted_at TEXT,
    embedding_status TEXT NOT NULL DEFAULT 'pending'
);

CREATE VIRTUAL TABLE notes_fts USING fts5(
    title, body,
    content=''
);

CREATE TABLE note_tags (
    note_id TEXT NOT NULL REFERENCES notes(id) ON DELETE CASCADE,
    tag TEXT NOT NULL,
    PRIMARY KEY (note_id, tag)
);

CREATE TABLE note_embeddings (
    note_id TEXT PRIMARY KEY REFERENCES notes(id) ON DELETE CASCADE,
    vec BLOB NOT NULL,
    model TEXT NOT NULL
);

CREATE TABLE note_attachments (
    note_id TEXT NOT NULL REFERENCES notes(id) ON DELETE CASCADE,
    path TEXT NOT NULL,
    kind TEXT NOT NULL,
    PRIMARY KEY (note_id, path)
);

CREATE INDEX idx_notes_updated ON notes(updated);
CREATE INDEX idx_notes_deleted ON notes(deleted_at);