import { invoke } from "@tauri-apps/api/core";
import type { Project, Task, Run, Message, Artifact } from "./types";

export async function dbHealth(): Promise<{ ok: boolean; path: string }> {
  return invoke("db_health");
}

export async function listProjects(): Promise<Project[]> {
  return invoke("list_projects");
}

export async function createProject(name: string, repoPath: string): Promise<Project> {
  return invoke("create_project", { name, repoPath });
}

export async function touchProject(projectId: string): Promise<void> {
  return invoke("touch_project", { projectId });
}

export async function getProject(projectId: string): Promise<Project> {
  return invoke("get_project", { projectId });
}

export async function listTasks(projectId: string): Promise<Task[]> {
  return invoke("list_tasks", { projectId });
}

export async function createTask(projectId: string, title: string, mode: Task["mode"]): Promise<Task> {
  return invoke("create_task", { projectId, title, mode });
}

export async function getTask(taskId: string): Promise<Task> {
  return invoke("get_task", { taskId });
}

export async function listRuns(taskId: string): Promise<Run[]> {
  return invoke("list_runs", { taskId });
}

export async function createRun(taskId: string, runType: Run["run_type"]): Promise<Run> {
  return invoke("create_run", { taskId, runType });
}

export async function listMessages(runId: string): Promise<Message[]> {
  return invoke("list_messages", { runId });
}

export async function addMessage(runId: string, role: Message["role"], content: string): Promise<Message> {
  return invoke("add_message", { runId, role, content });
}

export async function listArtifacts(taskId: string): Promise<Artifact[]> {
  return invoke("list_artifacts", { taskId });
}

export async function upsertArtifact(taskId: string, phaseId: string | null, kind: Artifact["kind"], content: string): Promise<Artifact> {
  return invoke("upsert_artifact", { taskId, phaseId, kind, content });
}

// Settings API
export async function getSettings(): Promise<Array<{ key: string; value: string; updated_at: string }>> {
  return invoke("get_settings");
}

export async function getSetting(key: string): Promise<string | null> {
  return invoke("get_setting", { key });
}

export async function setSetting(key: string, value: string): Promise<void> {
  return invoke("set_setting", { key, value });
}

export async function setSettings(pairs: Array<{ key: string; value: string }>): Promise<void> {
  return invoke("set_settings", { pairs });
}

// Tool calls API
export async function listToolCalls(runId: string): Promise<Array<{
  id: string;
  run_id: string;
  name: string;
  args_json: string;
  result_json: string;
  created_at: string;
}>> {
  return invoke("list_tool_calls_cmd", { runId });
}

export async function executeRepoTool(
  runId: string,
  projectId: string,
  name: string,
  args: Record<string, unknown>
): Promise<unknown> {
  return invoke("execute_repo_tool", { runId, projectId, name, args });
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

// Plan workflow API
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

export async function verifyTask(
  projectId: string,
  taskId: string,
  options?: {
    run_tests?: boolean;
    run_lint?: boolean;
    run_build?: boolean;
    staged?: boolean;
  }
): Promise<{
  run_id: string;
  report_md: string;
  ran_checks: { tests: boolean; lint: boolean; build: boolean };
  truncated: boolean;
}> {
  return invoke("verify_task_command", { projectId, taskId, options });
}
