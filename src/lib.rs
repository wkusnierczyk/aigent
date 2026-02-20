//! # aigent
//!
//! A Rust library for managing AI agent skill definitions (SKILL.md files).
//!
//! Implements the [Anthropic agent skill specification](https://platform.claude.com/docs/en/agents-and-tools/agent-skills/best-practices)
//! with validation, prompt generation, and skill building capabilities.
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use std::path::Path;
//!
//! // Validate a skill directory
//! let errors = aigent::validate(Path::new("my-skill"));
//! assert!(errors.is_empty());
//!
//! // Read skill properties
//! let props = aigent::read_properties(Path::new("my-skill")).unwrap();
//! println!("{}", props.name);
//! ```

#![warn(missing_docs)]

/// Skill builder: deterministic and LLM-enhanced skill generation.
pub mod builder;
/// Error types for skill operations.
pub mod errors;
/// Data model for SKILL.md frontmatter properties.
pub mod models;
/// SKILL.md frontmatter parser.
pub mod parser;
/// XML prompt generation for LLM injection.
pub mod prompt;
/// Skill directory and metadata validator.
pub mod validator;

// Re-export key types at crate root for convenience.
#[doc(inline)]
pub use errors::{AigentError, Result};
#[doc(inline)]
pub use models::SkillProperties;
pub use parser::{find_skill_md, parse_frontmatter, read_properties, KNOWN_KEYS};
pub use prompt::to_prompt;
pub use validator::{validate, validate_metadata};

#[doc(inline)]
pub use builder::{
    assess_clarity, build_skill, derive_name, init_skill, BuildResult, ClarityAssessment,
    LlmProvider, SkillSpec,
};
