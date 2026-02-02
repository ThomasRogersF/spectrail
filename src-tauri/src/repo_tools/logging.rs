use serde_json::Value;
use tauri::AppHandle;
use crate::db;
use crate::models::*;

const MAX_RESULT_CHARS: usize = 200_000;

pub fn log_tool_call(
    app: &AppHandle,
    run_id: &str,
    name: &str,
    args: &Value,
    result: &Value,
) -> Result<(), String> {
    let conn = db::connect(app).map_err(|e| e.to_string())?;
    let id = new_id();
    let created_at = now_iso();
    
    // Truncate result if too large
    let result_str = result.to_string();
    let final_result = if result_str.len() > MAX_RESULT_CHARS {
        let truncated_content = &result_str[..MAX_RESULT_CHARS];
        // Parse to JSON, add truncation metadata
        match serde_json::from_str::<Value>(truncated_content) {
            Ok(mut val) => {
                if let Some(obj) = val.as_object_mut() {
                    obj.insert("_truncated".to_string(), Value::Bool(true));
                    obj.insert("_original_size".to_string(), Value::Number((result_str.len() as i64).into()));
                }
                val.to_string()
            }
            Err(_) => {
                // Can't parse, just wrap it
                serde_json::json!({
                    "_truncated": true,
                    "_original_size": result_str.len(),
                    "_content": truncated_content
                }).to_string()
            }
        }
    } else {
        result_str
    };
    
    conn.execute(
        "INSERT INTO tool_calls (id, run_id, name, args_json, result_json, created_at) 
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        (&id, 
         run_id, 
         name, 
         &args.to_string(), 
         &final_result, 
         &created_at)
    ).map_err(|e| e.to_string())?;
    
    Ok(())
}

pub fn list_tool_calls(app: &AppHandle, run_id: &str) -> Result<Vec<ToolCallRow>, String> {
    let conn = db::connect(app).map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare(
        "SELECT id, run_id, name, args_json, result_json, created_at 
         FROM tool_calls WHERE run_id = ?1 ORDER BY created_at ASC"
    ).map_err(|e| e.to_string())?;
    
    let rows = stmt.query_map([run_id], |r| {
        Ok(ToolCallRow {
            id: r.get(0)?,
            run_id: r.get(1)?,
            name: r.get(2)?,
            args_json: r.get(3)?,
            result_json: r.get(4)?,
            created_at: r.get(5)?,
        })
    }).map_err(|e| e.to_string())?;
    
    let mut out = vec![];
    for row in rows {
        out.push(row.map_err(|e| e.to_string())?);
    }
    Ok(out)
}

fn now_iso() -> String {
    let t = time::OffsetDateTime::now_utc();
    t.format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string())
}

fn new_id() -> String {
    uuid::Uuid::new_v4().to_string()
}
