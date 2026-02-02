use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    #[serde(rename = "type")]
    pub call_type: String,
    pub function: ToolFunction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolFunction {
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    pub provider_name: String,
    pub base_url: String,
    pub model: String,
    pub temperature: f64,
    pub max_tokens: i64,
    pub extra_headers: Value,
}

#[derive(Debug, Clone, Serialize)]
pub struct OpenAIChatRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<i64>,
    pub stream: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OpenAIChatResponse {
    pub id: String,
    pub model: String,
    pub choices: Vec<Choice>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Choice {
    pub index: i64,
    pub message: ChatMessage,
    pub finish_reason: String,
}

#[derive(Debug, Clone)]
pub struct LlmResponse {
    pub content: Option<String>,
    pub tool_calls: Option<Vec<ToolCall>>,
}

#[derive(Debug, thiserror::Error)]
pub enum LlmError {
    #[error("Missing API key. Set SPECTRAIL_API_KEY environment variable.")]
    MissingApiKey,
    #[error("HTTP error: {0}")]
    Http(String),
    #[error("API error {status}: {message}")]
    Api { status: u16, message: String },
    #[error("Invalid response: {0}")]
    InvalidResponse(String),
    #[error("Timeout")]
    Timeout,
    #[error("Rate limited")]
    RateLimited,
}
