use std::env;

use serde::{Deserialize, Serialize};

use crate::builder::llm::LlmProvider;
use crate::errors::{AigentError, Result};

/// Default model for Ollama.
const DEFAULT_MODEL: &str = "llama3.2";

/// Ollama local LLM provider.
///
/// Requires `OLLAMA_HOST` to be set (opt-in, no auto-probe).
pub struct OllamaProvider {
    base_url: String,
    model: String,
}

impl OllamaProvider {
    /// Create a new Ollama provider from environment variables.
    ///
    /// Reads `OLLAMA_HOST` (required — opt-in to avoid latency from
    /// probing localhost) and `OLLAMA_MODEL` (optional, defaults to
    /// `llama3.2`).
    pub fn from_env() -> Option<Self> {
        let base_url = env::var("OLLAMA_HOST").ok()?;
        if base_url.is_empty() {
            return None;
        }
        let model = env::var("OLLAMA_MODEL")
            .ok()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| DEFAULT_MODEL.to_string());
        Some(Self { base_url, model })
    }
}

#[derive(Serialize)]
struct RequestBody {
    model: String,
    system: String,
    prompt: String,
    stream: bool,
}

#[derive(Deserialize)]
struct ResponseBody {
    response: String,
}

impl LlmProvider for OllamaProvider {
    fn generate(&self, system: &str, user: &str) -> Result<String> {
        let url = format!("{}/api/generate", self.base_url.trim_end_matches('/'));

        let body = RequestBody {
            model: self.model.clone(),
            system: system.to_string(),
            prompt: user.to_string(),
            stream: false,
        };

        let mut response = ureq::post(&url)
            .header("Content-Type", "application/json")
            .send_json(&body)
            .map_err(|e| AigentError::Build {
                message: format!("Ollama API request failed: {e}"),
            })?;

        let resp: ResponseBody =
            response
                .body_mut()
                .read_json()
                .map_err(|e| AigentError::Build {
                    message: format!("Ollama API response parse failed: {e}"),
                })?;

        if resp.response.is_empty() {
            return Err(AigentError::Build {
                message: "Ollama API returned empty response".to_string(),
            });
        }

        Ok(resp.response)
    }
}
