-- 0003_ai_provider_settings.sql
CREATE TABLE IF NOT EXISTS provider_settings (
    name         TEXT PRIMARY KEY,
    enabled      INTEGER NOT NULL DEFAULT 0,
    config_json  TEXT NOT NULL DEFAULT '{}'
);
INSERT OR IGNORE INTO provider_settings(name, enabled, config_json) VALUES
    ('ollama', 0, '{"base_url":"http://127.0.0.1:11434","chat_model":"llama3.2","embed_model":"nomic-embed-text"}'),
    ('openai', 0, '{}'),
    ('groq',   0, '{}');

-- ponytail: sqlite-vec requires INTEGER PRIMARY KEY (the implicit rowid) on vec0 tables.
-- We store notes.rowid in note_vec.rowid and JOIN ON n.rowid = v.rowid in rag_search.
CREATE VIRTUAL TABLE IF NOT EXISTS note_vec USING vec0(
    embedding float[768]
);