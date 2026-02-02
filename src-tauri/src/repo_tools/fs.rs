use ignore::WalkBuilder;
use serde_json::{json, Value};
use std::path::Path;
use crate::repo_tools::safety::{sanitize_path, truncate_string};
use crate::repo_tools::logging::log_tool_call;
use tauri::AppHandle;

const MAX_FILES_DEFAULT: usize = 2000;
const MAX_BYTES_DEFAULT: usize = 200_000;

pub async fn list_files(
    repo_path: &Path,
    args: &Value,
    app: &AppHandle,
    run_id: &str,
) -> Result<Value, String> {
    let max_files = args.get("max_files")
        .and_then(|v| v.as_u64())
        .unwrap_or(MAX_FILES_DEFAULT as u64) as usize;
    
    let mut files = vec![];
    let walker = WalkBuilder::new(repo_path)
        .hidden(false)
        .git_ignore(true)
        .filter_entry(|e| {
            let name = e.file_name()
                .to_str()
                .unwrap_or("");
            // Exclude common non-code directories
            !matches!(name, ".git" | "node_modules" | "target" | "dist" | "build" | ".next" | "__pycache__" | ".venv" | "venv" | ".pytest_cache" | ".mypy_cache")
        })
        .build();
    
    for entry in walker {
        if files.len() >= max_files {
            break;
        }
        
        if let Ok(entry) = entry {
            if entry.file_type().map_or(false, |ft| ft.is_file()) {
                let rel_path = entry.path()
                    .strip_prefix(repo_path)
                    .unwrap_or(entry.path())
                    .to_string_lossy()
                    .replace('\\', "/");
                files.push(rel_path);
            }
        }
    }
    
    let truncated = files.len() >= max_files;
    let result = json!({
        "files": files,
        "count": files.len(),
        "truncated": truncated,
    });
    
    log_tool_call(app, run_id, "list_files", args, &result)?;
    Ok(result)
}

pub async fn read_file(
    repo_path: &Path,
    args: &Value,
    app: &AppHandle,
    run_id: &str,
) -> Result<Value, String> {
    let rel_path = args.get("path")
        .and_then(|v| v.as_str())
        .ok_or("path is required")?;
    
    let max_bytes = args.get("max_bytes")
        .and_then(|v| v.as_u64())
        .unwrap_or(MAX_BYTES_DEFAULT as u64) as usize;
    
    let full_path = sanitize_path(repo_path, rel_path)
        .map_err(|e| e.to_string())?;
    
    // Read file
    let content = tokio::fs::read(&full_path).await
        .map_err(|e| format!("Cannot read file: {}", e))?;
    
    // Check if binary
    let is_binary = content.iter().any(|&b| b == 0 || (b < 32 && b != 9 && b != 10 && b != 13));
    
    if is_binary {
        let result = json!({
            "path": rel_path,
            "binary": true,
            "bytes": content.len(),
            "truncated": false,
        });
        log_tool_call(app, run_id, "read_file", args, &result)?;
        return Ok(result);
    }
    
    // Convert to string
    let text = String::from_utf8(content)
        .map_err(|_| "File is not valid UTF-8")?;
    
    let (content_truncated, truncated) = truncate_string(&text, max_bytes);
    
    let result = json!({
        "path": rel_path,
        "content": content_truncated,
        "bytes": text.len(),
        "truncated": truncated,
    });
    
    log_tool_call(app, run_id, "read_file", args, &result)?;
    Ok(result)
}
