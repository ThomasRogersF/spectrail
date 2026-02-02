# SpecTrail (Desktop)

SpecTrail is a BYOK, desktop-first planning + workflow engine for AI-assisted coding:
**Plan → Phase → Verify → Replay**, with everything stored in a local SQLite database so you can revisit projects, tasks, prompts, tool calls, and run history.

## Stack
- **Tauri** (Rust backend)
- **React + Vite + TypeScript** (frontend)
- **SQLite** (local DB) via `rusqlite`

## Getting started

### Prereqs
- Node.js 18+ (or 20+ recommended)
- Rust toolchain (stable)
- (Optional) `pnpm` (recommended)

### Install dependencies
```bash
cd spectrail
pnpm install   # or npm install
```

### Run desktop app (dev)
```bash
pnpm tauri dev   # or npm run tauri dev
```

### Build desktop app
```bash
pnpm tauri build
```

## What's included in this scaffold
- App routes:
  - `/projects`
  - `/projects/:id`
  - `/projects/:id/tasks/:taskId`
  - `/projects/:id/tasks/:taskId/runs/:runId`
  - `/settings`
- Minimal database + migrations:
  - `.migrations/001_init.sql` is applied on first launch
  - `.migrations/002_settings.sql` adds settings table
- Rust commands (Tauri IPC) for:
  - projects, tasks, runs, messages, artifacts
  - settings (get_settings, get_setting, set_setting, set_settings)
- UI screens with basic styling placeholders

## Sprint 1: Settings (Completed)

Sprint 1 implements a local-first settings system that stores non-secret BYOK configuration in SQLite.

### Settings stored:
| Setting | Default | Description |
|---------|---------|-------------|
| provider_name | router | Provider identifier |
| base_url | https://api.openai.com/v1 | API endpoint URL |
| model | gpt-4.1-mini | Model identifier |
| temperature | 0.2 | Sampling temperature |
| max_tokens | 2000 | Max tokens per request |
| extra_headers_json | {} | Custom headers as JSON |
| dev_mode | 0 | Enable dev mode warnings |

### Features:
- Settings persist across app restarts
- Validation for URLs and JSON
- Dev mode banner warning
- Bulk save with transactions

### API added:
```rust
get_settings() -> Vec<SettingsKV>
get_setting(key: String) -> Option<String>
set_setting(key: String, value: String)
set_settings(pairs: Vec<SettingInput>)
```

## Next steps
1. Add LLM client with OpenAI-compatible API + tool calling
2. Add repo tooling (list files / read / grep / git diff / run tests)
3. Add Plan Mode and Phases Mode workflow engine
4. Add Verify (plan compliance + test runner + diff review)
