-- Settings table for BYOK provider configuration (non-sensitive)
CREATE TABLE IF NOT EXISTS settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- Insert default settings for OpenAI-compatible API
-- These defaults match the README documentation
INSERT OR IGNORE INTO settings (key, value, updated_at) VALUES
('provider_name', 'openai', datetime('now')),
('base_url', 'https://openrouter.ai/api/v1', datetime('now')),
('model', 'z-ai/glm-4.7-flash', datetime('now')),
('temperature', '0.2', datetime('now')),
('max_tokens', '2000', datetime('now')),
('extra_headers_json', '{}', datetime('now')),
('dev_mode', '0', datetime('now'));
