# Implementation Plan: Restore Settings & API Key Integration

## 1. Restore `src/routes/Settings.tsx`

The `Settings.tsx` file is currently corrupted. I will recreate it with the following features:
- **UI Components**: Use Mantine components (`TextInput`, `PasswordInput`, `NumberInput`, `Button`, `Stack`, `Group`, `Card`, `Title`) to match the application's design.
- **State Management**: Load settings from the backend using `getSettings` on mount.
- **Form Fields**:
  - `provider_name` (Text)
  - `base_url` (Text)
  - `model` (Text)
  - `api_key` (Password)
  - `temperature` (Number)
  - `max_tokens` (Number)
- **Persistence**: Save all settings to the backend using `setSettings` when the "Save" button is clicked.
- **Feedback**: Show a success notification or toast (if available, otherwise simple alert or UI state change) upon saving.

## 2. Update Backend Workflows

The backend currently expects the API key to be in the `SPECTRAIL_API_KEY` environment variable. I will update the workflows to read it from the database settings instead.

### `src-tauri/src/workflows/plan.rs`
- **Current**: `get_api_key()` reads from `std::env::var("SPECTRAIL_API_KEY")`.
- **Change**: Modify `get_api_key` to accept the `settings` HashMap (which is already fetched in `generate_plan`) and look up the "api_key" value.
- **Fallback**: Keep the environment variable as a fallback if the setting is missing (optional, but good for backward compatibility/dev).

### `src-tauri/src/workflows/verify.rs`
- **Current**: `get_api_key()` reads from `std::env::var("SPECTRAIL_API_KEY")`.
- **Change**: Similar to `plan.rs`, modify `get_api_key` to accept the `settings` HashMap and look up "api_key".

## 3. Verification
- **Frontend**: Verify that the Settings page loads, allows editing, and saves values to the database (persists after reload).
- **Backend**: Verify that running a plan or verification task successfully retrieves the API key from the database and calls the LLM.

## Todo List
- [ ] Re-implement `src/routes/Settings.tsx`
- [ ] Update `src-tauri/src/workflows/plan.rs` to use settings for API key
- [ ] Update `src-tauri/src/workflows/verify.rs` to use settings for API key
