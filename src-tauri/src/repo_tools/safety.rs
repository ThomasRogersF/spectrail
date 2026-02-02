use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;
use tokio::process::Command;
use tokio::time::timeout;

#[derive(Debug, thiserror::Error)]
pub enum SafetyError {
    #[error("Path traversal attempt blocked")]
    PathTraversal,
    #[error("Invalid path: {0}")]
    InvalidPath(String),
    #[error("Command failed: {0}")]
    CommandFailed(String),
    #[error("Timeout")]
    Timeout,
}

/// Sanitize path to ensure it's within repo root
pub fn sanitize_path(repo_root: &Path, rel_path: &str) -> Result<PathBuf, SafetyError> {
    // Reject absolute paths
    if Path::new(rel_path).is_absolute() {
        return Err(SafetyError::PathTraversal);
    }
    
    // Normalize the path - handle both / and \
    let normalized = rel_path.replace('\\', "/");
    let components: Vec<&str> = normalized.split('/').filter(|s| !s.is_empty()).collect();
    
    // Build clean path manually (handles .. correctly)
    let mut clean_path = PathBuf::new();
    for comp in components {
        if comp == "." {
            continue;
        } else if comp == ".." {
            // Pop one directory level if possible
            if !clean_path.pop() {
                // Trying to go above root - block it
                return Err(SafetyError::PathTraversal);
            }
        } else {
            clean_path.push(comp);
        }
    }
    
    let full_path = repo_root.join(&clean_path);
    
    // Canonicalize and verify it's within repo
    // Note: canonicalize requires the path to exist
    let canonical_full = match full_path.canonicalize() {
        Ok(p) => p,
        Err(_) => {
            // Path doesn't exist - still validate it doesn't escape repo
            // Use absolute path for comparison
            let abs_root = repo_root.canonicalize()
                .map_err(|_| SafetyError::InvalidPath("Cannot canonicalize repo root".to_string()))?;
            let abs_full = std::env::current_dir()
                .map_err(|_| SafetyError::InvalidPath("Cannot get current dir".to_string()))?
                .join(&full_path);
            
            // Check if path starts with repo root
            let abs_full_str = abs_full.to_string_lossy();
            let abs_root_str = abs_root.to_string_lossy();
            
            if !abs_full_str.starts_with(&*abs_root_str) {
                return Err(SafetyError::PathTraversal);
            }
            
            return Ok(abs_full);
        }
    };
    
    let canonical_repo = repo_root.canonicalize()
        .map_err(|_| SafetyError::InvalidPath("Cannot canonicalize repo root".to_string()))?;
    
    if !canonical_full.starts_with(&canonical_repo) {
        return Err(SafetyError::PathTraversal);
    }
    
    Ok(canonical_full)
}

/// Truncate string with metadata
pub fn truncate_string(s: &str, max_chars: usize) -> (String, bool) {
    if s.len() <= max_chars {
        (s.to_string(), false)
    } else {
        let truncated: String = s.chars().take(max_chars).collect();
        (truncated, true)
    }
}

/// Safe command spawn with timeout
pub async fn safe_spawn(
    cmd: &str,
    args: &[&str],
    cwd: &Path,
    timeout_secs: u64,
) -> Result<(String, String, i32), SafetyError> {
    let output = timeout(
        Duration::from_secs(timeout_secs),
        Command::new(cmd)
            .args(args)
            .current_dir(cwd)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
    ).await
        .map_err(|_| SafetyError::Timeout)?
        .map_err(|e| SafetyError::CommandFailed(e.to_string()))?;
    
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let code = output.status.code().unwrap_or(-1);
    
    Ok((stdout, stderr, code))
}

/// Check if ripgrep is available
pub fn has_ripgrep() -> bool {
    which::which("rg").is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;
    use tempfile::TempDir;
    
    #[test]
    fn test_truncate_string() {
        let (result, truncated) = truncate_string("hello", 10);
        assert_eq!(result, "hello");
        assert!(!truncated);
        
        let (result, truncated) = truncate_string("hello world", 5);
        assert_eq!(result, "hello");
        assert!(truncated);
    }
    
    #[test]
    fn test_sanitize_path_valid() {
        let temp = TempDir::new().unwrap();
        let root = temp.path();
        
        // Create a file
        let file_path = root.join("test.txt");
        fs::write(&file_path, "test").unwrap();
        
        let result = sanitize_path(root, "test.txt").unwrap();
        assert_eq!(result, file_path.canonicalize().unwrap());
    }
    
    #[test]
    fn test_sanitize_path_traversal() {
        let temp = TempDir::new().unwrap();
        let root = temp.path();
        
        // Attempt to escape repo
        let result = sanitize_path(root, "../../../etc/passwd");
        assert!(result.is_err());
    }
    
    #[test]
    fn test_sanitize_path_absolute() {
        let temp = TempDir::new().unwrap();
        let root = temp.path();
        
        // Absolute path should be rejected
        let result = sanitize_path(root, "/etc/passwd");
        assert!(result.is_err());
    }
}
