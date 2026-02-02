use serde::Serialize;
use serde_json::{json, Value};
use tauri::AppHandle;
use std::collections::HashMap;
use std::path::Path;

use crate::db;
use crate::models::*;
use crate::repo_tools::{repo_tool_schemas, dispatch_repo_tool};
use crate::llm::{LlmClient, ChatMessage, LlmConfig, LlmError};

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

impl From<LlmError> for PlanError {
    fn from(e: LlmError) -> Self {
        PlanError {
            code: "LLM_ERROR".to_string(),
            message: e.to_string(),
        }
    }
}

pub async fn generate_plan(
    app: AppHandle,
    project_id: String,
    task_id: String,
) -> Result<PlanResult, PlanError> {
    // 1. Get task and project info
    let (task, project) = get_task_and_project(&app, &task_id, &project_id
    ).map_err(|e| PlanError { code: "DB_ERROR".into(), message: e })?;
    
    // 2. Get settings for LLM
    let settings = get_all_settings(&app)?;
    let llm_config = build_llm_config(&settings);
    let api_key = get_api_key(&settings)?;
    
    // 3. Create run
    let run_id = create_run_plan(&app, &task_id, &llm_config
    ).map_err(|e| PlanError { code: "RUN_ERROR".into(), message: e })?;
    
    // 4. Build initial messages
    let mut messages = build_initial_messages(&task, &project);
    
    // Log system and user messages
    for msg in &messages {
        log_message(&app, &run_id, &msg.role, msg.content.as_deref().unwrap_or("")
        ).map_err(|e| PlanError { code: "LOG_ERROR".into(), message: e })?;
    }
    
    // 5. Get tool schemas
    let tools = repo_tool_schemas();
    
    // 6. Tool-call loop
    let client = LlmClient::new(llm_config, api_key);
    let mut tool_calls_count = 0;
    let mut truncated = false;
    let mut final_plan = String::new();
    
    for _iteration in 0..MAX_TOOL_ITERATIONS {
        // Check context size
        let context_size: usize = messages.iter()
            .map(|m| m.content.as_ref().map_or(0, |c| c.len()))
            .sum();
        
        if context_size > MAX_CONTEXT_CHARS {
            truncated = true;
            messages = truncate_messages(messages, MAX_CONTEXT_CHARS);
        }
        
        // Call LLM
        let response = client.chat_with_tools(messages.clone(), tools.clone()).await?;
        
        // Check for tool calls
        if let Some(tool_calls) = response.tool_calls {
            if tool_calls.is_empty() {
                // No more tools, we have final plan
                final_plan = response.content.unwrap_or_default();
                
                // Log assistant message
                log_message(&app, &run_id, "assistant", &final_plan
                ).map_err(|e| PlanError { code: "LOG_ERROR".into(), message: e })?;
                break;
            }
            
            tool_calls_count += tool_calls.len();
            
            // Log assistant message with tool calls
            let tool_names: Vec<&str> = tool_calls.iter().map(|t| t.function.name.as_str()).collect();
            let assistant_content = response.content.clone()
                .unwrap_or_else(|| format!("Calling tools: {}", tool_names.join(", ")));
            log_message(&app, &run_id, "assistant", &assistant_content
            ).map_err(|e| PlanError { code: "LOG_ERROR".into(), message: e })?;
            
            // Execute each tool call
            for tool_call in &tool_calls {
                let tool_result = execute_single_tool(
                    &app,
                    &run_id,
                    &project_id,
                    &tool_call,
                ).await;
                
                // Add tool result as message
                let tool_content = match &tool_result {
                    Ok(val) => val.to_string(),
                    Err(e) => json!({ "error": e }).to_string(),
                };
                
                let tool_message = ChatMessage {
                    role: "tool".into(),
                    content: Some(tool_content.clone()),
                    tool_call_id: Some(tool_call.id.clone()),
                    tool_calls: None,
                };
                
                messages.push(tool_message.clone());
                
                // Log to database
                log_message(&app, &run_id, "tool", &tool_content
                ).map_err(|e| PlanError { code: "LOG_ERROR".into(), message: e })?;
            }
            
            // Add assistant message to context for next iteration
            messages.push(ChatMessage {
                role: "assistant".into(),
                content: response.content,
                tool_call_id: None,
                tool_calls: Some(tool_calls),
            });
        } else {
            // No tool calls, we have final plan
            final_plan = response.content.unwrap_or_default();
            
            // Log assistant message
            log_message(&app, &run_id, "assistant", &final_plan
            ).map_err(|e| PlanError { code: "LOG_ERROR".into(), message: e })?;
            break;
        }
    }
    
    // If we hit max iterations, add a note
    if tool_calls_count >= MAX_TOOL_ITERATIONS && final_plan.is_empty() {
        final_plan = format!(
            "**Error**: Reached maximum tool call limit ({}). Unable to complete plan.\n\n\
             Please try:\n\
             1. Breaking this task into smaller, more specific tasks\n\
             2. Providing more context about what needs to be done\n\
             3. Checking if the repository is accessible and contains the expected files",
            MAX_TOOL_ITERATIONS
        );
        truncated = true;
    }
    
    // Add truncation note if needed
    if truncated {
        final_plan = format!(
            "{}\n\n---\n\n**Note**: This plan was truncated due to context size limits. Some details may be incomplete.",
            final_plan
        );
    }
    
    // 7. Save plan artifact
    save_artifact(&app, &task_id, &final_plan
    ).map_err(|e| PlanError { code: "ARTIFACT_ERROR".into(), message: e })?;
    
    Ok(PlanResult {
        run_id,
        plan_md: final_plan,
        tool_calls_count,
        truncated,
    })
}

fn get_task_and_project(
    app: &AppHandle,
    task_id: &str,
    project_id: &str,
) -> Result<(Task, Project), String> {
    let conn = db::connect(app).map_err(|e| e.to_string())?;
    
    let task: Task = conn.query_row(
        "SELECT id, project_id, title, mode, status, created_at, updated_at FROM tasks WHERE id = ?1",
        [task_id],
        |r| Ok(Task {
            id: r.get(0)?,
            project_id: r.get(1)?,
            title: r.get(2)?,
            mode: r.get(3)?,
            status: r.get(4)?,
            created_at: r.get(5)?,
            updated_at: r.get(6)?,
        })
    ).map_err(|e| e.to_string())?;
    
    let project: Project = conn.query_row(
        "SELECT id, name, repo_path, created_at, last_opened_at FROM projects WHERE id = ?1",
        [project_id],
        |r| Ok(Project {
            id: r.get(0)?,
            name: r.get(1)?,
            repo_path: r.get(2)?,
            created_at: r.get(3)?,
            last_opened_at: r.get(4)?,
        })
    ).map_err(|e| e.to_string())?;
    
    Ok((task, project))
}

fn create_run_plan(
    app: &AppHandle,
    task_id: &str,
    llm_config: &LlmConfig,
) -> Result<String, String> {
    let conn = db::connect(app).map_err(|e| e.to_string())?;
    let id = new_id();
    let started_at = now_iso();
    
    conn.execute(
        "INSERT INTO runs (id, task_id, phase_id, run_type, provider, model, started_at, ended_at) 
         VALUES (?1, ?2, NULL, ?3, ?4, ?5, ?6, NULL)",
        (&id, task_id, "plan", &llm_config.provider_name, &llm_config.model, &started_at
        )
    ).map_err(|e| e.to_string())?;
    
    Ok(id)
}

fn log_message(
    app: &AppHandle,
    run_id: &str,
    role: &str,
    content: &str,
) -> Result<(), String> {
    let conn = db::connect(app).map_err(|e| e.to_string())?;
    let id = new_id();
    let created_at = now_iso();
    
    conn.execute(
        "INSERT INTO messages (id, run_id, role, content, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
        (&id, run_id, role, content, &created_at
        )
    ).map_err(|e| e.to_string())?;
    
    Ok(())
}

fn save_artifact(
    app: &AppHandle,
    task_id: &str,
    content: &str,
) -> Result<(), String> {
    let conn = db::connect(app).map_err(|e| e.to_string())?;
    let created_at = now_iso();
    let id = new_id();
    
    // Check if artifact exists
    let existing: Option<String> = conn.query_row(
        "SELECT id FROM artifacts WHERE task_id = ?1 AND phase_id IS NULL AND kind = ?2 LIMIT 1",
        (task_id, "plan_md"),
        |r| r.get(0)
    ).optional().map_err(|e| e.to_string())?;
    
    if let Some(existing_id) = existing {
        // Update
        conn.execute(
            "UPDATE artifacts SET content = ?1, created_at = ?2 WHERE id = ?3",
            (content, &created_at, &existing_id)
        ).map_err(|e| e.to_string())?;
    } else {
        // Insert
        conn.execute(
            "INSERT INTO artifacts (id, task_id, phase_id, kind, content, created_at, pinned) 
             VALUES (?1, ?2, NULL, ?3, ?4, ?5, 0)",
            (&id, task_id, "plan_md", content, &created_at
            )
        ).map_err(|e| e.to_string())?;
    }
    
    Ok(())
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
8. When complete, output ONLY the plan in the format above (no tool calls in final output)"#;

    let user_prompt = format!(
        r#"Task: {title}

Repository: {repo_path}

Please explore this codebase and create a detailed implementation plan.

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
    tool_call: &crate::llm::types::ToolCall,
) -> Result<Value, String> {
    // Parse args
    let args: Value = serde_json::from_str(&tool_call.function.arguments)
        .map_err(|e| format!("Failed to parse tool args: {}", e))?;
    
    // Add project_id to args if not present
    let mut args_with_project = args.clone();
    if let Some(obj) = args_with_project.as_object_mut() {
        obj.entry("project_id".to_string())
            .or_insert_with(|| json!(project_id));
    }
    
    // Get project repo path
    let conn = db::connect(app).map_err(|e| e.to_string())?;
    let repo_path: String = conn.query_row(
        "SELECT repo_path FROM projects WHERE id = ?1",
        [project_id],
        |r| r.get(0)
    ).map_err(|e| e.to_string())?;
    
    // Execute tool
    let repo_path = Path::new(&repo_path);
    dispatch_repo_tool(
        &tool_call.function.name,
        &args_with_project,
        repo_path,
        app,
        run_id,
    ).await
}

fn get_all_settings(app: &AppHandle) -> Result<HashMap<String, String>, PlanError> {
    let conn = db::connect(app).map_err(|e| PlanError {
        code: "DB_ERROR".into(),
        message: e.to_string(),
    })?;
    
    let mut stmt = conn.prepare("SELECT key, value FROM settings")
        .map_err(|e| PlanError {
            code: "DB_ERROR".into(),
            message: e.to_string(),
        })?;
    
    let rows = stmt.query_map([], |r| {
        Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
    }).map_err(|e| PlanError {
        code: "DB_ERROR".into(),
        message: e.to_string(),
    })?;
    
    let mut settings = HashMap::new();
    for row in rows {
        let (k, v) = row.map_err(|e| PlanError {
            code: "DB_ERROR".into(),
            message: e.to_string(),
        })?;
        settings.insert(k, v);
    }
    
    Ok(settings)
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

fn get_api_key(settings: &HashMap<String, String>) -> Result<String, PlanError> {
    // Try to get from settings first
    if let Some(key) = settings.get("api_key") {
        if !key.is_empty() {
            return Ok(key.clone());
        }
    }
    
    // Fallback to environment variable
    std::env::var("SPECTRAIL_API_KEY")
        .map_err(|_| PlanError {
            code: "NO_API_KEY".into(),
            message: "API key not set in settings or SPECTRAIL_API_KEY environment variable".into(),
        })
}

fn truncate_messages(messages: Vec<ChatMessage>, _max_chars: usize) -> Vec<ChatMessage> {
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

fn now_iso() -> String {
    let t = time::OffsetDateTime::now_utc();
    t.format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string())
}

fn new_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

// Helper trait for OptionRow
trait OptionalRow<T> {
    fn optional(self) -> Result<Option<T>, rusqlite::Error>;
}

impl<T> OptionalRow<T> for Result<T, rusqlite::Error> {
    fn optional(self) -> Result<Option<T>, rusqlite::Error> {
        match self {
            Ok(v) => Ok(Some(v)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }
}
