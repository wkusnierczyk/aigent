//! LLM provider implementations.
//!
//! Each module provides a struct implementing `LlmProvider` with a
//! `from_env()` constructor that reads API keys from environment variables.

/// Anthropic Claude API provider.
pub mod anthropic;
/// Google Gemini API provider.
pub mod google;
/// Ollama local model provider.
pub mod ollama;
/// OpenAI API provider.
pub mod openai;
