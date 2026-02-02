

---

## C) Schemas + Dispatcher (Sprint 4 Readiness)

### Tool Schemas (OpenAI-compatible)

**File**: `src-tauri/src/repo_tools/schemas.rs`
```rust
use serde_json::{json, Value};

pub fn repo_tool_schemas() -> Vec<Value> {
  vec![
    list_files_schema(),
    read_file_schema(),
    grep_schema(),
    git_status_schema(),
    git_diff_schema(),
    git_log_short_schema(),
    run_command_schema(),
  ]
}

fn list_files_schema() -> Value {
  json!({
    "type": "function",
    "function": {
      "name": "list_files",
      "description": "List files in the repository, respecting .gitignore. Returns relative paths.",
      "parameters": {
        "type": "object",
        "properties": {
          "project_id": {
            "type": "string",
            "description": "Project ID to operate on"
          },
          "globs": {
            "type": "array",
            "items": { "type": "string" },
            "description": "Optional glob patterns to filter files"
          },
          "max_files": {
            "type": "integer",
            "description": "Maximum files to return (default 2000)"
          }
        },
        "required": ["project_id"]
      }
    }
  })
}

fn read_file_schema() -> Value {
  json!({
    "type": "function",
    "function": {
      "name": "read_file",
      "description": "Read contents of a file within the repository. Large files are truncated.",
      "parameters": {
        "type": "object",
        "properties": {
          "project_id": {
            "type": "string",
            "description": "Project ID"
          },
          "path": {
            "type": "string",
            "description": "Relative path to file within repo"
          },
          "max_bytes": {
            "type": "integer",
            "description": "Max bytes to read (default 200000)"
          }
        },
        "required": ["project_id", "path"]
      }
    }
  })
}

fn grep_schema() -> Value {
  json!({
    "type": "function",
    "function": {
      "name": "grep",
      "description": "Search for text patterns in repository files. Uses ripgrep if available.",
      "parameters": {
        "type": "object",
        "properties": {
          "project_id": {
            "type": "string",
            "description": "Project ID"
          },
          "query": {
            "type": "string",
            "description": "Search pattern"
          },
          "path": {
            "type": "string",
            "description": "Optional subdirectory to search within"
          },
          "max_results": {
            "type": "integer",
            "description": "Max matches to return (default 200)"
          }
        },
        "required": ["project_id", "query"]
      }
    }
  })
}

fn git_status_schema() -> Value {
  json!({
    "type": "function",
    "function": {
      "name": "git_status",
      "description": "Get git status of the repository including branch info.",
      "parameters": {
        "type": "object",
        "properties": {
          "project_id": {
            "type": "string",
            "description": "Project ID"
          }
        },
        "required": ["project_id"]
      }
    }
  })
}

fn git_diff_schema() -> Value {
  json!({
    "type": "function",
    "function": {
      "name": "git_diff",
      "description": "Get git diff of unstaged or staged changes.",
      "parameters": {
        "type": "object",
        "properties": {
          "project_id": {
            "type": "string",
            "description": "Project ID"
          },
          "staged": {
            "type": "boolean",
            "description": "Show staged changes instead of unstaged"
          }
        },
        "required": ["project_id"]
      }
    }
  })
}

fn git_log_short_schema() -> Value {
  json!({
    "type": "function",
    "function": {
      "name": "git_log_short",
      "description": "Get recent commit history in concise format.",
      "parameters": {
        "type": "object",
        "properties": {
          "project_id": {
            "type": "string",
            "description": "Project ID"
          },
          "max_commits": {
            "type": "integer",
            "description": "Number of commits to retrieve (default 10)"
          }
        },
        "required": ["project_id"]
      }
    }
  })
}

fn run_command_schema() -> Value {
  json!({
    "type": "function",
    "function": {
      "name": "run_command",
      "description": "Run allowlisted test, lint, or build commands. Auto-detects package manager.",
      "parameters": {
        "type": "object",
        "properties": {
          "project_id": {
            "type": "string",
            "description": "Project ID"
          },
          "kind": {
            "type": "string",
            "enum": ["tests", "lint", "build"],
            "description": "Type of command to run"
          },
          "runner": {
            "type": "string",
            "enum": ["pnpm", "npm", "yarn", "cargo", "pytest"],
            "description": "Optional explicit runner (auto-detected if not provided)"
          }
        },
        "required": ["project_id", "kind"]
      }
    }
  })
}
```

### Dispatcher

**File**: `src-tauri/src/repo_tools/dispatcher.rs`
```rust
use serde_json::Value;
use std::path::Path;
use tauri::AppHandle;

use crate::repo_tools::fs::{list_files, read_file};
use crate::repo_tools::search::grep;
use crate::repo_tools::git::{git_status, git_diff, git_log_short};
use crate::repo_tools::runner::run_command;
use crate::repo_tools::schemas::repo_tool_schemas;

pub use crate::repo_tools::schemas::repo_tool_schemas;

pub async fn dispatch_repo_tool(
  name: &str,
  args: &Value,
  repo_path: &Path,
  app: &AppHandle,
  run_id: &str,
) -> Result<Value, String> {
  match name {
    "list_files" => list_files(repo_path, args, app, run_id).await,
    "read_file" => read_file(repo_path, args, app, run_id).await,
    "grep" => grep(repo_path, args, app, run_id).await,
    "git_status" => git_status(repo_path, args, app, run_id).await,
    "git_diff" => git_diff(repo_path, args, app, run_id).await,
    "git_log_short" => git_log_short(repo_path, args, app, run_id).await,
    "run_command" => run_command(repo_path, args, app, run_id).await,
    _ => Err(format!("Unknown tool: {}", name)),
  }
}
```

### Module Exports

**File**: `src-tauri/src/repo_tools/mod.rs`
```rust
pub mod dispatcher;
pub mod fs;
pub mod git;
pub mod logging;
pub mod runner;
pub mod safety;
pub mod schemas;
pub mod search;

pub use dispatcher::{dispatch_repo_tool, repo_tool_schemas};
pub use logging::{list_tool_calls, log_tool_call};
```

---

## D) Frontend (UI)

### 1. API Wrapper

**File**: `src/lib/api.ts` (additions)
```typescript
export async function listToolCalls(runId: string): Promise<Array<{
  id: string;
  run_id: string;
  name: string;
  args_json: string;
  result_json: string;
  created_at: string;
}>> {
  return invoke("list_tool_calls", { runId });
}

export async function executeRepoTool(
  projectId: string,
  runId: string,
  name: string,
  args: Record<string, unknown>
): Promise<unknown> {
  return invoke("execute_repo_tool", { projectId, runId, name, args });
}

export async function getRepoToolSchemas(): Promise<Array<{
  type: string;
  function: {
    name: string;
    description: string;
    parameters: unknown;
  };
}>> {
  return invoke("get_repo_tool_schemas");
}
```

### 2. Tool Call Display Component

**File**: `src/components/ToolCallList.tsx`
```typescript
import React, { useState } from "react";

interface ToolCall {
  id: string;
  name: string;
  args_json: string;
  result_json: string;
  created_at: string;
}

interface ToolCallListProps {
  toolCalls: ToolCall[];
}

export default function ToolCallList({ toolCalls }: ToolCallListProps) {
  const [expanded, setExpanded] = useState<Set<string>>(new Set());

  function toggle(id: string) {
    const next = new Set(expanded);
    if (next.has(id)) {
      next.delete(id);
    } else {
      next.add(id);
    }
    setExpanded(next);
  }

  if (toolCalls.length === 0) {
    return <div style={{ opacity: 0.6, fontSize: 14 }}>No tool calls</div>;
  }

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
      {toolCalls.map((tc) => {
        const isExpanded = expanded.has(tc.id);
        const args = JSON.parse(tc.args_json);
        const result = JSON.parse(tc.result_json);

        return (
          <div
            key={tc.id}
            style={{
              border: "1px solid #e0e0e0",
              borderRadius: 8,
              overflow: "hidden",
            }}
          >
            <button
              onClick={() => toggle(tc.id)}
              style={{
                width: "100%",
                padding: "8px 12px",
                textAlign: "left",
                background: "#f8f9fa",
                border: "none",
                cursor: "pointer",
                display: "flex",
                justifyContent: "space-between",
                alignItems: "center",
              }}
            >
              <span style={{ fontWeight: 600 }}>🔧 {tc.name}</span>
              <span style={{ fontSize: 12, opacity: 0.6 }}>
                {isExpanded ? "▲" : "▼"}
              </span>
            </button>

            {isExpanded && (
              <div style={{ padding: 12 }}>
                <div style={{ marginBottom: 12 }}>
                  <div style={{ fontSize: 11, textTransform: "uppercase", opacity: 0.6, marginBottom: 4 }}>
                    Arguments
                  </div>
                  <pre
                    style={{
                      margin: 0,
                      fontSize: 12,
                      background: "#f4f4f4",
                      padding: 8,
                      borderRadius: 4,
                      overflow: "auto",
                    }}
                  >
                    {JSON.stringify(args, null, 2)}
                  </pre>
                </div>

                <div>
                  <div style={{ fontSize: 11, textTransform: "uppercase", opacity: 0.6, marginBottom: 4 }}>
                    Result
                  </div>
                  <pre
                    style={{
                      margin: 0,
                      fontSize: 12,
                      background: "#f4f4f4",
                      padding: 8,
                      borderRadius: 4,
                      overflow: "auto",
                      maxHeight: 400,
                    }}
                  >
                    {JSON.stringify(result, null, 2)}
                  </pre>
                </div>
              </div>
            )}
          </div>
        );
      })}
    </div>
  );
}
```

### 3. Update RunDetail to Show Tool Calls

**File**: `src/routes/RunDetail.tsx` (modifications)
```typescript
import { useEffect, useState } from "react";
import { useParams } from "react-router-dom";
import { listMessages, listToolCalls } from "../lib/api";
import ToolCallList from "../components/ToolCallList";
// ... other imports

export default function RunDetail() {
  const { runId } = useParams<{ runId: string }>();
  const [messages, setMessages] = useState<Message[]>([]);
  const [toolCalls, setToolCalls] = useState<ToolCall[]>([]);
  const [activeTab, setActiveTab] = useState<"messages" | "tools">("messages");

  useEffect(() => {
    if (!runId) return;
    (async () => {
      setMessages(await listMessages(runId));
      setToolCalls(await listToolCalls(runId));
    })();
  }, [runId]);

  return (
    <div style={{ maxWidth: 980 }}>
      <h1>Run Detail</h1>
      
      {/* Tab navigation */}
      <div style={{ display: "flex", gap: 8, marginBottom: 16, borderBottom: "1px solid #eee" }}>
        <button
          onClick={() => setActiveTab("messages")}
          style={{
            padding: "8px 16px",
            border: "none",
            background: "transparent",
            borderBottom: activeTab === "messages" ? "2px solid #007bff" : "none",
            cursor: "pointer",
          }}
        >
          Messages ({messages.length})
        </button>
        <button
          onClick={() => setActiveTab("tools")}
          style={{
            padding: "8px 16px",
            border: "none",
            background: "transparent",
            borderBottom: activeTab === "tools" ? "2px solid #007bff" : "none",
            cursor: "pointer",
          }}
        >
          Tool Calls ({toolCalls.length})
        </button>
      </div>

      {activeTab === "messages" && (
        <div>{/* existing message rendering */}</div>
      )}

      {activeTab === "tools" && <ToolCallList toolCalls={toolCalls} />}
    </div>
  );
}
```

### 4. Dev Playground (Optional, dev_mode only)

**File**: `src/components/DevToolPlayground.tsx`
```typescript
import React, { useState } from "react";
import { executeRepoTool } from "../lib/api";

interface Props {
  projectId: string;
  runId: string;
}

export default function DevToolPlayground({ projectId, runId }: Props) {
  const [tool, setTool] = useState("list_files");
  const [args, setArgs] = useState("{}");
  const [result, setResult] = useState<unknown>(null);
  const [loading, setLoading] = useState(false);

  async function execute() {
    setLoading(true);
    try {
      const parsed = JSON.parse(args);
      const res = await executeRepoTool(projectId, runId, tool, parsed);
      setResult(res);
    } catch (err) {
      setResult({ error: String(err) });
    } finally {
      setLoading(false);
    }
  }

  return (
    <div style={{ padding: 16, background: "#fff3cd", borderRadius: 8 }}>
      <h3>🛠️ Dev Tool Playground</h3>
      
      <div style={{ display: "flex", gap: 8, marginBottom: 12 }}>
        <select value={tool} onChange={(e) => setTool(e.target.value)}>
          <option value="list_files">list_files</option>
          <option value="read_file">read_file</option>
          <option value="grep">grep</option>
          <option value="git_status">git_status</option>
          <option value="git_diff">git_diff</option>
          <option value="run_command">run_command</option>
        </select>
        
        <button onClick={execute} disabled={loading}>
          {loading ? "Running..." : "Execute"}
        </button>
      </div>

      <textarea
        value={args}
        onChange={(e) => setArgs(e.target.value)}
        rows={4}
        style={{ width: "100%", fontFamily: "monospace", marginBottom: 12 }}
        placeholder='{"path": "src/main.rs"}'
      />

      {result && (
        <pre style={{ background: "#f4f4f4", padding: 12, overflow: "auto", maxHeight: 400 }}>
          {JSON.stringify(result, null, 2)}
        </pre>
      )}
    </div>
  );
}
```

---

## E) Integration Steps

### 1. Update Cargo.toml

```toml
[dependencies]
# Existing: reqwest, tokio, backoff, etc.

# New for Sprint 3
ignore = "0.4"
which = "6"
walkdir = "2"
```

### 2. Update lib.rs

```rust
mod commands;
mod db;
mod llm;
mod models;
mod repo_tools;  // NEW
mod settings;

// In invoke_handler, add:
// commands::list_tool_calls,
// commands::execute_repo_tool,
// commands::get_repo_tool_schemas,
```

### 3. DB Migration (if tool_calls table missing)

**File**: `src-tauri/migrations/003_tool_calls.sql` (only if needed)
```sql
CREATE TABLE IF NOT EXISTS tool_calls (
  id TEXT PRIMARY KEY,
  run_id TEXT NOT NULL,
  name TEXT NOT NULL,
  args_json TEXT NOT NULL,
  result_json TEXT NOT NULL,
  created_at TEXT NOT NULL,
  FOREIGN KEY(run_id) REFERENCES runs(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_tool_calls_run ON tool_calls(run_id);
```

---

## Acceptance Criteria

- [ ] `list_files`: gitignore-aware, excludes junk, returns relative paths
- [ ] `read_file`: path traversal blocked, large files truncated, binary detection
- [ ] `grep`: ripgrep preferred with fallback, results limited
- [ ] `git_status`, `git_diff`, `git_log_short`: functional with truncation
- [ ] `run_command`: strict allowlist, no arbitrary commands, timeout handled
- [ ] Every tool call logged to `tool_calls` table
- [ ] RunDetail shows tool calls in collapsible UI
- [ ] Schemas match OpenAI function calling format
- [ ] Dispatcher ready for Sprint 4 tool loop

---

## Implementation Order

1. ✅ Tool call logging + `list_tool_calls` command
2. ✅ Safety utilities (`sanitize_path`, `truncate_string`, `safe_spawn`)
3. ✅ File tools (`list_files`, `read_file`)
4. ✅ Search tool (`grep` with ripgrep fallback)
5. ✅ Git tools (`git_status`, `git_diff`, `git_log_short`)
6. ✅ Runner with allowlist (`run_command`)
7. ✅ Schemas + dispatcher
8. ✅ Frontend: API wrappers + ToolCallList component
9. ✅ Frontend: Update RunDetail with tabs
10. ⬜ Optional: Dev playground
11. ⬜ README update
