use serde_json::{json, Value};
use std::path::Path;
use std::time::Instant;
use crate::repo_tools::safety::truncate_string;
use crate::repo_tools::logging::log_tool_call;
use tauri::AppHandle;
use tokio::process::Command;
use std::process::Stdio;
use std::time::Duration;
use tokio::time::timeout;

const MAX_OUTPUT_CHARS: usize = 200_000;

#[derive(Debug, Clone, Copy)]
enum CommandKind {
    Tests,
    Lint,
    Build,
}

impl CommandKind {
    fn from_str(s: &str) -> Option<Self> {
        match s {
            "tests" => Some(CommandKind::Tests),
            "lint" => Some(CommandKind::Lint),
            "build" => Some(CommandKind::Build),
            _ => None,
        }
    }
}

pub async fn run_command(
    repo_path: &Path,
    args: &Value,
    app: &AppHandle,
    run_id: &str,
) -> Result<Value, String> {
    let kind_str = args.get("kind")
        .and_then(|v| v.as_str())
        .ok_or("kind is required (tests, lint, or build)")?;
    
    let kind = CommandKind::from_str(kind_str)
        .ok_or("invalid kind, must be: tests, lint, or build")?;
    
    // Auto-detect runner
    let runner = detect_runner(repo_path, args.get("runner").and_then(|v| v.as_str()))?;
    
    // Build allowlisted command
    let cmd_parts = build_command(&runner, kind)?;
    
    let start = Instant::now();
    
    // Spawn directly since safe_spawn expects &[&str]
    let output = timeout(
        Duration::from_secs(300),
        Command::new(&cmd_parts[0])
            .args(&cmd_parts[1..])
            .current_dir(repo_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
    ).await
        .map_err(|_| "Timeout".to_string())?
        .map_err(|e| format!("Command failed: {}", e))?;
    
    let duration_ms = start.elapsed().as_millis() as u64;
    
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let code = output.status.code().unwrap_or(-1);
    
    let (stdout_trunc, out_trunc) = truncate_string(&stdout, MAX_OUTPUT_CHARS);
    let (stderr_trunc, err_trunc) = truncate_string(&stderr, MAX_OUTPUT_CHARS);
    
    let result = json!({
        "stdout": stdout_trunc,
        "stderr": stderr_trunc,
        "code": code,
        "duration_ms": duration_ms,
        "truncated": out_trunc || err_trunc,
    });
    
    log_tool_call(app, run_id, "run_command", args, &result)?;
    Ok(result)
}

fn detect_runner(repo_path: &Path, explicit: Option<&str>) -> Result<String, String> {
    if let Some(runner) = explicit {
        return Ok(runner.to_string());
    }
    
    // Check for JS package managers
    if repo_path.join("pnpm-lock.yaml").exists() {
        return Ok("pnpm".to_string());
    }
    if repo_path.join("yarn.lock").exists() {
        return Ok("yarn".to_string());
    }
    if repo_path.join("package-lock.json").exists() {
        return Ok("npm".to_string());
    }
    
    // Check for Rust
    if repo_path.join("Cargo.toml").exists() {
        return Ok("cargo".to_string());
    }
    
    // Check for Python
    if repo_path.join("pyproject.toml").exists() || repo_path.join("requirements.txt").exists() {
        return Ok("python".to_string());
    }
    
    Err("Could not detect project type. Specify 'runner' explicitly.".to_string())
}

fn build_command(runner: &str, kind: CommandKind) -> Result<Vec<String>, String> {
    let cmd = match (runner, kind) {
        // JavaScript/TypeScript
        ("pnpm", CommandKind::Tests) => vec!["pnpm", "test"],
        ("pnpm", CommandKind::Lint) => vec!["pnpm", "lint"],
        ("pnpm", CommandKind::Build) => vec!["pnpm", "build"],
        ("npm", CommandKind::Tests) => vec!["npm", "test"],
        ("npm", CommandKind::Lint) => vec!["npm", "run", "lint"],
        ("npm", CommandKind::Build) => vec!["npm", "run", "build"],
        ("yarn", CommandKind::Tests) => vec!["yarn", "test"],
        ("yarn", CommandKind::Lint) => vec!["yarn", "lint"],
        ("yarn", CommandKind::Build) => vec!["yarn", "build"],
        
        // Rust
        ("cargo", CommandKind::Tests) => vec!["cargo", "test"],
        ("cargo", CommandKind::Lint) => vec!["cargo", "clippy", "--", "-D", "warnings"],
        ("cargo", CommandKind::Build) => vec!["cargo", "build"],
        
        // Python
        ("python" | "pytest", CommandKind::Tests) => vec!["pytest"],
        ("python", CommandKind::Lint) => vec!["ruff", "check", "."],
        ("python", CommandKind::Build) => return Err("Python doesn't have a build step".to_string()),
        
        _ => return Err(format!("Unsupported runner '{}' for kind '{:?}'", runner, kind)),
    };
    
    Ok(cmd.iter().map(|s| s.to_string()).collect())
}
