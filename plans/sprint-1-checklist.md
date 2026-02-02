# Sprint 1 Testing Guide

**Date:** 2026-02-01  
**Purpose:** Step-by-step manual testing guide for Sprint 1 Settings System implementation

---

## Prerequisites

### 1. Environment Setup

```bash
# Install dependencies
pnpm install

# Build and run the app (development mode)
pnpm tauri dev
```

**Verify:** App launches without errors and shows the main window

### 2. Access the Settings Page

1. Look for a **Settings** link/navigation in the app
2. Click it to navigate to the Settings page
3. **Verify:** Settings page loads with the following fields:
   - Provider Name (default: `router`)
   - Base URL (default: `https://api.openai.com/v1`)
   - Model (default: `gpt-4.1-mini`)
   - Temperature (default: `0.2`)
   - Max Tokens (default: `2000`)
   - Extra Headers (JSON) (default: `{}`)
   - Dev Mode checkbox (default: unchecked)
   - Database status indicator showing path

---

## Test 1: Settings Persistence Across Restart

**Purpose:** Verify that settings saved in one session are restored after app restart

### Steps

1. **Open Settings page** and note the current values

2. **Modify all fields** with distinct test values:
   | Field | Test Value |
   |-------|------------|
   | Provider Name | `openai-test` |
   | Base URL | `https://test.example.com/v1` |
   | Model | `gpt-4-test` |
   | Temperature | `0.8` |
   | Max Tokens | `4000` |
   | Extra Headers (JSON) | `{"X-Test": "value"}` |
   | Dev Mode | Check the checkbox |

3. **Click "Save Settings"**
   - **Verify:** Green "✓ Saved" appears briefly
   - **Verify:** No error messages displayed

4. **Close the app completely** (not just minimize)
   - Close the Tauri window
   - Ensure no background processes remain

5. **Reopen the app:**
   ```bash
   pnpm tauri dev
   ```

6. **Navigate back to Settings page**

7. **Verify all values persisted:**
   - Provider Name: `openai-test`
   - Base URL: `https://test.example.com/v1`
   - Model: `gpt-4-test`
   - Temperature: `0.8`
   - Max Tokens: `4000`
   - Extra Headers: `{"X-Test": "value"}`
   - Dev Mode: **Yellow warning banner** appears at top

### Expected Result

✅ All modified values are restored exactly as saved  
✅ Dev Mode warning banner displays when enabled  
✅ Database path shown in status indicator

### Failure Indicators

- Fields show default values instead of saved values
- Dev Mode checkbox is unchecked after restart
- Yellow warning banner does not appear

---

## Test 2: Migration Idempotency

**Purpose:** Verify that running migrations multiple times does not cause errors or data loss

### Steps

1. **Locate the database file:**
   - Look at the Settings page Database section
   - Note the path (e.g., `C:\Users\<username>\AppData\Roaming\com.spectrail.dev\spectrail.sqlite`)

2. **Verify database exists:**
   ```bash
   # Windows PowerShell
   Test-Path "C:\Users\$env:USERNAME\AppData\Roaming\com.spectrail.dev\spectrail.sqlite"
   ```

3. **Close the app completely**

4. **Reopen the app** (this triggers migration on startup)
   ```bash
   pnpm tauri dev
   ```

5. **Check Settings page** - verify previously saved settings are still present

6. **Close and reopen the app 2 more times** (total 3 restarts after initial setup)

7. **Navigate to Settings and verify:**
   - All previously saved settings are intact
   - Database status shows "OK"
   - No errors in terminal console

### Expected Result

✅ App starts without errors on every launch  
✅ Settings data is never corrupted or lost  
✅ No duplicate table errors in console  
✅ Migration SQL uses `CREATE TABLE IF NOT EXISTS` (already verified in [`002_settings.sql`](src-tauri/migrations/002_settings.sql:2))

### Verification via SQL (Optional)

```bash
# Using sqlite3 CLI
sqlite3 "C:\Users\<username>\AppData\Roaming\com.spectrail.dev\spectrail.sqlite" ".tables"
# Should show: artifacts, messages, projects, runs, settings, tasks

sqlite3 "C:\Users\<username>\AppData\Roaming\com.spectrail.dev\spectrail.sqlite" "SELECT * FROM settings;"
# Should show all saved key-value pairs
```

---

## Test 3: extra_headers_json Validation

**Purpose:** Verify that invalid JSON is rejected and valid JSON objects are accepted

### Test 3a: Invalid JSON - Syntax Error

1. Go to Settings page
2. In **Extra Headers (JSON)** field, enter:
   ```
   {invalid json}
   ```
3. Click **Save Settings**

**Expected:** 
- Save is blocked
- Red error message appears below field: "Invalid JSON"
- No "✓ Saved" confirmation

### Test 3b: Invalid JSON - Array Instead of Object

1. Clear the field and enter:
   ```json
   ["header1", "header2"]
   ```
2. Click **Save Settings**

**Expected:**
- Save is blocked
- Red error message: "Must be a valid JSON object"

### Test 3c: Invalid JSON - String Instead of Object

1. Clear the field and enter:
   ```json
   "just a string"
   ```
2. Click **Save Settings**

**Expected:**
- Save is blocked
- Red error message: "Must be a valid JSON object"

### Test 3d: Valid JSON Object - Should Save

1. Clear the field and enter:
   ```json
   {"Authorization": "Bearer test123", "X-Custom-Header": "value"}
   ```
2. Click **Save Settings**

**Expected:**
- Green "✓ Saved" appears
- No error messages
- Settings persist after restart (verify by closing/reopening app)

### Test 3e: Empty Object - Should Save

1. Clear the field and enter:
   ```json
   {}
   ```
2. Click **Save Settings**

**Expected:**
- Green "✓ Saved" appears
- Settings persist after restart

### Test 3f: Nested JSON Object - Should Save

1. Clear the field and enter:
   ```json
   {"nested": {"key": "value"}, "array": [1, 2, 3]}
   ```
2. Click **Save Settings**

**Expected:**
- Green "✓ Saved" appears
- Settings persist after restart

---

## Test 4: Transaction Verification (Code Review)

**Purpose:** Verify that [`set_settings`](src-tauri/src/commands.rs:310) uses atomic transactions for bulk updates

### Code Review Steps

1. **Open file:** [`src-tauri/src/commands.rs`](src-tauri/src/commands.rs)

2. **Navigate to line 309-325** and verify the following code pattern:
   ```rust
   #[tauri::command]
   pub fn set_settings(app: AppHandle, pairs: Vec<SettingInput>) -> Result<(), String> {
     let mut conn = db::connect(&app).map_err(|e| e.to_string())?;
     let tx = conn.transaction().map_err(|e| e.to_string())?;  // <-- TRANSACTION START
     let updated_at = now_iso();
     
     for pair in pairs {
       tx.execute(
         "INSERT INTO settings (key, value, updated_at) VALUES (?1, ?2, ?3)
          ON CONFLICT(key) DO UPDATE SET value=excluded.value, updated_at=excluded.updated_at",
         (&pair.key, &pair.value, &updated_at)
       ).map_err(|e| e.to_string())?;
     }
     
     tx.commit().map_err(|e| e.to_string())?;  // <-- TRANSACTION COMMIT
     Ok(())
   }
   ```

3. **Verify the transaction pattern:**
   - [ ] `conn.transaction()` is called to start a transaction
   - [ ] All inserts use `tx.execute()` (not `conn.execute()`)
   - [ ] `tx.commit()` is called after all operations
   - [ ] Error mapping uses `?` operator for early return on failure

4. **Open file:** [`src-tauri/migrations/002_settings.sql`](src-tauri/migrations/002_settings.sql)

5. **Verify the table schema:**
   ```sql
   CREATE TABLE IF NOT EXISTS settings (
       key TEXT PRIMARY KEY,
       value TEXT NOT NULL,
       updated_at TEXT NOT NULL
   );
   ```
   - [ ] Primary key on `key` field
   - [ ] `ON CONFLICT(key) DO UPDATE` pattern is appropriate for upsert behavior

### Expected Result

✅ All checklist items above are confirmed  
✅ Transaction ensures all-or-nothing save behavior  
✅ No partial save scenarios possible

---

## Test 5: Defaults Merging

**Purpose:** Verify that the UI merges default values with database values correctly

### Test Setup

First, clear all settings to simulate a fresh database:

1. **Close the app**

2. **Delete or rename the database** (optional - for clean test):
   ```powershell
   # Windows
   Rename-Item "$env:APPDATA\com.spectrail.dev\spectrail.sqlite" "spectrail.sqlite.backup"
   ```

3. **Reopen the app:**
   ```bash
   pnpm tauri dev
   ```

### Test Steps

1. **Navigate to Settings page**

2. **Verify default values are displayed:**
   | Field | Expected Default |
   |-------|-----------------|
   | Provider Name | `router` |
   | Base URL | `https://api.openai.com/v1` |
   | Model | `gpt-4.1-mini` |
   | Temperature | `0.2` |
   | Max Tokens | `2000` |
   | Extra Headers | `{}` |
   | Dev Mode | unchecked |

3. **Change only ONE field:**
   - Change **Model** to `custom-model-123`
   - Leave all other fields at defaults
   - Click **Save Settings**

4. **Close the app completely**

5. **Reopen the app and navigate to Settings**

6. **Verify merging behavior:**
   - [ ] Model shows `custom-model-123` (from database)
   - [ ] All other fields still show their **default values** (not empty!)
   - [ ] Provider Name: `router`
   - [ ] Base URL: `https://api.openai.com/v1`
   - [ ] Temperature: `0.2`
   - [ ] Max Tokens: `2000`
   - [ ] Extra Headers: `{}`

7. **Check code implementation** (optional verification):
   - Open [`src/routes/Settings.tsx`](src/routes/Settings.tsx:30-44)
   - Verify the merge logic:
   ```typescript
   async function loadSettings() {
     try {
       const dbSettings = await getSettings();
       const merged: SettingsMap = { ...DEFAULT_SETTINGS };  // Start with defaults
       for (const { key, value } of dbSettings) {
         if (key in DEFAULT_SETTINGS) {
           merged[key] = value;  // Override with DB values
         }
       }
       setSettingsState(merged);
     } catch (e) {
       console.error("Failed to load settings:", e);
     }
   }
   ```

### Expected Result

✅ Fields with saved values show saved values  
✅ Fields without saved values show defaults (not empty)  
✅ Merge logic correctly combines `DEFAULT_SETTINGS` with database values  

---

## Summary Checklist

| Test | Description | Status |
|------|-------------|--------|
| ☐ Prerequisites | App runs and Settings page loads | |
| ☐ Test 1 | Settings persist across restart | |
| ☐ Test 2 | Migration idempotency verified | |
| ☐ Test 3a | Invalid JSON rejected (syntax error) | |
| ☐ Test 3b | Invalid JSON rejected (array) | |
| ☐ Test 3c | Invalid JSON rejected (string) | |
| ☐ Test 3d | Valid JSON object accepted | |
| ☐ Test 3e | Empty object accepted | |
| ☐ Test 3f | Nested object accepted | |
| ☐ Test 4 | Transaction pattern verified in code | |
| ☐ Test 5 | Defaults merging works correctly | |

---

## Troubleshooting

### Issue: Settings don't persist after restart

**Symptoms:** Saved values revert to defaults after app restart

**Diagnostic Steps:**

1. **Check database file exists:**
   ```powershell
   Get-Item "$env:APPDATA\com.spectrail.dev\spectrail.sqlite"
   ```

2. **Verify settings are being written:**
   ```powershell
   sqlite3 "$env:APPDATA\com.spectrail.dev\spectrail.sqlite" "SELECT * FROM settings;"
   ```

3. **Check browser console for errors:**
   - Press `F12` or `Ctrl+Shift+I` in the app
   - Look for red error messages when clicking Save

4. **Check Rust console for errors:**
   - Look at the terminal where `pnpm tauri dev` is running
   - Search for "settings" or "sql" errors

**Common Fixes:**
- Database file may be in a different location than expected
- Migration may not have run - check [`db.rs`](src-tauri/src/db.rs:42-45)

---

### Issue: "Invalid JSON" error on valid input

**Symptoms:** Extra Headers field rejects valid JSON

**Diagnostic Steps:**

1. **Verify JSON format:**
   - Must use double quotes: `"key": "value"` (not single quotes)
   - Must be an object `{}`, not an array `[]` or string `""`

2. **Check for invisible characters:**
   - Copy the JSON to a text editor
   - Ensure no smart quotes or special Unicode characters

3. **Test with minimal valid JSON:**
   ```json
   {}
   ```

---

### Issue: Dev Mode warning not showing

**Symptoms:** Checkbox is checked but no yellow banner appears

**Diagnostic Steps:**

1. **Verify the setting was saved:**
   - Check in database: `SELECT value FROM settings WHERE key = 'dev_mode';`
   - Should return `1` or `true`

2. **Check React state:**
   - Open browser console
   - Type: `document.querySelector('input[type="checkbox"]').checked`
   - Should return `true`

3. **Verify banner code exists:**
   - Check [`Settings.tsx`](src/routes/Settings.tsx:116-130) lines 116-130
   - Look for `isDevMode` conditional rendering

---

### Issue: App fails to start after migration changes

**Symptoms:** Rust compilation errors or runtime crashes

**Diagnostic Steps:**

1. **Check migration SQL syntax:**
   ```bash
   cd src-tauri
   cargo check
   ```

2. **Verify migration file exists:**
   - [`src-tauri/migrations/002_settings.sql`](src-tauri/migrations/002_settings.sql)

3. **Rebuild the app:**
   ```bash
   pnpm tauri dev
   ```

**Nuclear Option (Data Loss):**
If database is corrupted:
```powershell
Remove-Item "$env:APPDATA\com.spectrail.dev\spectrail.sqlite"
```
Then restart the app (migrations will recreate tables).

---

### Issue: Database "locked" errors

**Symptoms:** Error messages about database being locked

**Solution:**
1. Close all instances of the app
2. Check for zombie processes in Task Manager
3. Restart the app

---

## Related Files Reference

| File | Purpose |
|------|---------|
| [`src-tauri/migrations/002_settings.sql`](src-tauri/migrations/002_settings.sql) | Database schema for settings table |
| [`src-tauri/src/db.rs`](src-tauri/src/db.rs) | Database initialization with migration |
| [`src-tauri/src/commands.rs`](src-tauri/src/commands.rs) | Backend commands for settings CRUD |
| [`src-tauri/src/models.rs`](src-tauri/src/models.rs) | Rust struct definitions |
| [`src/lib/api.ts`](src/lib/api.ts) | Frontend API functions |
| [`src/routes/Settings.tsx`](src/routes/Settings.tsx) | Settings UI component |
