use serde_json::{json, Value};
use std::path::Path;
use crate::repo_tools::safety::{safe_spawn, has_ripgrep};
use crate::repo_tools::logging::log_tool_call;
use tauri::AppHandle;

const MAX_RESULTS_DEFAULT: usize = 200;

pub async fn grep(
    repo_path: &Path,
    args: &Value,
    app: &AppHandle,
    run_id: &str,
) -> Result<Value, String> {
    let query = args.get("query")
        .and_then(|v| v.as_str())
        .ok_or("query is required")?;
    
    let path_filter = args.get("path").and_then(|v| v.as_str());
    let max_results = args.get("max_results")
        .and_then(|v| v.as_u64())
        .unwrap_or(MAX_RESULTS_DEFAULT as u64) as usize;
    
    let matches = if has_ripgrep() {
        grep_ripgrep(repo_path, query, path_filter, max_results).await?
    } else {
        grep_fallback(repo_path, query, path_filter, max_results).await?
    };
    
    let truncated = matches.len() >= max_results;
    let result = json!({
        "matches": matches,
        "truncated": truncated,
        "count": matches.len(),
    });
    
    log_tool_call(app, run_id, "grep", args, &result)?;
    Ok(result)
}

async fn grep_ripgrep(
    repo_path: &Path,
    query: &str,
    path_filter: Option<&str>,
    max_results: usize,
) -> Result<Vec<Value>, String> {
    let max_results_str = max_results.to_string();
    let mut args: Vec<&str> = vec![
        "-n",
        "--max-count",
        &max_results_str,
        "--max-columns",
        "200",
        "-g",
        "!.git",
        "-g",
        "!node_modules",
        "-g",
        "!target",
        "-g",
        "!dist",
        "-g",
        "!build",
    ];
    
    if let Some(path) = path_filter {
        args.push(path);
    }
    
    args.push(query);
    args.push(".");
    
    let (stdout, _, code) = safe_spawn("rg", &args, repo_path, 30)
        .await
        .map_err(|e| e.to_string())?;
    
    // rg returns 1 when no matches found, that's OK
    let _ = code;
    
    let mut matches = vec![];
    for line in stdout.lines() {
        // Parse: path:line:text
        if let Some((path_rest, text)) = line.split_once(':') {
            if let Some((path, line_num)) = path_rest.rsplit_once(':') {
                if let Ok(num) = line_num.parse::<u32>() {
                    matches.push(json!({
                        "path": path,
                        "line": num,
                        "text": text,
                    }));
                }
            }
        }
    }
    
    Ok(matches)
}

async fn grep_fallback(
    repo_path: &Path,
    query: &str,
    path_filter: Option<&str>,
    max_results: usize,
) -> Result<Vec<Value>, String> {
    use walkdir::WalkDir;
    
    let mut matches = vec![];
    let query_lower = query.to_lowercase();
    
    let search_root = if let Some(subdir) = path_filter {
        repo_path.join(subdir)
    } else {
        repo_path.to_path_buf()
    };
    
    for entry in WalkDir::new(search_root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
            let name = e.file_name().to_str().unwrap_or("");
            !matches!(name, ".git" | "node_modules" | "target" | "dist" | "build" | "__pycache__" | ".venv" | "venv")
        })
        .filter_map(|e| e.ok())
    {
        if matches.len() >= max_results {
            break;
        }
        
        if entry.file_type().is_file() {
            let path = entry.path();
            if let Ok(content) = tokio::fs::read_to_string(path).await {
                let rel_path = path.strip_prefix(repo_path).unwrap_or(path)
                    .to_string_lossy();
                
                for (line_num, line) in content.lines().enumerate() {
                    if line.to_lowercase().contains(&query_lower) {
                        matches.push(json!({
                            "path": rel_path,
                            "line": line_num + 1,
                            "text": line.chars().take(200).collect::<String>(),
                        }));
                        
                        if matches.len() >= max_results {
                            break;
                        }
                    }
                }
            }
        }
    }
    
    Ok(matches)
}
