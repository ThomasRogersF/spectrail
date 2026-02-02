# Sprint 4: Plan Mode (LLM Tool-Call Loop)

## Overview
Add end-to-end Plan Mode functionality: generate a detailed implementation plan using LLM with repo tool calls.

## Architecture

```
TaskDetail.tsx
    ↓ click "Generate Plan"
generate_plan(project_id, task_id)
    ↓
Workflow Engine:
  1. Create run (type="plan")
  2. Build system prompt + user message
  3. Fetch tool schemas
  4. LLM request with tools
  5. Tool-call loop (max 12 iterations)
     - Execute tool via execute_repo_tool
     - Log tool result as message
     - Continue
  6. Final plan → artifact (kind="plan_md")
    ↓
Return { run_id, plan_md }
    ↓
UI: Display plan, copy button, link to run
```

---

## A) Backend: Plan Workflow Engine

### Module Structure

**File**: `src-tauri/src/workflows/mod.rs`
```rust
pub mod plan;
```

**File**: `src-taurus/src/workflows/plan.rs`

```rust
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tauri::AppHandle;
use std::collections::HashMap;

use crate::commands::{create_run, add_message, upsert_artifact, get_task, get_project, get_settings};
use crate::repo_tools::{get_repo_tool_schemas, execute_repo_tool};
use crate::models::*;
use crate::llm::{LlmClient, ChatMessage, LlmConfig};

const MAX_TOOL_ITERATIONS: usize = 12;
const MAX_CONTEXT_CHARS: usize = 100_000;

#[derive(Debug, Serialize)]
pub struct PlanResult {
    pub run_id: String,
    pub plan_md: String,
    pub tool_calls_count: usize,
    pub truncated: bool,
}

#[derive(Debug, Serialize)]
pub struct PlanError {
    pub code: String,
    pub message: String,
}

pub async fn generate_plan(
    app: AppHandle,
    project_id: String,
    task_id: String,
) -> Result<PlanResult, PlanError> {
    // 1. Get task and project info
    let task = get_task(app.clone(), task_id.clone())
        .map_err(|e| PlanError { code: "TASK_NOT_FOUND".into(), message: e })?;
    
    let project = get_project(app.clone(), project_id.clone())
        .map_err(|e| PlanError { code: "PROJECT_NOT_FOUND".into(), message: e })?;
    
    // 2. Get settings for LLM
    let settings = get_all_settings(&app)?;
    let llm_config = build_llm_config(&settings);
    let api_key = get_api_key()?;
    
    // 3. Create run
    let run = create_run_plan(&app, task_id.clone()).await?;
    let run_id = run.id.clone();
    
    // 4. Build messages
    let mut messages = build_initial_messages(&task, &project);
    
    // 5. Get tool schemas
    let tools = get_repo_tool_schemas();
    
    // 6. Tool-call loop
    let client = LlmClient::new(llm_config, api_key);
    let mut tool_calls_count = 0;
    let mut truncated = false;
    let mut final_plan = String::new();
    
    for iteration in 0..MAX_TOOL_ITERATIONS {
        // Log context size check
        let context_size: usize = messages.iter()
            .map(|m| m.content.as_ref().map_or(0, |c| c.len()))
            .sum();
        
        if context_size > MAX_CONTEXT_CHARS {
            truncated = true;
            // Truncate oldest messages
            messages = truncate_messages(messages, MAX_CONTEXT_CHARS);
        }
        
        // Call LLM
        let response = client.chat_with_tools(messages.clone(), tools.clone()).await
            .map_err(|e| PlanError { code: "LLM_ERROR".into(), message: e.to_string() })?;
        
        // Log assistant message
        if let Some(content) = &response.content {
            add_message(app.clone(), run_id.clone(), "assistant".into(), content.clone())
                .await
                .map_err(|e| PlanError { code: "LOG_ERROR".into(), message: e })?;
        }
        
        // Check for tool calls
        if let Some(tool_calls) = response.tool_calls {
            if tool_calls.is_empty() {
                // No more tools, we have final plan
                final_plan = response.content.unwrap_or_default();
                break;
            }
            
            tool_calls_count += tool_calls.len();
            
            // Execute each tool call
            for tool_call in tool_calls {
                let tool_result = execute_single_tool(
                    &app,
                    &run_id,
                    &project_id,
                    &tool_call,
                ).await?;
                
                // Add tool result as message
                let tool_message = ChatMessage {
                    role: "tool".into(),
                    content: Some(tool_result),
                    tool_call_id: Some(tool_call.id.clone()),
                    tool_calls: None,
                };
                
                messages.push(tool_message.clone());
                
                // Log to database
                add_message(
                    app.clone(),
                    run_id.clone(),
                    "tool".into(),
                    tool_message.content.unwrap_or_default(),
                ).await.ok(); // Don't fail on log error
            }
        } else {
            // No tool calls, we have final plan
            final_plan = response.content.unwrap_or_default();
            break;
        }
        
        // Add assistant message to context for next iteration
        messages.push(ChatMessage {
            role: "assistant".into(),
            content: response.content,
            tool_call_id: None,
            tool_calls: response.tool_calls,
        });
    }
    
    // If we hit max iterations, add a note
    if tool_calls_count >= MAX_TOOL_ITERATIONS && final_plan.is_empty() {
        final_plan = format!(
            "{plan}\n\n---\n\n**Note**: Reached maximum tool call limit ({max}). Plan may be incomplete.\nConsider breaking this task into smaller phases.",
            plan = final_plan,
            max = MAX_TOOL_ITERATIONS
        );
        truncated = true;
    }
    
    // 7. Save plan artifact
    upsert_artifact(
        app.clone(),
        task_id,
        None,
        "plan_md".into(),
        final_plan.clone(),
    ).await
    .map_err(|e| PlanError { code: "ARTIFACT_ERROR".into(), message: e })?;
    
    Ok(PlanResult {
        run_id,
        plan_md: final_plan,
        tool_calls_count,
        truncated,
    })
}

fn build_initial_messages(task: &Task, project: &Project) -> Vec<ChatMessage> {
    let system_prompt = r#"You are a senior technical lead creating detailed implementation plans.

Your task: Analyze the codebase and produce a comprehensive implementation plan.

Required output format (Markdown):

# Implementation Plan: [Title]

## 1. Summary
Brief overview of the approach (2-3 sentences).

## 2. Goals & Non-Goals
**Goals:**
- What this implementation achieves

**Non-Goals:**
- What is explicitly out of scope

## 3. Repo Context Assumptions
- Key files/modules that exist
- Dependencies to leverage

## 4. File-by-File Changes
For each file to modify/create:
- **Path**: relative path
- **Purpose**: what this file does
- **Key Changes**: specific modifications

## 5. Step-by-Step Implementation Checklist
- [ ] Step 1: ...
- [ ] Step 2: ...
(Ordered by dependency, earliest first)

## 6. Risks + Mitigations
| Risk | Mitigation |
|------|------------|
| Risk description | How to address it |

## 7. Validation Steps
- [ ] Tests: `run_command` with kind="tests"
- [ ] Lint: `run_command` with kind="lint"
- [ ] Build: `run_command` with kind="build"

---

Instructions:
1. Use the provided tools to explore the codebase before writing the plan
2. Call `list_files` to understand the project structure
3. Call `read_file` to examine key files
4. Call `grep` to find relevant code patterns
5. Call `git_status` and `git_diff` to see current state
6. Only write the plan after gathering sufficient context
7. If you need more information, make another tool call
8. When complete, output ONLY the plan in the format above (no tool calls in final output)
"#;

    let user_prompt = format!(
        r#"Task: {title}

Repository: {repo_path}

Please explore this codebase and create a detailed implementation plan.

Available tools:
- list_files: See project structure
- read_file: Examine file contents
- grep: Search for patterns
- git_status/git_diff: Check current changes
- run_command: Run tests/lint/build

Start by listing files to understand the project structure, then read key files to understand the codebase before writing your plan."#,
        title = task.title,
        repo_path = project.repo_path,
    );

    vec![
        ChatMessage {
            role: "system".into(),
            content: Some(system_prompt.into()),
            tool_call_id: None,
            tool_calls: None,
        },
        ChatMessage {
            role: "user".into(),
            content: Some(user_prompt),
            tool_call_id: None,
            tool_calls: None,
        },
    ]
}

async fn execute_single_tool(
    app: &AppHandle,
    run_id: &str,
    project_id: &str,
    tool_call: &ToolCall,
) -> Result<String, PlanError> {
    // Parse args
    let args: Value = serde_json::from_str(&tool_call.function.arguments)
        .map_err(|e| PlanError {
            code: "INVALID_ARGS".into(),
            message: format!("Failed to parse tool args: {}", e),
        })?;
    
    // Add project_id to args if not present
    let mut args_with_project = args.clone();
    if let Some(obj) = args_with_project.as_object_mut() {
        obj.entry("project_id".to_string())
            .or_insert_with(|| json!(project_id));
    }
    
    // Execute tool
    let result = execute_repo_tool(
        app.clone(),
        run_id.to_string(),
        project_id.to_string(),
        tool_call.function.name.clone(),
        args_with_project,
    ).await;
    
    match result {
        Ok(val) => Ok(val.to_string()),
        Err(e) => Ok(json!({ "error": e }).to_string()),
    }
}

async fn create_run_plan(app: &AppHandle, task_id: String) -> Result<Run, PlanError> {
    // Use spawn_blocking for sync DB operations
    let app = app.clone();
    let result = tokio::task::spawn_blocking(move || {
        create_run(app, task_id, "plan".into())
    }).await;
    
    match result {
        Ok(Ok(run)) => Ok(run),
        Ok(Err(e)) => Err(PlanError { code: "RUN_CREATE_ERROR".into(), message: e }),
        Err(e) => Err(PlanError { code: "RUN_CREATE_ERROR".into(), message: e.to_string() }),
    }
}

fn get_all_settings(app: &AppHandle) -> Result<HashMap<String, String>, PlanError> {
    // Implementation from settings module
    todo!()
}

fn build_llm_config(settings: &HashMap<String, String>) -> LlmConfig {
    LlmConfig {
        provider_name: settings.get("provider_name").cloned().unwrap_or_default(),
        base_url: settings.get("base_url").cloned().unwrap_or_default(),
        model: settings.get("model").cloned().unwrap_or_default(),
        temperature: settings.get("temperature")
            .and_then(|s| s.parse().ok()).unwrap_or(0.2),
        max_tokens: settings.get("max_tokens")
            .and_then(|s| s.parse().ok()).unwrap_or(4000),
        extra_headers: settings.get("extra_headers_json")
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_else(|| json!({})),
    }
}

fn get_api_key() -> Result<String, PlanError> {
    std::env::var("SPECTRAIL_API_KEY")
        .map_err(|_| PlanError {
            code: "NO_API_KEY".into(),
            message: "SPECTRAIL_API_KEY environment variable not set".into(),
        })
}

fn truncate_messages(messages: Vec<ChatMessage>, max_chars: usize) -> Vec<ChatMessage> {
    // Keep system message and most recent messages
    if messages.len() < 3 {
        return messages;
    }
    
    let system = messages.first().cloned();
    let recent: Vec<_> = messages.into_iter().rev().take(6).rev().collect();
    
    let mut result = Vec::new();
    if let Some(sys) = system {
        result.push(sys);
    }
    result.extend(recent);
    result
}

// Types for LLM tool calling
#[derive(Debug, Clone)]
pub struct ToolCall {
    pub id: String,
    pub function: ToolFunction,
}

#[derive(Debug, Clone)]
pub struct ToolFunction {
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Clone)]
pub struct LlmResponse {
    pub content: Option<String>,
    pub tool_calls: Option<Vec<ToolCall>>,
}
```

---

## B) Backend: LLM Client Updates

**File**: `src-tauri/src/llm/types.rs` (additions)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    #[serde(rename = "type")]
    pub call_type: String,
    pub function: ToolFunction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolFunction {
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct OpenAIChatRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Value>>,
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
```

**File**: `src-tauri/src/llm/client.rs` (add method)

```rust
impl LlmClient {
    pub async fn chat_with_tools(
        &self,
        messages: Vec<ChatMessage>,
        tools: Vec<Value>,
    ) -> Result<LlmResponse, LlmError> {
        let request = OpenAIChatRequest {
            model: self.config.model.clone(),
            messages,
            tools: Some(tools),
            temperature: Some(self.config.temperature),
            max_tokens: Some(self.config.max_tokens),
            stream: false,
        };
        
        // ... existing request building code ...
        
        let response: OpenAIChatResponse = retry(backoff, operation).await?;
        
        if let Some(choice) = response.choices.into_iter().next() {
            Ok(LlmResponse {
                content: choice.message.content,
                tool_calls: choice.message.tool_calls.map(|tcs| {
                    tcs.into_iter().map(|tc| crate::workflows::plan::ToolCall {
                        id: tc.id,
                        function: crate::workflows::plan::ToolFunction {
                            name: tc.function.name,
                            arguments: tc.function.arguments,
                        },
                    }).collect()
                }),
            })
        } else {
            Err(LlmError::InvalidResponse("No choices in response".into()))
        }
    }
}
```

---

## C) Backend: New Tauri Command

**File**: `src-tauri/src/commands.rs` (addition)

```rust
use crate::workflows::plan::{generate_plan, PlanResult, PlanError};

#[tauri::command]
pub async fn generate_plan_command(
    app: AppHandle,
    project_id: String,
    task_id: String,
) -> Result<PlanResult, String> {
    generate_plan(app, project_id, task_id)
        .await
        .map_err(|e| format!("[{}] {}", e.code, e.message))
}
```

**File**: `src-tauri/src/lib.rs` (register command)

Add to invoke_handler:
```rust
commands::generate_plan_command,
```

---

## D) Frontend: TaskDetail Updates

**File**: `src/lib/api.ts` (addition)

```typescript
export async function generatePlan(
  projectId: string,
  taskId: string
): Promise<{
  run_id: string;
  plan_md: string;
  tool_calls_count: number;
  truncated: boolean;
}> {
  return invoke("generate_plan_command", { projectId, taskId });
}
```

**File**: `src/routes/TaskDetail.tsx` (additions)

```typescript
// Add state
const [isGeneratingPlan, setIsGeneratingPlan] = useState(false);
const [lastPlanRunId, setLastPlanRunId] = useState<string | null>(null);

// Add function
async function handleGeneratePlan() {
  if (!projectId || !taskId) return;
  
  setIsGeneratingPlan(true);
  try {
    const result = await generatePlan(projectId, taskId);
    setLastPlanRunId(result.run_id);
    
    // Refresh artifacts to show new plan
    const updatedArtifacts = await listArtifacts(taskId);
    setArtifacts(updatedArtifacts);
    
    // Refresh runs list
    const updatedRuns = await listRuns(taskId);
    setRuns(updatedRuns);
    
    // Show success (could add toast here)
    console.log(`Plan generated with ${result.tool_calls_count} tool calls`);
    if (result.truncated) {
      console.warn("Plan was truncated due to limits");
    }
  } catch (err) {
    console.error("Failed to generate plan:", err);
    alert(`Failed to generate plan: ${err}`);
  } finally {
    setIsGeneratingPlan(false);
  }
}

// Add copy function
async function copyPlanToClipboard() {
  if (planArtifact) {
    await navigator.clipboard.writeText(planArtifact.content);
    // Could show toast here
  }
}

// In JSX, add buttons:
{/* Plan Artifact Section */}
<div style={{ marginBottom: 16 }}>
  <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
    <h2>Plan</h2>
    <div style={{ display: "flex", gap: 8 }}>
      <button
        onClick={handleGeneratePlan}
        disabled={isGeneratingPlan}
        style={{
          padding: "8px 16px",
          background: isGeneratingPlan ? "#ccc" : "#007bff",
          color: "white",
          border: "none",
          borderRadius: 6,
          cursor: isGeneratingPlan ? "not-allowed" : "pointer",
        }}
      >
        {isGeneratingPlan ? "Generating..." : "Generate Plan"}
      </button>
      
      {planArtifact && (
        <button
          onClick={copyPlanToClipboard}
          style={{
            padding: "8px 16px",
            background: "#6c757d",
            color: "white",
            border: "none",
            borderRadius: 6,
            cursor: "pointer",
          }}
        >
          Copy Plan
        </button>
      )}
    </div>
  </div>
  
  {isGeneratingPlan && (
    <div style={{ padding: 12, background: "#e7f3ff", borderRadius: 8, marginTop: 8 }}>
      🤖 Planning... Tool calls will appear in RunDetail.
    </div>
  )}
  
  {planArtifact ? (
    <div>
      <pre style={{
        whiteSpace: "pre-wrap",
        background: "#f8f9fa",
        padding: 16,
        borderRadius: 8,
        marginTop: 8,
        maxHeight: 600,
        overflow: "auto",
      }}>
        {planArtifact.content}
      </pre>
      
      {lastPlanRunId && (
        <Link
          to={`/projects/${projectId}/tasks/${taskId}/runs/${lastPlanRunId}`}
          style={{ fontSize: 14, marginTop: 8, display: "inline-block" }}
        >
          View Run Details →
        </Link>
      )}
    </div>
  ) : (
    <div style={{ opacity: 0.6, marginTop: 8 }}>
      No plan yet. Click "Generate Plan" to create one.
    </div>
  )}
</div>
```

---

## E) Acceptance Criteria

- [ ] Can create a Task (mode=plan) and click Generate Plan
- [ ] Plan is generated with tool exploration (list_files, read_file, grep)
- [ ] RunDetail shows:
  - System prompt message
  - User request message
  - Assistant messages with tool_calls
  - Tool result messages
  - Tool Calls tab with structured tool_calls
- [ ] Plan is saved to artifacts (kind="plan_md")
- [ ] Plan follows required Markdown structure
- [ ] Loop bounded at 12 tool calls max
- [ ] Context truncation works when limit exceeded
- [ ] Copy Plan button works
- [ ] Link to run details works

---

## F) Implementation Order

1. Add ToolCall/ToolFunction types to LLM module
2. Add chat_with_tools method to LlmClient
3. Create workflows/plan.rs with generate_plan function
4. Add generate_plan_command Tauri command
5. Register command in lib.rs
6. Update TaskDetail.tsx UI
7. Test end-to-end on real repo
