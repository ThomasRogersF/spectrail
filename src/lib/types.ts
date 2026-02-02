export type ID = string;

export interface Project {
  id: ID;
  name: string;
  repo_path: string;
  created_at: string;
  last_opened_at: string | null;
}

export interface Task {
  id: ID;
  project_id: ID;
  title: string;
  mode: "plan" | "phases" | "review";
  status: "draft" | "active" | "done" | "archived";
  created_at: string;
  updated_at: string;
}

export interface Run {
  id: ID;
  task_id: ID;
  phase_id: ID | null;
  run_type: "plan" | "verify" | "handoff" | "review" | "phases";
  provider: string | null;
  model: string | null;
  started_at: string;
  ended_at: string | null;
}

export interface Message {
  id: ID;
  run_id: ID;
  role: "user" | "assistant" | "tool";
  content: string;
  created_at: string;
}

export interface Artifact {
  id: ID;
  task_id: ID;
  phase_id: ID | null;
  kind: "plan_md" | "phase_list" | "verification_report" | "handoff_prompt" | "notes";
  content: string;
  created_at: string;
  pinned: 0 | 1;
}
