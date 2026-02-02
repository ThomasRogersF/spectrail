use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tauri::AppHandle;
use std::collections::HashMap;
use std::path::Path;

use crate::db;
use crate::models::*;
use crate::repo_tools::dispatch_repo_tool;
use crate::llm::{LlmClient, ChatMessage, LlmConfig};

const MAX_CONTEXT_CHARS: usize = 100_000;

#[derive(Debug, Deserialize, Clone)]
pub struct VerifyOptions {
    #[serde(default = "default_true")]
    pub run_tests: bool,
    #[serde(default)]
    pub run_lint: bool,
    #[serde(default)]
    pub run_build: bool,
    #[serde(default)]
    pub staged: bool,
    #[serde(default = "default_max")]
    pub max_tool_calls: usize,
}

fn default_true() -> bool { true }
fn default_max() -> usize { 8 }

impl Default for VerifyOptions {
    fn default() -> Self {
        Self {
            run_tests: true,
            run_lint: false,
            run_build: false,
            staged: false,
            max_tool_calls: 8,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct VerifyResult {
    pub run_id: String,
    pub report_md: String,
    pub ran_checks: RanChecks,
    pub truncated: bool,
}

#[derive(Debug, Serialize)]
pub struct RanChecks {
    pub tests: bool,
    pub lint: bool,
    pub build: bool,
}

#[derive(Debug, Serialize)]
pub struct VerifyError {
    pub code: String,
    pub message: String,
}

pub async fn verify_task(
    app: AppHandle,
    project_id: String,
    task_id: String,
    options: VerifyOptions,
) -> Result<VerifyResult, VerifyError> {
    // 1. Get task and project info
    let (task, project) = get_task_and_project(&app, &task_id, &project_id)
        .map_err(|e| VerifyError { code: "DB_ERROR".into(), message: e })?;

    // 2. Get settings for LLM
    let settings = get_all_settings(&app)?;
    let llm_config = build_llm_config(&settings);
    let api_key = get_api_key(&settings)?;

    // 3. Create run
    let run_id = create_run_verify(&app, &task_id, &llm_config)
        .map_err(|e| VerifyError { code: "RUN_ERROR".into(), message: e })?;

    // 4. Load plan artifact (if exists)
    let plan_md = load_plan_artifact(&app, &task_id).ok();

    // 5. Gather repo state
    let repo_path = Path::new(&project.repo_path);
    let mut truncated = false;
    let mut tool_calls_count = 0;

    // git_status
    let status_result = execute_tool_simple(
        &app, &run_id, &project_id, repo_path, "git_status", json!({})
    ).await;
    let git_status = format_tool_result(&status_result);
    if status_result.as_ref().map_or(false, |v| {
        v.get("truncated").and_then(|t| t.as_bool()).unwrap_or(false)
    }) {
        truncated = true;
    }
    tool_calls_count += 1;

    // git_diff
    let diff_result = execute_tool_simple(
        &app, &run_id, &project_id, repo_path, "git_diff", json!({ "staged": options.staged })
    ).await;
    let git_diff = format_tool_result(&diff_result);
    if diff_result.as_ref().map_or(false, |v| {
        v.get("truncated").and_then(|t| t.as_bool()).unwrap_or(false)
    }) {
        truncated = true;
    }
    tool_calls_count += 1;

    // 6. Run optional checks
    let mut ran_checks = RanChecks { tests: false, lint: false, build: false };
    let mut test_output = String::new();
    let mut lint_output = String::new();
    let mut build_output = String::new();

    if options.run_tests && tool_calls_count < options.max_tool_calls {
        let result = execute_tool_simple(
            &app, &run_id, &project_id, repo_path, "run_command", json!({ "kind": "tests" })
        ).await;
        test_output = format_tool_result(&result);
        if result.as_ref().map_or(false, |v| {
            v.get("truncated").and_then(|t| t.as_bool()).unwrap_or(false)
        }) {
            truncated = true;
        }
        ran_checks.tests = true;
        tool_calls_count += 1;
    }

    if options.run_lint && tool_calls_count < options.max_tool_calls {
        let result = execute_tool_simple(
            &app, &run_id, &project_id, repo_path, "run_command", json!({ "kind": "lint" })
        ).await;
        lint_output = format_tool_result(&result);
        if result.as_ref().map_or(false, |v| {
            v.get("truncated").and_then(|t| t.as_bool()).unwrap_or(false)
        }) {
            truncated = true;
        }
        ran_checks.lint = true;
        tool_calls_count += 1;
    }

    if options.run_build && tool_calls_count < options.max_tool_calls {
        let result = execute_tool_simple(
            &app, &run_id, &project_id, repo_path, "run_command", json!({ "kind": "build" })
        ).await;
        build_output = format_tool_result(&result);
        if result.as_ref().map_or(false, |v| {
            v.get("truncated").and_then(|t| t.as_bool()).unwrap_or(false)
        }) {
            truncated = true;
        }
        ran_checks.build = true;
        tool_calls_count += 1;
    }

    // 7. Build LLM messages
    let messages = build_verify_messages(
        &task,
        plan_md.as_deref(),
        &git_status,
        &git_diff,
        &test_output,
        &lint_output,
        &build_output,
        options.staged,
        truncated,
    );

    // Log messages
    for msg in &messages {
        log_message(&app, &run_id, &msg.role, msg.content.as_deref().unwrap_or(""))
            .map_err(|e| VerifyError { code: "LOG_ERROR".into(), message: e })?;
    }

    // 8. Call LLM (single call, no tool loop needed)
    let client = LlmClient::new(llm_config, api_key);
    let response = client.chat_with_tools(messages, vec![]).await
        .map_err(|e| VerifyError { code: "LLM_ERROR".into(), message: e.to_string() })?;

    let report_md = response.content.unwrap_or_else(|| {
        "**Error**: No response from LLM".to_string()
    });

    // Log assistant message
    log_message(&app, &run_id, "assistant", &report_md)
        .map_err(|e| VerifyError { code: "LOG_ERROR".into(), message: e })?;

    // 9. Save verification report
    save_artifact(&app, &task_id, "verification_report", &report_md)
        .map_err(|e| VerifyError { code: "ARTIFACT_ERROR".into(), message: e })?;

    Ok(VerifyResult {
        run_id,
        report_md,
        ran_checks,
        truncated,
    })
}

fn build_verify_messages(
    task: &Task,
    plan_md: Option<&str>,
    git_status: &str,
    git_diff: &str,
    test_output: &str,
    lint_output: &str,
    build_output: &str,
    staged: bool,
    mut truncated: bool,
) -> Vec<ChatMessage> {
    let system_prompt = r#"You are a senior code reviewer conducting a verification review.

Your task: Compare the actual changes in the repository against the implementation plan (if provided) and produce a verification report.

Required output format (Markdown):

# Verification Report

## 1. Verdict
One of:
- ✅ **Matches** - Changes fully implement the plan with no issues
- ⚠️ **Partially Matches** - Changes mostly implement the plan with minor issues
- ❌ **Does Not Match** - Changes diverge significantly from the plan or have serious issues

## 2. Summary of Changes Observed
Brief overview of what was actually changed in the codebase.

## 3. Plan Compliance Analysis
(if a plan was provided; otherwise state "No plan provided - general review")
- What was implemented correctly
- What's missing or incomplete
- What diverged from the plan and why

## 4. Risk Review
| Risk | Severity | Notes |
|------|----------|-------|
| e.g., Breaking change | High/Med/Low | Explanation |
| e.g., Security concern | High/Med/Low | Explanation |
| e.g., Performance impact | High/Med/Low | Explanation |

## 5. Test/Check Results
Summarize the test, lint, and build results (if available).

## 6. Recommended Next Actions
- [ ] Specific action item
- [ ] Another action item

## 7. Patch Suggestions (Optional)
High-level suggestions for improvements (not full code patches).

---

Instructions:
- Be objective and thorough
- Cite specific files/paths when discussing changes
- If no plan was provided, do a general code review focusing on best practices
- Always include a clear verdict at the top"#;

    let mut user_prompt = format!(
        "Task: {}\n\n",
        task.title
    );

    if let Some(plan) = plan_md {
        user_prompt.push_str("## Implementation Plan\n\n");
        let truncated_plan = if plan.len() > 5000 {
            &plan[..5000]
        } else {
            plan
        };
        user_prompt.push_str(truncated_plan);
        user_prompt.push_str("\n\n---\n\n");
    } else {
        user_prompt.push_str("*No implementation plan provided. Conducting general code review.*\n\n");
    }

    user_prompt.push_str("## Repository State\n\n");
    user_prompt.push_str(&format!("### Git Status\n```\n{}\n```\n\n", git_status));
    
    let diff_label = if staged { "Staged Changes" } else { "Unstaged Changes" };
    let truncated_diff = if git_diff.len() > 30000 {
        truncated = true;
        &git_diff[..30000]
    } else {
        git_diff
    };
    user_prompt.push_str(&format!("### {}\n```diff\n{}\n```\n\n", diff_label, truncated_diff));

    if !test_output.is_empty() {
        let truncated_test = if test_output.len() > 10000 {
            truncated = true;
            &test_output[..10000]
        } else {
            test_output
        };
        user_prompt.push_str(&format!("### Test Results\n```\n{}\n```\n\n", truncated_test));
    }

    if !lint_output.is_empty() {
        let truncated_lint = if lint_output.len() > 5000 {
            truncated = true;
            &lint_output[..5000]
        } else {
            lint_output
        };
        user_prompt.push_str(&format!("### Lint Results\n```\n{}\n```\n\n", truncated_lint));
    }

    if !build_output.is_empty() {
        let truncated_build = if build_output.len() > 5000 {
            truncated = true;
            &build_output[..5000]
        } else {
            build_output
        };
        user_prompt.push_str(&format!("### Build Results\n```\n{}\n```\n\n", truncated_build));
    }

    if truncated {
        user_prompt.push_str("\n*Note: Some inputs were truncated due to size limits.*\n");
    }

    // Cap total prompt size
    if user_prompt.len() > MAX_CONTEXT_CHARS {
        user_prompt = user_prompt[..MAX_CONTEXT_CHARS].to_string();
        user_prompt.push_str("\n\n[Content truncated due to size limits]");
    }

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

async fn execute_tool_simple(
    app: &AppHandle,
    run_id: &str,
    project_id: &str,
    repo_path: &Path,
    name: &str,
    args: Value,
) -> Result<Value, String> {
    dispatch_repo_tool(name, &args, repo_path, app, run_id).await
}

fn format_tool_result(result: &Result<Value, String>) -> String {
    match result {
        Ok(val) => val.to_string(),
        Err(e) => json!({ "error": e }).to_string(),
    }
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

fn load_plan_artifact(app: &AppHandle, task_id: &str) -> Result<String, String> {
    let conn = db::connect(app).map_err(|e| e.to_string())?;
    
    let content: String = conn.query_row(
        "SELECT content FROM artifacts WHERE task_id = ?1 AND phase_id IS NULL AND kind = 'plan_md' ORDER BY created_at DESC LIMIT 1",
        [task_id],
        |r| r.get(0)
    ).map_err(|e| e.to_string())?;
    
    Ok(content)
}

fn create_run_verify(
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
        (
            &id, task_id, "verify", &llm_config.provider_name, &llm_config.model, &started_at
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
        (
            &id, run_id, role, content, &created_at
        )
    ).map_err(|e| e.to_string())?;
    
    Ok(())
}

fn save_artifact(
    app: &AppHandle,
    task_id: &str,
    kind: &str,
    content: &str,
) -> Result<(), String> {
    let conn = db::connect(app).map_err(|e| e.to_string())?;
    let created_at = now_iso();
    let id = new_id();
    
    // Check if artifact exists
    let existing: Option<String> = conn.query_row(
        "SELECT id FROM artifacts WHERE task_id = ?1 AND phase_id IS NULL AND kind = ?2 LIMIT 1",
        (task_id, kind),
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
            (
                &id, task_id, kind, content, &created_at
            )
        ).map_err(|e| e.to_string())?;
    }
    
    Ok(())
}

fn get_all_settings(app: &AppHandle) -> Result<HashMap<String, String>, VerifyError> {
    let conn = db::connect(app).map_err(|e| VerifyError {
        code: "DB_ERROR".into(),
        message: e.to_string(),
    })?;
    
    let mut stmt = conn.prepare("SELECT key, value FROM settings")
        .map_err(|e| VerifyError {
            code: "DB_ERROR".into(),
            message: e.to_string(),
        })?;
    
    let rows = stmt.query_map([], |r| {
        Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
    }).map_err(|e| VerifyError {
        code: "DB_ERROR".into(),
        message: e.to_string(),
    })?;
    
    let mut settings = HashMap::new();
    for row in rows {
        let (k, v) = row.map_err(|e| VerifyError {
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

fn get_api_key(settings: &HashMap<String, String>) -> Result<String, VerifyError> {
    // Try to get from settings first
    if let Some(key) = settings.get("api_key") {
        if !key.is_empty() {
            return Ok(key.clone());
        }
    }
    
    // Fallback to environment variable
    std::env::var("SPECTRAIL_API_KEY")
        .map_err(|_| VerifyError {
            code: "NO_API_KEY".into(),
            message: "API key not set in settings or SPECTRAIL_API_KEY environment variable".into(),
        })
}

fn now_iso() -> String {
    let t = time::OffsetDateTime::now_utc();
    t.format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string())
}

fn new_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

// Helper trait for OptionalRow
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
