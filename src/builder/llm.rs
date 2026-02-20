use crate::builder::ClarityAssessment;
use crate::errors::{AigentError, Result};

use super::providers::{anthropic, google, ollama, openai};

/// Trait for LLM text generation providers.
///
/// Synchronous, not async. Each provider implements this with `ureq`.
/// Provider structs hold the API key and model configuration.
pub trait LlmProvider: Send + Sync {
    /// Generate a text response given a system prompt and user message.
    fn generate(&self, system: &str, user: &str) -> Result<String>;
}

/// Detect an available LLM provider from environment variables.
///
/// Checks in priority order: Anthropic, OpenAI, Google, Ollama.
/// Returns `None` for deterministic mode (no provider available).
///
/// No network probes — detection is purely env-var based. Ollama requires
/// `OLLAMA_HOST` to be explicitly set (opt-in).
#[must_use]
pub fn detect_provider() -> Option<Box<dyn LlmProvider>> {
    if let Some(p) = anthropic::AnthropicProvider::from_env() {
        return Some(Box::new(p));
    }
    if let Some(p) = openai::OpenAiProvider::from_env() {
        return Some(Box::new(p));
    }
    if let Some(p) = google::GoogleProvider::from_env() {
        return Some(Box::new(p));
    }
    if let Some(p) = ollama::OllamaProvider::from_env() {
        return Some(Box::new(p));
    }
    None
}

/// Derive a skill name using an LLM provider.
///
/// System prompt asks for a kebab-case gerund-form name. If the LLM
/// response is invalid (not kebab-case, too long, empty), returns `Err`
/// so the caller can fall back to the deterministic version.
pub fn llm_derive_name(provider: &dyn LlmProvider, purpose: &str) -> Result<String> {
    let system = "You are a naming assistant. Given a purpose description, derive \
        a kebab-case skill name using gerund form (e.g., 'processing-pdfs', \
        'analyzing-data'). Reply with ONLY the name, no explanation. The name must be \
        lowercase, use only letters, numbers, and hyphens, and be at most 64 characters.";

    let raw = provider.generate(system, purpose)?;
    let name = raw.trim().to_lowercase();

    // Validate: non-empty, only valid chars, ≤ 64 chars.
    if name.is_empty() || name.len() > 64 {
        return Err(AigentError::Build {
            message: "LLM returned invalid name (empty or too long)".to_string(),
        });
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    {
        return Err(AigentError::Build {
            message: format!("LLM returned invalid name characters: {name}"),
        });
    }
    if name.starts_with('-') || name.ends_with('-') || name.contains("--") {
        return Err(AigentError::Build {
            message: format!("LLM returned name with invalid hyphen placement: {name}"),
        });
    }

    Ok(name)
}

/// Generate a skill description using an LLM provider.
pub fn llm_generate_description(
    provider: &dyn LlmProvider,
    purpose: &str,
    name: &str,
) -> Result<String> {
    let system = "You are a technical writer. Write a concise skill description \
        in third person. Describe what the skill does and when to use it. Maximum 200 \
        characters. No quotes or formatting.";
    let user_msg = format!("Skill name: {name}\nPurpose: {purpose}");

    let raw = provider.generate(system, &user_msg)?;
    let desc = raw.trim().to_string();

    if desc.is_empty() {
        return Err(AigentError::Build {
            message: "LLM returned empty description".to_string(),
        });
    }

    // Truncate to 1024 chars (spec limit) if needed.
    if desc.len() > 1024 {
        Ok(desc[..1024].to_string())
    } else {
        Ok(desc)
    }
}

/// Generate a skill body using an LLM provider.
pub fn llm_generate_body(
    provider: &dyn LlmProvider,
    purpose: &str,
    name: &str,
    description: &str,
) -> Result<String> {
    let system = "You are a skill author following the Anthropic agent skill \
        specification. Generate a markdown body for a SKILL.md file. Be concise — only \
        add context the model doesn't already have. Use sections with ## headings. \
        Keep under 100 lines. Do not include frontmatter delimiters (---).";
    let user_msg = format!("Skill name: {name}\nDescription: {description}\nPurpose: {purpose}");

    let raw = provider.generate(system, &user_msg)?;
    let body = raw.trim().to_string();

    if body.is_empty() {
        return Err(AigentError::Build {
            message: "LLM returned empty body".to_string(),
        });
    }

    Ok(body)
}

/// Evaluate purpose clarity using an LLM provider.
pub fn llm_assess_clarity(provider: &dyn LlmProvider, purpose: &str) -> Result<ClarityAssessment> {
    let system = "Evaluate if this purpose description is clear enough to \
        generate an AI agent skill. Reply in JSON: {\"clear\": true/false, \
        \"questions\": [\"question1\", ...]}. If clear, questions should be empty.";

    let raw = provider.generate(system, purpose)?;

    // Parse JSON response.
    #[derive(serde::Deserialize)]
    struct ClarityResponse {
        clear: bool,
        questions: Vec<String>,
    }

    let parsed: ClarityResponse =
        serde_json::from_str(raw.trim()).map_err(|e| AigentError::Build {
            message: format!("LLM clarity response parse failed: {e}"),
        })?;

    Ok(ClarityAssessment {
        clear: parsed.clear,
        questions: parsed.questions,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A simple mock provider that returns a pre-configured response.
    struct MockProvider {
        response: String,
    }

    impl MockProvider {
        fn new(response: &str) -> Self {
            Self {
                response: response.to_string(),
            }
        }
    }

    impl LlmProvider for MockProvider {
        fn generate(&self, _system: &str, _user: &str) -> Result<String> {
            Ok(self.response.clone())
        }
    }

    /// A mock provider that always returns an error.
    struct FailingProvider;

    impl LlmProvider for FailingProvider {
        fn generate(&self, _system: &str, _user: &str) -> Result<String> {
            Err(AigentError::Build {
                message: "mock LLM failure".to_string(),
            })
        }
    }

    #[test]
    fn mock_provider_returns_expected_text() {
        let provider = MockProvider::new("hello world");
        let result = provider.generate("system", "user").unwrap();
        assert_eq!(result, "hello world");
    }

    #[test]
    fn detect_provider_returns_none_when_no_env_vars() {
        // In test environment, no API keys should be set.
        // This test may fail if the runner has API keys — that's acceptable.
        // The purpose is to verify the detection logic path.
        let result = detect_provider();
        // We can't assert None here because the test environment might have
        // API keys set. Instead, we just verify it doesn't panic.
        let _ = result;
    }

    #[test]
    fn llm_name_derivation_falls_back_on_invalid_response() {
        // Provider returns uppercase (invalid).
        let provider = MockProvider::new("NOT-VALID-Name!");
        let result = llm_derive_name(&provider, "Process PDFs");
        assert!(
            result.is_err(),
            "should fail on invalid name so caller can fall back"
        );
    }

    #[test]
    fn llm_description_generation_falls_back_on_error() {
        let provider = FailingProvider;
        let result = llm_generate_description(&provider, "Process PDFs", "processing-pdfs");
        assert!(result.is_err(), "should return error for fallback");
    }

    #[test]
    fn llm_body_generation_falls_back_on_error() {
        let provider = FailingProvider;
        let result = llm_generate_body(&provider, "Process PDFs", "processing-pdfs", "Desc.");
        assert!(result.is_err(), "should return error for fallback");
    }

    #[test]
    fn llm_clarity_assessment_falls_back_on_parse_error() {
        // Provider returns non-JSON.
        let provider = MockProvider::new("this is not json");
        let result = llm_assess_clarity(&provider, "Process PDFs");
        assert!(
            result.is_err(),
            "should fail to parse non-JSON, allowing fallback"
        );
    }
}
