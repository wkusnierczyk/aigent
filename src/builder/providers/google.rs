use std::env;

use serde::{Deserialize, Serialize};

use crate::builder::llm::LlmProvider;
use crate::errors::{AigentError, Result};

/// Default model for Google Gemini.
const DEFAULT_MODEL: &str = "gemini-2.0-flash";

/// Google Generative Language API provider.
pub struct GoogleProvider {
    api_key: String,
    model: String,
}

impl GoogleProvider {
    /// Create a new Google provider from environment variables.
    ///
    /// Reads `GOOGLE_API_KEY` (required) and `GOOGLE_MODEL` (optional,
    /// defaults to `gemini-2.0-flash`).
    pub fn from_env() -> Option<Self> {
        let api_key = env::var("GOOGLE_API_KEY").ok()?;
        if api_key.is_empty() {
            return None;
        }
        let model = env::var("GOOGLE_MODEL")
            .ok()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| DEFAULT_MODEL.to_string());
        Some(Self { api_key, model })
    }
}

#[derive(Serialize)]
struct Part {
    text: String,
}

#[derive(Serialize)]
struct Content {
    parts: Vec<Part>,
}

#[derive(Serialize)]
struct SystemInstruction {
    parts: Vec<Part>,
}

#[derive(Serialize)]
struct RequestBody {
    system_instruction: SystemInstruction,
    contents: Vec<Content>,
}

#[derive(Deserialize)]
struct ResponsePart {
    text: String,
}

#[derive(Deserialize)]
struct ResponseContent {
    parts: Vec<ResponsePart>,
}

#[derive(Deserialize)]
struct Candidate {
    content: ResponseContent,
}

#[derive(Deserialize)]
struct ResponseBody {
    candidates: Vec<Candidate>,
}

impl LlmProvider for GoogleProvider {
    fn generate(&self, system: &str, user: &str) -> Result<String> {
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent",
            self.model
        );

        let body = RequestBody {
            system_instruction: SystemInstruction {
                parts: vec![Part {
                    text: system.to_string(),
                }],
            },
            contents: vec![Content {
                parts: vec![Part {
                    text: user.to_string(),
                }],
            }],
        };

        let mut response = ureq::post(&url)
            .header("Content-Type", "application/json")
            .header("x-goog-api-key", &self.api_key)
            .send_json(&body)
            .map_err(|e| AigentError::Build {
                message: format!("Google API request failed: {e}"),
            })?;

        let resp: ResponseBody =
            response
                .body_mut()
                .read_json()
                .map_err(|e| AigentError::Build {
                    message: format!("Google API response parse failed: {e}"),
                })?;

        resp.candidates
            .into_iter()
            .next()
            .and_then(|c| c.content.parts.into_iter().next())
            .map(|p| p.text)
            .ok_or_else(|| AigentError::Build {
                message: "Google API returned empty candidates".to_string(),
            })
    }
}
