# Sprint 1: Settings System Implementation Spec

## Overview
Implement a local-first settings system that stores non-secret BYOK configuration in SQLite.

## File Changes

### 1. Database Migration
**File**: `src-tauri/migrations/002_settings.sql`
```sql
CREATE TABLE IF NOT EXISTS settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
```

### 2. Database Initialization
**File**: `src-tauri/src/db.rs`
- Update `init_db()` to apply both migrations
- Use `include_str!` for both 001_init.sql and 002_settings.sql

### 3. Backend Commands
**File**: `src-tauri/src/commands.rs`
Add these commands:
- `get_settings() -> Vec<SettingsKV>` - returns all settings
- `get_setting(key: String) -> Option<String>` - returns single setting
- `set_setting(key: String, value: String) -> Result<(), String>` - upsert single
- `set_settings(pairs: Vec<SettingInput>) -> Result<(), String>` - bulk upsert with transaction

Use SQL:
```sql
INSERT INTO settings (key, value, updated_at) VALUES (?1, ?2, ?3)
ON CONFLICT(key) DO UPDATE SET value=excluded.value, updated_at=excluded.updated_at
```

### 4. Type Definitions
**File**: `src-tauri/src/models.rs`
Add:
```rust
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SettingsKV {
    pub key: String,
    pub value: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct SettingInput {
    pub key: String,
    pub value: String,
}
```

### 5. Command Registration
**File**: `src-tauri/src/lib.rs`
Add to invoke_handler:
```rust
get_settings,
get_setting,
set_setting,
set_settings,
```

### 6. Frontend API
**File**: `src/lib/api.ts`
Add:
```typescript
export async function getSettings(): Promise<Array<{key: string, value: string}>>
export async function setSetting(key: string, value: string): Promise<void>
export async function setSettings(pairs: Array<{key: string, value: string}>): Promise<void>
```

### 7. Settings UI
**File**: `src/routes/Settings.tsx`

Form fields with defaults:
- provider_name: "router"
- base_url: "https://api.openai.com/v1"
- model: "gpt-4.1-mini"
- temperature: "0.2"
- max_tokens: "2000"
- extra_headers_json: "{}"
- dev_mode: "0"

Validation:
- base_url must start with http:// or https://
- extra_headers_json must parse as valid JSON object
- temperature/max_tokens should be numeric (warning only)

UI elements:
- Form inputs for all fields
- Dev mode toggle (checkbox)
- Save button with bulk update
- "Saved" confirmation
- Warning banner when dev_mode is true

## Default Settings Object
```typescript
const DEFAULT_SETTINGS = {
  provider_name: "router",
  base_url: "https://api.openai.com/v1",
  model: "gpt-4.1-mini",
  temperature: "0.2",
  max_tokens: "2000",
  extra_headers_json: "{}",
  dev_mode: "0",
};
```

## Acceptance Criteria
- [ ] App launches without errors
- [ ] Settings table created in SQLite
- [ ] Settings persist across restarts
- [ ] Validation prevents invalid JSON in extra_headers_json
- [ ] Dev mode warning banner shows when enabled
- [ ] No API key storage exists yet
