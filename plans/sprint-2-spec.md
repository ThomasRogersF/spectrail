# Sprint 2: LLM Client + Test Connection

## Overview
Add an OpenAI-compatible LLM client with BYOK support, request/response logging to DB, and a "Test connection" button in Settings.

## Scope
- LLM client module with OpenAI-compatible API
- API key handling (keychain preferred, env var fallback)
- Request/response logging to SQLite (runs + messages tables)
- "Test connection" button in Settings UI
- No repo tools yet (Sprint 3)
- No tool calling loop yet (Sprint 3)
- No Plan/Verify workflows yet (Sprint 4)

---

## Backend Work (Rust)

### 1. Dependencies (Cargo.toml)

Add to `[dependencies]`:
```toml
reqwest = { version = "0.12", features = ["json", "rustls-tls"] }
tokio = { version = "1", features = ["rt-multi-thread"] }
backoff = { version = "0.4", features = ["tokio"] }
```

### 2. LLM Module Structure

**File**: `src-tauri/src/llm/mod.rs`
```rust
pub mod client;
pub mod types;

pub use client::{LlmClient, LlmConfig};
pub use types::{ChatMessage, OpenAIChatRequest, OpenAIChatResponse, Choice};
```

**File**: `src-tauri/src/llm/types.rs`
```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String, // "system", "user", "assistant"
    pub content: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct OpenAIChatRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<i64>,
    pub stream: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OpenAIChatResponse {
    pub id: String,
    pub model: String,
    pub choices: Vec<Choice>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Choice {
    pub index: i64,
    pub message: ChatMessage,
    pub finish_reason: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct LlmConfig {
    pub provider_name: String,
    pub base_url: String,
    pub model: String,
    pub temperature: f64,
    pub max_tokens: i64,
    pub extra_headers: serde_json::Value,
}

impl LlmConfig {
    pub fn from_settings(settings: &std::collections::HashMap<String, String>) -> Self {
        Self {
            provider_name: settings.get("provider_name").cloned().unwrap_or_default(),
            base_url: settings.get("base_url").cloned().unwrap_or_default(),
            model: settings.get("model").cloned().unwrap_or_default(),
            temperature: settings.get("temperature")
                .and_then(|s| s.parse().ok()).unwrap_or(0.7),
            max_tokens: settings.get("max_tokens")
                .and_then(|s| s.parse().ok()).unwrap_or(4096),
            extra_headers: settings.get("extra_headers_json")
                .and_then(|s| serde_json::from_str(s).ok())
                .unwrap_or(serde_json::json!({})),
        }
    }
}
```

**File**: `src-tauri/src/llm/client.rs`
```rust
use reqwest::{Client, StatusCode};
use std::time::Duration;
use backoff::{ExponentialBackoff, future::retry};

use crate::llm::types::*;

pub struct LlmClient {
    http: Client,
    config: LlmConfig,
    api_key: String,
}

#[derive(Debug, thiserror::Error)]
pub enum LlmError {
    #[error("Missing API key. Set SPECTRAIL_API_KEY environment variable or configure keychain storage.")]
    MissingApiKey,
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("API error {status}: {message}")]
    Api { status: u16, message: String },
    #[error("Invalid response: {0}")]
    InvalidResponse(String),
    #[error("Timeout")]
    Timeout,
    #[error("Rate limited")]
    RateLimited,
}

impl LlmClient {
    pub fn new(config: LlmConfig, api_key: String) -> Self {
        let http = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to build HTTP client");
        
        Self { http, config, api_key }
    }
    
    pub async fn chat(&self, messages: Vec<ChatMessage>) -> Result<String, LlmError> {
        if self.api_key.is_empty() {
            return Err(LlmError::MissingApiKey);
        }
        
        let request = OpenAIChatRequest {
            model: self.config.model.clone(),
            messages,
            temperature: Some(self.config.temperature),
            max_tokens: Some(self.config.max_tokens),
            stream: false,
        };
        
        let url = format!("{}/chat/completions", self.config.base_url.trim_end_matches('/'));
        
        // Retry logic with exponential backoff
        let operation = || async {
            let mut headers = reqwest::header::HeaderMap::new();
            headers.insert(
                reqwest::header::AUTHORIZATION,
                format!("Bearer {}", self.api_key).parse().unwrap(),
            );
            headers.insert(
                reqwest::header::CONTENT_TYPE,
                "application/json".parse().unwrap(),
            );
            
            // Add extra headers from config
            if let Some(obj) = self.config.extra_headers.as_object() {
                for (key, value) in obj {
                    if let Some(val_str) = value.as_str() {
                        if let (Ok(header_name), Ok(header_value)) = (
                            reqwest::header::HeaderName::from_bytes(key.as_bytes()),
                            val_str.parse::<reqwest::header::HeaderValue>()
                        ) {
                            headers.insert(header_name, header_value);
                        }
                    }
                }
            }
            
            let response = self.http
                .post(&url)
                .headers(headers)
                .json(&request)
                .send()
                .await?;
            
            let status = response.status();
            
            if status.is_success() {
                let chat_response: OpenAIChatResponse = response.json().await?;
                Ok(chat_response)
            } else {
                let error_text = response.text().await.unwrap_or_default();
                match status {
                    StatusCode::TOO_MANY_REQUESTS => Err(LlmError::RateLimited),
                    StatusCode::UNAUTHORIZED => Err(LlmError::Api { 
                        status: 401, 
                        message: "Invalid API key".to_string() 
                    }),
                    _ if status.as_u16() >= 500 => Err(LlmError::Api { 
                        status: status.as_u16(), 
                        message: error_text 
                    }),
                    _ => Err(LlmError::Api { 
                        status: status.as_u16(), 
                        message: error_text 
                    }),
                }
            }
        };
        
        let backoff = ExponentialBackoff {
            initial_interval: Duration::from_millis(500),
            max_interval: Duration::from_secs(4),
            max_elapsed_time: Some(Duration::from_secs(30)),
            ..Default::default()
        };
        
        let result: OpenAIChatResponse = retry(backoff, operation).await?;
        
        result.choices
            .into_iter()
            .next()
            .map(|c| c.message.content)
            .ok_or_else(|| LlmError::InvalidResponse("No choices in response".to_string()))
    }
    
    pub async fn test_connection(&self) -> Result<String, LlmError> {
        let messages = vec![
            ChatMessage {
                role: "user".to_string(),
                content: "Say 'OK' and nothing else.".to_string(),
            }
        ];
        self.chat(messages).await
    }
}
```

### 3. Settings Helper

**File**: `src-tauri/src/settings.rs` (new module)
```rust
use std::collections::HashMap;
use tauri::AppHandle;
use crate::db;
use crate::llm::LlmConfig;

pub fn get_all_settings(app: &AppHandle) -> Result<HashMap<String, String>, String> {
    let conn = db::connect(app).map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare("SELECT key, value FROM settings")
        .map_err(|e| e.to_string())?;
    
    let rows = stmt.query_map([], |r| {
        Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
    }).map_err(|e| e.to_string())?;
    
    let mut settings = HashMap::new();
    for row in rows {
        let (k, v) = row.map_err(|e| e.to_string())?;
        settings.insert(k, v);
    }
    Ok(settings)
}

pub fn get_llm_config(app: &AppHandle) -> Result<LlmConfig, String> {
    let settings = get_all_settings(app)?;
    Ok(LlmConfig::from_settings(&settings))
}

pub fn get_api_key() -> Result<String, String> {
    // Try environment variable first
    if let Ok(key) = std::env::var("SPECTRAIL_API_KEY") {
        return Ok(key);
    }
    
    // TODO: Add keychain retrieval in Sprint 2+ if implemented
    Ok(String::new())
}
```

### 4. System Project/Task for Test Runs

**File**: `src-tauri/src/settings.rs` (add to existing)
```rust
use crate::models::*;

const SYSTEM_PROJECT_ID: &str = "system-project";
const SYSTEM_TASK_ID: &str = "system-task-llm-test";

pub fn ensure_system_project_task(app: &AppHandle) -> Result<(String, String), String> {
    let conn = db::connect(app).map_err(|e| e.to_string())?;
    let now = now_iso();
    
    // Create system project if not exists
    conn.execute(
        "INSERT OR IGNORE INTO projects (id, name, repo_path, created_at, last_opened_at)
         VALUES (?1, ?2, ?3, ?4, NULL)",
        (SYSTEM_PROJECT_ID, "System", "__system__", &now)
    ).map_err(|e| e.to_string())?;
    
    // Create LLM test task if not exists
    conn.execute(
        "INSERT OR IGNORE INTO tasks (id, project_id, title, mode, status, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        (SYSTEM_TASK_ID, SYSTEM_PROJECT_ID, "LLM Connectivity", "plan", "active", &now, &now)
    ).map_err(|e| e.to_string())?;
    
    Ok((SYSTEM_PROJECT_ID.to_string(), SYSTEM_TASK_ID.to_string()))
}
```

### 5. New Tauri Commands

**File**: `src-tauri/src/commands.rs` (additions)
```rust
use crate::llm::{LlmClient, ChatMessage};
use crate::settings::{get_llm_config, get_api_key, ensure_system_project_task};

#[tauri::command]
pub async fn llm_test_connection(app: AppHandle) -> Result<String, String> {
    let config = get_llm_config(&app)?;
    let api_key = get_api_key()?;
    
    // Get or create system project/task for logging
    let (_, task_id) = ensure_system_project_task(&app)?;
    
    // Create a run for this test
    let run = create_run(app.clone(), task_id, "test".to_string()).await?;
    
    let client = LlmClient::new(config, api_key);
    
    // Log user message
    add_message(app.clone(), run.id.clone(), "user".to_string(),
        "Say 'OK' and nothing else.".to_string()).await?;
    
    let response = client.test_connection().await
        .map_err(|e| e.to_string())?;
    
    // Log assistant response
    add_message(app, run.id, "assistant".to_string(), response.clone()).await?;
    
    Ok(response)
}

// Helper: async version of create_run
async fn create_run(app: AppHandle, task_id: String, run_type: String) -> Result<Run, String> {
    let config = get_llm_config(&app)?;
    
    tokio::task::spawn_blocking(move || {
        let conn = db::connect(&app).map_err(|e| e.to_string())?;
        let id = new_id();
        let started_at = now_iso();
        conn.execute(
            "INSERT INTO runs (id, task_id, phase_id, run_type, provider, model, started_at, ended_at)
             VALUES (?1, ?2, NULL, ?3, ?4, ?5, ?6, NULL)",
            (&id, &task_id, &run_type, &config.provider_name, &config.model, &started_at)
        ).map_err(|e| e.to_string())?;
        Ok(Run {
            id, task_id, phase_id: None, run_type,
            provider: Some(config.provider_name),
            model: Some(config.model),
            started_at, ended_at: None
        })
    }).await.map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn llm_simple_chat(
    app: AppHandle,
    run_id: String,
    user_message: String
) -> Result<String, String> {
    let config = get_llm_config(&app)?;
    let api_key = get_api_key()?;
    
    // Log user message
    add_message(app.clone(), run_id.clone(), "user".to_string(), user_message.clone()).await?;
    
    let client = LlmClient::new(config, api_key);
    let messages = vec![
        ChatMessage {
            role: "user".to_string(),
            content: user_message,
        }
    ];
    
    let response = client.chat(messages).await
        .map_err(|e| e.to_string())?;
    
    // Log assistant response
    add_message(app, run_id, "assistant".to_string(), response.clone()).await?;
    
    Ok(response)
}

// Helper to make add_message async-compatible
async fn add_message(app: AppHandle, run_id: String, role: String, content: String) -> Result<(), String> {
    // This runs on the main thread via spawn_blocking or tauri's async runtime
    tokio::task::spawn_blocking(move || {
        let conn = db::connect(&app).map_err(|e| e.to_string())?;
        let id = new_id();
        let created_at = now_iso();
        conn.execute(
            "INSERT INTO messages (id, run_id, role, content, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            (&id, &run_id, &role, &content, &created_at)
        ).map_err(|e| e.to_string())?;
        Ok(())
    }).await.map_err(|e| e.to_string())?
}
```

### 5. Update Module Exports

**File**: `src-tauri/src/lib.rs`
```rust
mod commands;
mod db;
mod models;
mod llm;
mod settings;
```

Add to invoke_handler:
```rust
commands::llm_test_connection,
commands::llm_simple_chat,
```

---

## Frontend Work (React)

### 1. API Wrapper

**File**: `src/lib/api.ts` (additions)
```typescript
export async function llmTestConnection(): Promise<string> {
  return invoke("llm_test_connection");
}

export async function llmSimpleChat(runId: string, userMessage: string): Promise<string> {
  return invoke("llm_simple_chat", { runId, userMessage });
}
```

### 2. Settings UI Updates

**File**: `src/routes/Settings.tsx` (additions)

Add to the form:
```typescript
const [apiKey, setApiKey] = useState("");
const [testStatus, setTestStatus] = useState<"idle" | "testing" | "success" | "error">("idle");
const [testResult, setTestResult] = useState("");
const [testError, setTestError] = useState("");
const [showRawError, setShowRawError] = useState(false);

// Check if API key is configured (env var)
useEffect(() => {
  // For now, just check if we have a saved key or env var indication
  // In Sprint 2+, this would check keychain
  getSetting("api_key_source").then(source => {
    if (source === "env") {
      setApiKey("[Set via SPECTRAIL_API_KEY environment variable]");
    }
  });
}, []);

// Test connection handler
async function handleTestConnection() {
  setTestStatus("testing");
  setTestResult("");
  setTestError("");
  setShowRawError(false);
  
  try {
    const result = await llmTestConnection();
    setTestResult(result);
    setTestStatus("success");
  } catch (err: any) {
    setTestError(err?.toString?.() || String(err));
    setTestStatus("error");
  }
}
```

Add to the form UI:
```tsx
{/* API Key Section */}
<div style={{ marginTop: 24, padding: 16, background: "#f8f9fa", borderRadius: 8 }}>
  <h3 style={{ marginTop: 0, marginBottom: 12 }}>API Key</h3>
  
  {apiKey.startsWith("[") ? (
    <div style={{ color: "#28a745", fontSize: 14 }}>
      ✓ {apiKey}
    </div>
  ) : (
    <div>
      <div style={{ color: "#dc3545", fontSize: 14, marginBottom: 8 }}>
        ⚠️ API key not configured
      </div>
      <div style={{ fontSize: 13, color: "#666" }}>
        Set the <code>SPECTRAIL_API_KEY</code> environment variable to your API key.
        <br />
        Example: <code>SPECTRAIL_API_KEY=sk-... pnpm tauri dev</code>
      </div>
    </div>
  )}
</div>

{/* Test Connection Section */}
<div style={{ marginTop: 24 }}>
  <button
    onClick={handleTestConnection}
    disabled={testStatus === "testing"}
    style={{
      padding: "10px 20px",
      borderRadius: 8,
      border: "none",
      background: testStatus === "success" ? "#28a745" : "#007bff",
      color: "white",
      fontSize: 14,
      fontWeight: 600,
      cursor: testStatus === "testing" ? "not-allowed" : "pointer",
      opacity: testStatus === "testing" ? 0.7 : 1,
    }}
  >
    {testStatus === "testing" ? "Testing..." : "Test Connection"}
  </button>
  
  {testStatus === "success" && (
    <div style={{ marginTop: 12, padding: 12, background: "#d4edda", borderRadius: 8, color: "#155724" }}>
      <strong>✓ Connection successful</strong>
      <div style={{ marginTop: 4, fontSize: 13 }}>Response: {testResult}</div>
    </div>
  )}
  
  {testStatus === "error" && (
    <div style={{ marginTop: 12, padding: 12, background: "#f8d7da", borderRadius: 8, color: "#721c24" }}>
      <strong>✗ Connection failed</strong>
      <div style={{ marginTop: 4, fontSize: 13 }}>{testError}</div>
      <button
        onClick={() => setShowRawError(!showRawError)}
        style={{
          marginTop: 8,
          fontSize: 12,
          background: "transparent",
          border: "none",
          color: "#721c24",
          textDecoration: "underline",
          cursor: "pointer",
        }}
      >
        {showRawError ? "Hide" : "Show"} raw error
      </button>
      {showRawError && (
        <pre style={{ marginTop: 8, fontSize: 11, overflow: "auto" }}>
          {testError}
        </pre>
      )}
    </div>
  )}
</div>
```

---

## System Project/Task (No DB Migration Needed)

Instead of making `runs.task_id` nullable, we create a hidden "System" project with an "LLM Connectivity" task. This approach:
- ✅ No schema changes required
- ✅ Test runs are logged normally with valid task_id
- ✅ Runs appear in the System project (can be hidden from UI if desired)

**Implementation**: See `ensure_system_project_task()` in settings.rs section above.

**System IDs**:
- Project ID: `system-project`
- Task ID: `system-task-llm-test`

These are created on first test connection using `INSERT OR IGNORE`, so they're idempotent.

---

## README Update

Add to README.md under Sprint 2 section:

```markdown
## Sprint 2: LLM Client + Test Connection (Completed)

### Features
- OpenAI-compatible LLM client with retry logic
- API key via SPECTRAIL_API_KEY environment variable
- "Test Connection" button in Settings
- Exponential backoff retry (0.5s, 1s, 2s, 4s)
- Request/response structure for future logging

### Configuration
Set your API key as environment variable:
```bash
SPECTRAIL_API_KEY=sk-... pnpm tauri dev
```

Or on Windows:
```powershell
$env:SPECTRAIL_API_KEY="sk-..."
pnpm tauri dev
```

### New Commands
- `llm_test_connection()` - Tests provider connectivity
- `llm_simple_chat(run_id, message)` - Chat with logging
```

---

## Acceptance Criteria Checklist

- [ ] User can enter base_url/model in Settings
- [ ] "Test connection" button returns success/failure
- [ ] Missing API key shows clear instructions (env var)
- [ ] Invalid API key shows readable error
- [ ] Connection errors show retry behavior
- [ ] Successful connection returns assistant response
- [ ] No API key stored in SQLite (env var only in Sprint 2)
- [ ] Code structure supports tool calling (Sprint 3)
- [ ] Retry logic implemented with exponential backoff
- [ ] Extra headers from settings are sent with requests
