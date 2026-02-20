use std::env;

use serde::{Deserialize, Serialize};

use crate::builder::llm::LlmProvider;
use crate::errors::{AigentError, Result};

/// Default model for Anthropic.
const DEFAULT_MODEL: &str = "claude-sonnet-4-20250514";

/// Anthropic Messages API provider.
pub struct AnthropicProvider {
    api_key: String,
    model: String,
}

impl AnthropicProvider {
    /// Create a new Anthropic provider from environment variables.
    ///
    /// Reads `ANTHROPIC_API_KEY` (required) and `ANTHROPIC_MODEL` (optional,
    /// defaults to `claude-sonnet-4-20250514`).
    pub fn from_env() -> Option<Self> {
        let api_key = env::var("ANTHROPIC_API_KEY").ok()?;
        if api_key.is_empty() {
            return None;
        }
        let model = env::var("ANTHROPIC_MODEL")
            .ok()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| DEFAULT_MODEL.to_string());
        Some(Self { api_key, model })
    }
}

#[derive(Serialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Serialize)]
struct RequestBody {
    model: String,
    max_tokens: u32,
    system: String,
    messages: Vec<Message>,
}

#[derive(Deserialize)]
struct ContentBlock {
    text: String,
}

#[derive(Deserialize)]
struct ResponseBody {
    content: Vec<ContentBlock>,
}

impl LlmProvider for AnthropicProvider {
    fn generate(&self, system: &str, user: &str) -> Result<String> {
        let body = RequestBody {
            model: self.model.clone(),
            max_tokens: 1024,
            system: system.to_string(),
            messages: vec![Message {
                role: "user".to_string(),
                content: user.to_string(),
            }],
        };

        let mut response = ureq::post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .send_json(&body)
            .map_err(|e| AigentError::Build {
                message: format!("Anthropic API request failed: {e}"),
            })?;

        let resp: ResponseBody =
            response
                .body_mut()
                .read_json()
                .map_err(|e| AigentError::Build {
                    message: format!("Anthropic API response parse failed: {e}"),
                })?;

        resp.content
            .into_iter()
            .next()
            .map(|b| b.text)
            .ok_or_else(|| AigentError::Build {
                message: "Anthropic API returned empty content".to_string(),
            })
    }
}
