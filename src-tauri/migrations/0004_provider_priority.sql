-- 0004_provider_priority.sql
ALTER TABLE provider_settings ADD COLUMN chat_priority INTEGER NOT NULL DEFAULT 100;

-- Ensure openai and groq rows exist with sane defaults (ollama already inserted in 0003).
INSERT OR IGNORE INTO provider_settings(name, enabled, config_json, chat_priority) VALUES
    ('openai', 0, '{"base_url":"https://api.openai.com/v1","chat_model":"gpt-4o-mini","embed_model":"text-embedding-3-small"}', 100),
    ('groq',   0, '{"base_url":"https://api.groq.com/openai/v1","chat_model":"llama-3.1-70b-versatile","transcribe_model":"whisper-large-v3-turbo"}', 200);

-- ponytail: chat_priority defines fallback order. Lower = tried first.
-- Default 0 is reserved for ollama (set explicitly below). Cloud defaults to 100+ in insertion order.
UPDATE provider_settings SET chat_priority = 0 WHERE name = 'ollama';

-- Default transcription provider (Groq Whisper).
ALTER TABLE provider_settings ADD COLUMN transcribe_provider TEXT NOT NULL DEFAULT 'groq';
