use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub type ID = String;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Project {
  pub id: ID,
  pub name: String,
  pub repo_path: String,
  pub created_at: String,
  pub last_opened_at: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Task {
  pub id: ID,
  pub project_id: ID,
  pub title: String,
  pub mode: String,   // plan|phases|review
  pub status: String, // draft|active|done|archived
  pub created_at: String,
  pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Run {
  pub id: ID,
  pub task_id: ID,
  pub phase_id: Option<ID>,
  pub run_type: String, // plan|verify|handoff|review|phases
  pub provider: Option<String>,
  pub model: Option<String>,
  pub started_at: String,
  pub ended_at: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Message {
  pub id: ID,
  pub run_id: ID,
  pub role: String, // user|assistant|tool
  pub content: String,
  pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Artifact {
  pub id: ID,
  pub task_id: ID,
  pub phase_id: Option<ID>,
  pub kind: String, // plan_md|phase_list|verification_report|handoff_prompt|notes
  pub content: String,
  pub created_at: String,
  pub pinned: i64,
}

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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ToolCallRow {
  pub id: ID,
  pub run_id: ID,
  pub name: String,
  pub args_json: String,
  pub result_json: String,
  pub created_at: String,
}

pub fn new_id() -> ID {
  Uuid::new_v4().to_string()
}
