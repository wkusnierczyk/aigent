/// LLM provider implementations.
///
/// Each module provides a struct implementing `LlmProvider` with a
/// `from_env()` constructor that reads API keys from environment variables.
pub mod anthropic;
pub mod google;
pub mod ollama;
pub mod openai;
