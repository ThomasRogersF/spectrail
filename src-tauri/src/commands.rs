use tauri::AppHandle;

use crate::db;
use crate::models::*;

fn now_iso() -> String {
  // RFC3339-ish without nanos; good enough for sorting/display.
  let t = time::OffsetDateTime::now_utc();
  t.format(&time::format_description::well_known::Rfc3339).unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string())
}

#[tauri::command]
pub fn db_health(app: AppHandle) -> Result<serde_json::Value, String> {
  let p = db::paths(&app).map_err(|e| e.to_string())?;
  Ok(serde_json::json!({ "ok": true, "path": p.db_path.to_string_lossy() }))
}

#[tauri::command]
pub fn list_projects(app: AppHandle) -> Result<Vec<Project>, String> {
  let conn = db::connect(&app).map_err(|e| e.to_string())?;
  let mut stmt = conn.prepare(
    "SELECT id, name, repo_path, created_at, last_opened_at FROM projects ORDER BY COALESCE(last_opened_at, created_at) DESC"
  ).map_err(|e| e.to_string())?;
  let rows = stmt.query_map([], |r| {
    Ok(Project {
      id: r.get(0)?,
      name: r.get(1)?,
      repo_path: r.get(2)?,
      created_at: r.get(3)?,
      last_opened_at: r.get(4)?,
    })
  }).map_err(|e| e.to_string())?;

  let mut out = vec![];
  for row in rows {
    out.push(row.map_err(|e| e.to_string())?);
  }
  Ok(out)
}

#[tauri::command]
pub fn create_project(app: AppHandle, name: String, repo_path: String) -> Result<Project, String> {
  let conn = db::connect(&app).map_err(|e| e.to_string())?;
  let id = new_id();
  let created_at = now_iso();
  conn.execute(
    "INSERT INTO projects (id, name, repo_path, created_at, last_opened_at) VALUES (?1, ?2, ?3, ?4, NULL)",
    (&id, &name, &repo_path, &created_at)
  ).map_err(|e| e.to_string())?;

  Ok(Project { id, name, repo_path, created_at, last_opened_at: None })
}

#[tauri::command]
pub fn touch_project(app: AppHandle, project_id: String) -> Result<(), String> {
  let conn = db::connect(&app).map_err(|e| e.to_string())?;
  let t = now_iso();
  conn.execute(
    "UPDATE projects SET last_opened_at = ?1 WHERE id = ?2",
    (&t, &project_id)
  ).map_err(|e| e.to_string())?;
  Ok(())
}

#[tauri::command]
pub fn get_project(app: AppHandle, project_id: String) -> Result<Project, String> {
  let conn = db::connect(&app).map_err(|e| e.to_string())?;
  conn.query_row(
    "SELECT id, name, repo_path, created_at, last_opened_at FROM projects WHERE id = ?1",
    [&project_id],
    |r| Ok(Project {
      id: r.get(0)?,
      name: r.get(1)?,
      repo_path: r.get(2)?,
      created_at: r.get(3)?,
      last_opened_at: r.get(4)?,
    })
  ).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_tasks(app: AppHandle, project_id: String) -> Result<Vec<Task>, String> {
  let conn = db::connect(&app).map_err(|e| e.to_string())?;
  let mut stmt = conn.prepare(
    "SELECT id, project_id, title, mode, status, created_at, updated_at FROM tasks WHERE project_id = ?1 ORDER BY updated_at DESC"
  ).map_err(|e| e.to_string())?;
  let rows = stmt.query_map([project_id], |r| {
    Ok(Task {
      id: r.get(0)?,
      project_id: r.get(1)?,
      title: r.get(2)?,
      mode: r.get(3)?,
      status: r.get(4)?,
      created_at: r.get(5)?,
      updated_at: r.get(6)?,
    })
  }).map_err(|e| e.to_string())?;

  let mut out = vec![];
  for row in rows {
    out.push(row.map_err(|e| e.to_string())?);
  }
  Ok(out)
}

#[tauri::command]
pub fn create_task(app: AppHandle, project_id: String, title: String, mode: String) -> Result<Task, String> {
  let conn = db::connect(&app).map_err(|e| e.to_string())?;
  let id = new_id();
  let ts = now_iso();
  conn.execute(
    "INSERT INTO tasks (id, project_id, title, mode, status, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, 'draft', ?5, ?6)",
    (&id, &project_id, &title, &mode, &ts, &ts)
  ).map_err(|e| e.to_string())?;

  Ok(Task { id, project_id, title, mode, status: "draft".into(), created_at: ts.clone(), updated_at: ts })
}

#[tauri::command]
pub fn get_task(app: AppHandle, task_id: String) -> Result<Task, String> {
  let conn = db::connect(&app).map_err(|e| e.to_string())?;
  conn.query_row(
    "SELECT id, project_id, title, mode, status, created_at, updated_at FROM tasks WHERE id = ?1",
    [&task_id],
    |r| Ok(Task {
      id: r.get(0)?,
      project_id: r.get(1)?,
      title: r.get(2)?,
      mode: r.get(3)?,
      status: r.get(4)?,
      created_at: r.get(5)?,
      updated_at: r.get(6)?,
    })
  ).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_runs(app: AppHandle, task_id: String) -> Result<Vec<Run>, String> {
  let conn = db::connect(&app).map_err(|e| e.to_string())?;
  let mut stmt = conn.prepare(
    "SELECT id, task_id, phase_id, run_type, provider, model, started_at, ended_at FROM runs WHERE task_id = ?1 ORDER BY started_at DESC"
  ).map_err(|e| e.to_string())?;
  let rows = stmt.query_map([task_id], |r| {
    Ok(Run {
      id: r.get(0)?,
      task_id: r.get(1)?,
      phase_id: r.get(2)?,
      run_type: r.get(3)?,
      provider: r.get(4)?,
      model: r.get(5)?,
      started_at: r.get(6)?,
      ended_at: r.get(7)?,
    })
  }).map_err(|e| e.to_string())?;

  let mut out = vec![];
  for row in rows {
    out.push(row.map_err(|e| e.to_string())?);
  }
  Ok(out)
}

#[tauri::command]
pub fn create_run(app: AppHandle, task_id: String, run_type: String) -> Result<Run, String> {
  let conn = db::connect(&app).map_err(|e| e.to_string())?;
  let id = new_id();
  let started_at = now_iso();
  conn.execute(
    "INSERT INTO runs (id, task_id, phase_id, run_type, provider, model, started_at, ended_at) VALUES (?1, ?2, NULL, ?3, NULL, NULL, ?4, NULL)",
    (&id, &task_id, &run_type, &started_at)
  ).map_err(|e| e.to_string())?;
  Ok(Run { id, task_id, phase_id: None, run_type, provider: None, model: None, started_at, ended_at: None })
}

#[tauri::command]
pub fn list_messages(app: AppHandle, run_id: String) -> Result<Vec<Message>, String> {
  let conn = db::connect(&app).map_err(|e| e.to_string())?;
  let mut stmt = conn.prepare(
    "SELECT id, run_id, role, content, created_at FROM messages WHERE run_id = ?1 ORDER BY created_at ASC"
  ).map_err(|e| e.to_string())?;
  let rows = stmt.query_map([run_id], |r| {
    Ok(Message {
      id: r.get(0)?,
      run_id: r.get(1)?,
      role: r.get(2)?,
      content: r.get(3)?,
      created_at: r.get(4)?,
    })
  }).map_err(|e| e.to_string())?;

  let mut out = vec![];
  for row in rows {
    out.push(row.map_err(|e| e.to_string())?);
  }
  Ok(out)
}

#[tauri::command]
pub fn add_message(app: AppHandle, run_id: String, role: String, content: String) -> Result<Message, String> {
  let conn = db::connect(&app).map_err(|e| e.to_string())?;
  let id = new_id();
  let created_at = now_iso();
  conn.execute(
    "INSERT INTO messages (id, run_id, role, content, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
    (&id, &run_id, &role, &content, &created_at)
  ).map_err(|e| e.to_string())?;
  Ok(Message { id, run_id, role, content, created_at })
}

#[tauri::command]
pub fn list_artifacts(app: AppHandle, task_id: String) -> Result<Vec<Artifact>, String> {
  let conn = db::connect(&app).map_err(|e| e.to_string())?;
  let mut stmt = conn.prepare(
    "SELECT id, task_id, phase_id, kind, content, created_at, pinned FROM artifacts WHERE task_id = ?1 ORDER BY created_at DESC"
  ).map_err(|e| e.to_string())?;
  let rows = stmt.query_map([task_id], |r| {
    Ok(Artifact {
      id: r.get(0)?,
      task_id: r.get(1)?,
      phase_id: r.get(2)?,
      kind: r.get(3)?,
      content: r.get(4)?,
      created_at: r.get(5)?,
      pinned: r.get(6)?,
    })
  }).map_err(|e| e.to_string())?;

  let mut out = vec![];
  for row in rows {
    out.push(row.map_err(|e| e.to_string())?);
  }
  Ok(out)
}

#[tauri::command]
pub fn upsert_artifact(app: AppHandle, task_id: String, phase_id: Option<String>, kind: String, content: String) -> Result<Artifact, String> {
  let conn = db::connect(&app).map_err(|e| e.to_string())?;
  // If an artifact of same (task_id, phase_id, kind) exists, update it; else insert.
  let existing: Option<String> = conn.query_row(
    "SELECT id FROM artifacts WHERE task_id = ?1 AND COALESCE(phase_id,'') = COALESCE(?2,'') AND kind = ?3 LIMIT 1",
    (task_id.as_str(), phase_id.as_deref().unwrap_or(""), kind.as_str()),
    |r| r.get(0)
  ).optional().map_err(|e| e.to_string())?;

  let created_at = now_iso();
  let id = if let Some(id) = existing {
    conn.execute(
      "UPDATE artifacts SET content = ?1, created_at = ?2 WHERE id = ?3",
      (&content, &created_at, &id)
    ).map_err(|e| e.to_string())?;
    id
  } else {
    let id = new_id();
    conn.execute(
      "INSERT INTO artifacts (id, task_id, phase_id, kind, content, created_at, pinned) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0)",
      (&id, &task_id, &phase_id, &kind, &content, &created_at)
    ).map_err(|e| e.to_string())?;
    id
  };

  Ok(Artifact { id, task_id, phase_id, kind, content, created_at, pinned: 0 })
}

// Settings commands
#[tauri::command]
pub fn get_settings(app: AppHandle) -> Result<Vec<SettingsKV>, String> {
  let conn = db::connect(&app).map_err(|e| e.to_string())?;
  let mut stmt = conn.prepare(
    "SELECT key, value, updated_at FROM settings ORDER BY key"
  ).map_err(|e| e.to_string())?;
  let rows = stmt.query_map([], |r| {
    Ok(SettingsKV {
      key: r.get(0)?,
      value: r.get(1)?,
      updated_at: r.get(2)?,
    })
  }).map_err(|e| e.to_string())?;

  let mut out = vec![];
  for row in rows {
    out.push(row.map_err(|e| e.to_string())?);
  }
  Ok(out)
}

#[tauri::command]
pub fn get_setting(app: AppHandle, key: String) -> Result<Option<String>, String> {
  let conn = db::connect(&app).map_err(|e| e.to_string())?;
  let result: Option<String> = conn.query_row(
    "SELECT value FROM settings WHERE key = ?1",
    [&key],
    |r| r.get(0)
  ).optional().map_err(|e| e.to_string())?;
  Ok(result)
}

#[tauri::command]
pub fn set_setting(app: AppHandle, key: String, value: String) -> Result<(), String> {
  let conn = db::connect(&app).map_err(|e| e.to_string())?;
  let updated_at = now_iso();
  conn.execute(
    "INSERT INTO settings (key, value, updated_at) VALUES (?1, ?2, ?3)
     ON CONFLICT(key) DO UPDATE SET value=excluded.value, updated_at=excluded.updated_at",
    (&key, &value, &updated_at)
  ).map_err(|e| e.to_string())?;
  Ok(())
}

#[tauri::command]
pub fn set_settings(app: AppHandle, pairs: Vec<SettingInput>) -> Result<(), String> {
  let mut conn = db::connect(&app).map_err(|e| e.to_string())?;
  let tx = conn.transaction().map_err(|e| e.to_string())?;
  let updated_at = now_iso();
  
  for pair in pairs {
    tx.execute(
      "INSERT INTO settings (key, value, updated_at) VALUES (?1, ?2, ?3)
       ON CONFLICT(key) DO UPDATE SET value=excluded.value, updated_at=excluded.updated_at",
      (&pair.key, &pair.value, &updated_at)
    ).map_err(|e| e.to_string())?;
  }
  
  tx.commit().map_err(|e| e.to_string())?;
  Ok(())
}

// Repo tools commands
use crate::repo_tools::{list_tool_calls, dispatch_repo_tool, repo_tool_schemas};

#[tauri::command]
pub fn list_tool_calls_cmd(app: AppHandle, run_id: String) -> Result<Vec<ToolCallRow>, String> {
  list_tool_calls(&app, &run_id)
}

#[tauri::command]
pub async fn execute_repo_tool(
  app: AppHandle,
  run_id: String,
  project_id: String,
  name: String,
  args: serde_json::Value,
) -> Result<serde_json::Value, String> {
  // Look up repo_path from DB
  let project = get_project(app.clone(), project_id)?;
  let repo_path = std::path::Path::new(&project.repo_path);
  
  // Dispatch tool
  let result = dispatch_repo_tool(&name, &args, repo_path, &app, &run_id).await;
  
  result
}

#[tauri::command]
pub fn get_repo_tool_schemas() -> Vec<serde_json::Value> {
  repo_tool_schemas()
}

// Plan workflow command
use crate::workflows::plan::{generate_plan, PlanResult};
use crate::workflows::verify::{verify_task, VerifyOptions, VerifyResult};

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

#[tauri::command]
pub async fn verify_task_command(
  app: AppHandle,
  project_id: String,
  task_id: String,
  options: Option<VerifyOptions>,
) -> Result<VerifyResult, String> {
  let opts = options.unwrap_or_default();
  verify_task(app, project_id, task_id, opts)
    .await
    .map_err(|e| format!("[{}] {}", e.code, e.message))
}

// needed for .optional()
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
