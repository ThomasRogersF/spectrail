use serde_json::{json, Value};
use std::path::Path;
use crate::repo_tools::safety::{safe_spawn, truncate_string};
use crate::repo_tools::logging::log_tool_call;
use tauri::AppHandle;

const MAX_DIFF_CHARS: usize = 200_000;

pub async fn git_status(
    repo_path: &Path,
    args: &Value,
    app: &AppHandle,
    run_id: &str,
) -> Result<Value, String> {
    let (stdout, stderr, code) = safe_spawn(
        "git",
        &["status", "--porcelain=v1", "-b"],
        repo_path,
        10
    ).await.map_err(|e| e.to_string())?;
    
    let result = json!({
        "stdout": stdout,
        "stderr": stderr,
        "code": code,
    });
    
    log_tool_call(app, run_id, "git_status", args, &result)?;
    Ok(result)
}

pub async fn git_diff(
    repo_path: &Path,
    args: &Value,
    app: &AppHandle,
    run_id: &str,
) -> Result<Value, String> {
    let staged = args.get("staged").and_then(|v| v.as_bool()).unwrap_or(false);
    
    let mut cmd_args = vec!["diff"];
    if staged {
        cmd_args.push("--staged");
    }
    
    let (stdout, stderr, code) = safe_spawn(
        "git",
        &cmd_args,
        repo_path,
        10
    ).await.map_err(|e| e.to_string())?;
    
    let (diff_truncated, truncated) = truncate_string(&stdout, MAX_DIFF_CHARS);
    
    let result = json!({
        "diff": diff_truncated,
        "stderr": stderr,
        "code": code,
        "truncated": truncated,
    });
    
    log_tool_call(app, run_id, "git_diff", args, &result)?;
    Ok(result)
}

pub async fn git_log_short(
    repo_path: &Path,
    args: &Value,
    app: &AppHandle,
    run_id: &str,
) -> Result<Value, String> {
    let max_commits = args.get("max_commits")
        .and_then(|v| v.as_u64())
        .unwrap_or(10) as usize;
    
    let format_arg = format!("-n{}", max_commits);
    let (stdout, stderr, code) = safe_spawn(
        "git",
        &[
            "log",
            &format_arg,
            "--pretty=format:%h%x09%ad%x09%s",
            "--date=iso",
        ],
        repo_path,
        10
    ).await.map_err(|e| e.to_string())?;
    
    let mut commits = vec![];
    for line in stdout.lines() {
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() >= 3 {
            commits.push(json!({
                "hash": parts[0],
                "date": parts[1],
                "subject": parts[2],
            }));
        }
    }
    
    let result = json!({
        "commits": commits,
        "stderr": stderr,
        "code": code,
        "truncated": commits.len() >= max_commits,
    });
    
    log_tool_call(app, run_id, "git_log_short", args, &result)?;
    Ok(result)
}
