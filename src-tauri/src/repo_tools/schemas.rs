use serde_json::{json, Value};

pub fn repo_tool_schemas() -> Vec<Value> {
    vec![
        list_files_schema(),
        read_file_schema(),
        grep_schema(),
        git_status_schema(),
        git_diff_schema(),
        git_log_short_schema(),
        run_command_schema(),
    ]
}

fn list_files_schema() -> Value {
    json!({
        "type": "function",
        "function": {
            "name": "list_files",
            "description": "List files in the repository, respecting .gitignore. Returns relative paths.",
            "parameters": {
                "type": "object",
                "properties": {
                    "project_id": {
                        "type": "string",
                        "description": "Project ID to operate on"
                    },
                    "globs": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Optional glob patterns to filter files"
                    },
                    "max_files": {
                        "type": "integer",
                        "description": "Maximum files to return (default 2000)"
                    }
                },
                "required": ["project_id"]
            }
        }
    })
}

fn read_file_schema() -> Value {
    json!({
        "type": "function",
        "function": {
            "name": "read_file",
            "description": "Read contents of a file within the repository. Large files are truncated.",
            "parameters": {
                "type": "object",
                "properties": {
                    "project_id": {
                        "type": "string",
                        "description": "Project ID"
                    },
                    "path": {
                        "type": "string",
                        "description": "Relative path to file within repo"
                    },
                    "max_bytes": {
                        "type": "integer",
                        "description": "Max bytes to read (default 200000)"
                    }
                },
                "required": ["project_id", "path"]
            }
        }
    })
}

fn grep_schema() -> Value {
    json!({
        "type": "function",
        "function": {
            "name": "grep",
            "description": "Search for text patterns in repository files. Uses ripgrep if available.",
            "parameters": {
                "type": "object",
                "properties": {
                    "project_id": {
                        "type": "string",
                        "description": "Project ID"
                    },
                    "query": {
                        "type": "string",
                        "description": "Search pattern"
                    },
                    "path": {
                        "type": "string",
                        "description": "Optional subdirectory to search within"
                    },
                    "max_results": {
                        "type": "integer",
                        "description": "Max matches to return (default 200)"
                    }
                },
                "required": ["project_id", "query"]
            }
        }
    })
}

fn git_status_schema() -> Value {
    json!({
        "type": "function",
        "function": {
            "name": "git_status",
            "description": "Get git status of the repository including branch info.",
            "parameters": {
                "type": "object",
                "properties": {
                    "project_id": {
                        "type": "string",
                        "description": "Project ID"
                    }
                },
                "required": ["project_id"]
            }
        }
    })
}

fn git_diff_schema() -> Value {
    json!({
        "type": "function",
        "function": {
            "name": "git_diff",
            "description": "Get git diff of unstaged or staged changes.",
            "parameters": {
                "type": "object",
                "properties": {
                    "project_id": {
                        "type": "string",
                        "description": "Project ID"
                    },
                    "staged": {
                        "type": "boolean",
                        "description": "Show staged changes instead of unstaged"
                    }
                },
                "required": ["project_id"]
            }
        }
    })
}

fn git_log_short_schema() -> Value {
    json!({
        "type": "function",
        "function": {
            "name": "git_log_short",
            "description": "Get recent commit history in concise format.",
            "parameters": {
                "type": "object",
                "properties": {
                    "project_id": {
                        "type": "string",
                        "description": "Project ID"
                    },
                    "max_commits": {
                        "type": "integer",
                        "description": "Number of commits to retrieve (default 10)"
                    }
                },
                "required": ["project_id"]
            }
        }
    })
}

fn run_command_schema() -> Value {
    json!({
        "type": "function",
        "function": {
            "name": "run_command",
            "description": "Run allowlisted test, lint, or build commands. Auto-detects package manager.",
            "parameters": {
                "type": "object",
                "properties": {
                    "project_id": {
                        "type": "string",
                        "description": "Project ID"
                    },
                    "kind": {
                        "type": "string",
                        "enum": ["tests", "lint", "build"],
                        "description": "Type of command to run"
                    },
                    "runner": {
                        "type": "string",
                        "enum": ["pnpm", "npm", "yarn", "cargo", "pytest"],
                        "description": "Optional explicit runner (auto-detected if not provided)"
                    }
                },
                "required": ["project_id", "kind"]
            }
        }
    })
}
