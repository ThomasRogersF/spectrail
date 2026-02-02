use reqwest::{Client, StatusCode};
use serde_json::Value;
use std::time::Duration;
use backoff::{ExponentialBackoff, future::retry, Error as BackoffError};

use crate::llm::types::*;

pub struct LlmClient {
    http: Client,
    config: LlmConfig,
    api_key: String,
}

impl LlmClient {
    pub fn new(config: LlmConfig, api_key: String) -> Self {
        let http = Client::builder()
            .timeout(Duration::from_secs(120))
            .build()
            .expect("Failed to build HTTP client");
        
        Self { http, config, api_key }
    }

    pub async fn chat_with_tools(
        &self,
        messages: Vec<ChatMessage>,
        tools: Vec<Value>,
    ) -> Result<LlmResponse, LlmError> {
        if self.api_key.is_empty() {
            return Err(LlmError::MissingApiKey);
        }

        let request = OpenAIChatRequest {
            model: self.config.model.clone(),
            messages,
            tools: Some(tools),
            temperature: Some(self.config.temperature),
            max_tokens: Some(self.config.max_tokens),
            stream: false,
        };

        let url = format!("{}/chat/completions", self.config.base_url.trim_end_matches('/'));

        let operation = || async {
            let mut headers = reqwest::header::HeaderMap::new();
            headers.insert(
                reqwest::header::AUTHORIZATION,
                format!("Bearer {}", self.api_key).parse().unwrap(),
            );
            headers.insert(
                reqwest::header::CONTENT_TYPE,
                "application/json".parse().unwrap(),
            );

            // Add extra headers from config
            if let Some(obj) = self.config.extra_headers.as_object() {
                for (key, value) in obj {
                    if let Some(val_str) = value.as_str() {
                        if let (Ok(header_name), Ok(header_value)) = (
                            reqwest::header::HeaderName::from_bytes(key.as_bytes()),
                            val_str.parse::<reqwest::header::HeaderValue>()
                        ) {
                            headers.insert(header_name, header_value);
                        }
                    }
                }
            }

            let response = self.http
                .post(&url)
                .headers(headers)
                .json(&request)
                .send()
                .await
                .map_err(|e| BackoffError::transient(LlmError::Http(e.to_string())))?;

            let status = response.status();

            if status.is_success() {
                let chat_response: OpenAIChatResponse = response
                    .json()
                    .await
                    .map_err(|e| BackoffError::permanent(LlmError::InvalidResponse(e.to_string())))?;
                Ok(chat_response)
            } else {
                let error_text = response.text().await.unwrap_or_default();
                match status {
                    StatusCode::TOO_MANY_REQUESTS => Err(BackoffError::transient(LlmError::RateLimited)),
                    StatusCode::UNAUTHORIZED => Err(BackoffError::permanent(LlmError::Api {
                        status: 401,
                        message: "Invalid API key".to_string(),
                    })),
                    _ if status.as_u16() >= 500 => Err(BackoffError::transient(LlmError::Api {
                        status: status.as_u16(),
                        message: error_text,
                    })),
                    _ => Err(BackoffError::permanent(LlmError::Api {
                        status: status.as_u16(),
                        message: error_text,
                    })),
                }
            }
        };

        let backoff = ExponentialBackoff {
            initial_interval: Duration::from_millis(500),
            max_interval: Duration::from_secs(4),
            max_elapsed_time: Some(Duration::from_secs(30)),
            ..Default::default()
        };

        let result: OpenAIChatResponse = retry(backoff, operation).await?;

        if let Some(choice) = result.choices.into_iter().next() {
            Ok(LlmResponse {
                content: choice.message.content,
                tool_calls: choice.message.tool_calls,
            })
        } else {
            Err(LlmError::InvalidResponse("No choices in response".to_string()))
        }
    }
}
