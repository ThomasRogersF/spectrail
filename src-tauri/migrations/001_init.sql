-- SpecTrail SQLite schema (idempotent-ish)
PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS projects (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  repo_path TEXT NOT NULL,
  created_at TEXT NOT NULL,
  last_opened_at TEXT
);

CREATE INDEX IF NOT EXISTS idx_projects_last_opened ON projects(last_opened_at);

CREATE TABLE IF NOT EXISTS tasks (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL,
  title TEXT NOT NULL,
  mode TEXT NOT NULL,    -- plan|phases|review
  status TEXT NOT NULL,  -- draft|active|done|archived
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  FOREIGN KEY(project_id) REFERENCES projects(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_tasks_project ON tasks(project_id);

CREATE TABLE IF NOT EXISTS phases (
  id TEXT PRIMARY KEY,
  task_id TEXT NOT NULL,
  idx INTEGER NOT NULL,
  title TEXT NOT NULL,
  status TEXT NOT NULL, -- todo|active|done
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  FOREIGN KEY(task_id) REFERENCES tasks(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_phases_task ON phases(task_id);

CREATE TABLE IF NOT EXISTS runs (
  id TEXT PRIMARY KEY,
  task_id TEXT NOT NULL,
  phase_id TEXT,
  run_type TEXT NOT NULL, -- plan|verify|handoff|review|phases
  provider TEXT,
  model TEXT,
  started_at TEXT NOT NULL,
  ended_at TEXT,
  FOREIGN KEY(task_id) REFERENCES tasks(id) ON DELETE CASCADE,
  FOREIGN KEY(phase_id) REFERENCES phases(id) ON DELETE SET NULL
);

CREATE INDEX IF NOT EXISTS idx_runs_task ON runs(task_id);

CREATE TABLE IF NOT EXISTS messages (
  id TEXT PRIMARY KEY,
  run_id TEXT NOT NULL,
  role TEXT NOT NULL, -- user|assistant|tool
  content TEXT NOT NULL,
  created_at TEXT NOT NULL,
  FOREIGN KEY(run_id) REFERENCES runs(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_messages_run ON messages(run_id);

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

CREATE TABLE IF NOT EXISTS artifacts (
  id TEXT PRIMARY KEY,
  task_id TEXT NOT NULL,
  phase_id TEXT,
  kind TEXT NOT NULL, -- plan_md|phase_list|verification_report|handoff_prompt|notes
  content TEXT NOT NULL,
  created_at TEXT NOT NULL,
  pinned INTEGER NOT NULL DEFAULT 0,
  FOREIGN KEY(task_id) REFERENCES tasks(id) ON DELETE CASCADE,
  FOREIGN KEY(phase_id) REFERENCES phases(id) ON DELETE SET NULL
);

CREATE INDEX IF NOT EXISTS idx_artifacts_task ON artifacts(task_id);
