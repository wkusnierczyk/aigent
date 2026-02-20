use std::env;

use serde::{Deserialize, Serialize};

use crate::builder::llm::LlmProvider;
use crate::errors::{AigentError, Result};

/// Default model for OpenAI.
const DEFAULT_MODEL: &str = "gpt-4o";

/// Default API base URL.
const DEFAULT_BASE_URL: &str = "https://api.openai.com/v1";

/// OpenAI Chat Completions API provider.
///
/// Also supports OpenAI-compatible endpoints (vLLM, LM Studio, etc.)
/// via `OPENAI_API_BASE` or `OPENAI_BASE_URL` environment variables.
pub struct OpenAiProvider {
    api_key: String,
    base_url: String,
    model: String,
}

impl OpenAiProvider {
    /// Create a new OpenAI provider from environment variables.
    ///
    /// Reads `OPENAI_API_KEY` (required), `OPENAI_MODEL` (optional, defaults
    /// to `gpt-4o`), and `OPENAI_API_BASE` or `OPENAI_BASE_URL` (optional,
    /// defaults to `https://api.openai.com/v1`).
    pub fn from_env() -> Option<Self> {
        let api_key = env::var("OPENAI_API_KEY").ok()?;
        if api_key.is_empty() {
            return None;
        }
        let base_url = env::var("OPENAI_API_BASE")
            .or_else(|_| env::var("OPENAI_BASE_URL"))
            .ok()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| DEFAULT_BASE_URL.to_string());
        let model = env::var("OPENAI_MODEL")
            .ok()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| DEFAULT_MODEL.to_string());
        Some(Self {
            api_key,
            base_url,
            model,
        })
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
    messages: Vec<Message>,
}

#[derive(Deserialize)]
struct Choice {
    message: ChoiceMessage,
}

#[derive(Deserialize)]
struct ChoiceMessage {
    content: String,
}

#[derive(Deserialize)]
struct ResponseBody {
    choices: Vec<Choice>,
}

impl LlmProvider for OpenAiProvider {
    fn generate(&self, system: &str, user: &str) -> Result<String> {
        let url = format!("{}/chat/completions", self.base_url.trim_end_matches('/'));

        let body = RequestBody {
            model: self.model.clone(),
            messages: vec![
                Message {
                    role: "system".to_string(),
                    content: system.to_string(),
                },
                Message {
                    role: "user".to_string(),
                    content: user.to_string(),
                },
            ],
        };

        let mut response = ureq::post(&url)
            .header("Authorization", &format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .send_json(&body)
            .map_err(|e| AigentError::Build {
                message: format!("OpenAI API request failed: {e}"),
            })?;

        let resp: ResponseBody =
            response
                .body_mut()
                .read_json()
                .map_err(|e| AigentError::Build {
                    message: format!("OpenAI API response parse failed: {e}"),
                })?;

        resp.choices
            .into_iter()
            .next()
            .map(|c| c.message.content)
            .ok_or_else(|| AigentError::Build {
                message: "OpenAI API returned empty choices".to_string(),
            })
    }
}
