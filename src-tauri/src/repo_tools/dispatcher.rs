use serde_json::Value;
use std::path::Path;
use tauri::AppHandle;

use crate::repo_tools::fs::{list_files, read_file};
use crate::repo_tools::search::grep;
use crate::repo_tools::git::{git_status, git_diff, git_log_short};
use crate::repo_tools::runner::run_command;

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
