//! LLM Client — gọi AI API (OpenAI / AWS Bedrock).
//!
//! Hỗ trợ:
//! - OpenAI Chat Completions API (GPT-4o, GPT-4o-mini)
//! - AWS Bedrock InvokeModel (Claude 3 Haiku/Sonnet) — placeholder cho tương lai
//!
//! Security:
//! - API key không được log
//! - Response được validate trước khi parse

use std::time::Duration;

use reqwest::Client;
use serde::{Deserialize, Serialize};

use super::{AiConfig, AiError, AiProvider};

/// HTTP client cho LLM API calls.
pub struct LlmClient {
    http_client: Client,
    config: AiConfig,
}

// ── OpenAI API Types ───────────────────────────────────────────────────

#[derive(Debug, Serialize)]
struct OpenAiRequest {
    model: String,
    messages: Vec<OpenAiMessage>,
    max_tokens: u32,
    temperature: f32,
    response_format: OpenAiResponseFormat,
}

#[derive(Debug, Serialize)]
struct OpenAiMessage {
    role: String,
    content: String,
}

#[derive(Debug, Serialize)]
struct OpenAiResponseFormat {
    #[serde(rename = "type")]
    format_type: String,
}

#[derive(Debug, Deserialize)]
struct OpenAiResponse {
    choices: Vec<OpenAiChoice>,
}

#[derive(Debug, Deserialize)]
struct OpenAiChoice {
    message: OpenAiChoiceMessage,
}

#[derive(Debug, Deserialize)]
struct OpenAiChoiceMessage {
    content: Option<String>,
}

// ── Implementation ─────────────────────────────────────────────────────

impl LlmClient {
    /// Tạo LLM client mới từ AiConfig.
    pub fn new(config: AiConfig) -> Self {
        let http_client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_secs))
            .build()
            .expect("failed to build HTTP client");

        Self { http_client, config }
    }

    /// Gửi prompt đến LLM và nhận response text.
    ///
    /// # Errors
    /// - `AiError::Disabled` — nếu AI feature bị tắt
    /// - `AiError::ServiceUnavailable` — nếu API call thất bại
    /// - `AiError::Timeout` — nếu quá thời gian
    /// - `AiError::RateLimited` — nếu bị rate limit bởi provider
    /// - `AiError::ParseError` — nếu response không đúng format
    pub async fn analyze(&self, system_prompt: &str, user_prompt: &str) -> Result<String, AiError> {
        if !self.config.enabled {
            return Err(AiError::Disabled);
        }

        match &self.config.provider {
            AiProvider::OpenAI { api_key, model, base_url } => {
                self.call_openai(api_key, model, base_url, system_prompt, user_prompt).await
            }
            AiProvider::AwsBedrock { .. } => {
                // Placeholder — sẽ implement khi cần
                Err(AiError::ServiceUnavailable("AWS Bedrock not yet implemented".to_string()))
            }
        }
    }

    /// Gọi OpenAI Chat Completions API.
    async fn call_openai(
        &self,
        api_key: &str,
        model: &str,
        base_url: &str,
        system_prompt: &str,
        user_prompt: &str,
    ) -> Result<String, AiError> {
        let url = format!("{}/chat/completions", base_url.trim_end_matches('/'));

        let request_body = OpenAiRequest {
            model: model.to_string(),
            messages: vec![
                OpenAiMessage {
                    role: "system".to_string(),
                    content: system_prompt.to_string(),
                },
                OpenAiMessage {
                    role: "user".to_string(),
                    content: user_prompt.to_string(),
                },
            ],
            max_tokens: self.config.max_tokens,
            temperature: 0.1, // Low temperature cho deterministic analysis
            response_format: OpenAiResponseFormat {
                format_type: "json_object".to_string(),
            },
        };

        let response = self
            .http_client
            .post(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    AiError::Timeout(self.config.timeout_secs)
                } else {
                    AiError::ServiceUnavailable(e.to_string())
                }
            })?;

        let status = response.status();

        if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
            return Err(AiError::RateLimited(60));
        }

        if !status.is_success() {
            let error_body = response.text().await.unwrap_or_default();
            tracing::error!(
                status = %status,
                "AI API request failed (response body not logged for security)"
            );
            return Err(AiError::ServiceUnavailable(format!(
                "API returned status {status}: {error_body}"
            )));
        }

        let api_response: OpenAiResponse = response
            .json()
            .await
            .map_err(|e| AiError::ParseError(format!("failed to parse API response: {e}")))?;

        let content = api_response
            .choices
            .into_iter()
            .next()
            .and_then(|c| c.message.content)
            .ok_or_else(|| AiError::ParseError("empty response from AI".to_string()))?;

        Ok(content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_disabled_returns_error() {
        let config = AiConfig {
            enabled: false,
            provider: AiProvider::OpenAI {
                api_key: String::new(),
                model: "gpt-4o-mini".to_string(),
                base_url: "https://api.openai.com/v1".to_string(),
            },
            timeout_secs: 30,
            max_tokens: 2000,
            rate_limit_per_user: 5,
        };

        let client = LlmClient::new(config);
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(client.analyze("system", "user"));
        assert!(matches!(result, Err(AiError::Disabled)));
    }
}
