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
//! let diags = aigent::validate(Path::new("my-skill"));
//! let has_errors = diags.iter().any(|d| d.is_error());
//!
//! // Read skill properties
//! let props = aigent::read_properties(Path::new("my-skill")).unwrap();
//! println!("{}", props.name);
//! ```

#![warn(missing_docs)]

/// Skill-to-plugin assembly: packages skills into Claude Code plugins.
pub mod assembler;
/// Skill builder: deterministic and LLM-enhanced skill generation.
pub mod builder;
/// Cross-skill conflict detection for skill collections.
pub mod conflict;
/// Structured diagnostics for validation, linting, and error reporting.
pub mod diagnostics;
/// Error types for skill operations.
pub mod errors;
/// Auto-fix application for fixable diagnostics.
pub mod fixer;
/// SKILL.md formatting: canonical key ordering and markdown cleanup.
pub mod formatter;
/// Symlink-safe filesystem helpers.
pub(crate) mod fs_util;
/// Semantic lint checks for skill quality improvement.
pub mod linter;
/// Data model for SKILL.md frontmatter properties.
pub mod models;
/// SKILL.md frontmatter parser.
pub mod parser;
/// Multi-format prompt generation for LLM injection.
pub mod prompt;
/// Quality scoring for skill best-practices compliance.
pub mod scorer;
/// Directory structure validation for skill packages.
pub mod structure;
/// Fixture-based skill testing: run test suites defined in `tests.yml`.
pub mod test_runner;
/// Skill tester and previewer for evaluation-driven development.
pub mod tester;
/// Skill directory and metadata validator.
pub mod validator;

// Re-export key types at crate root for convenience.
pub use assembler::{assemble_plugin, AssembleOptions, AssembleResult, AssembleWarning};
pub use conflict::{detect_conflicts, detect_conflicts_with_threshold};
#[doc(inline)]
pub use diagnostics::{Diagnostic, Severity, ValidationTarget};
#[doc(inline)]
pub use errors::{AigentError, Result};
pub use fixer::apply_fixes;
pub use formatter::{diff_skill, format_content, format_skill, FormatResult};
pub use linter::lint;
#[doc(inline)]
pub use models::SkillProperties;
pub use parser::{
    find_skill_md, parse_frontmatter, read_body, read_properties, CLAUDE_CODE_KEYS, KNOWN_KEYS,
};
pub use prompt::{
    collect_skills, collect_skills_verbose, estimate_tokens, format_budget, format_entries,
    to_prompt, to_prompt_format, PromptFormat, SkillEntry,
};
pub use scorer::{score, ScoreResult};
pub use structure::validate_structure;
pub use test_runner::{
    format_text as format_test_suite, generate_fixture, run_test_suite, TestSuiteResult,
};
pub use tester::{test_skill, TestResult};
pub use validator::{
    discover_skills, discover_skills_verbose, known_keys_for, validate, validate_metadata,
    validate_metadata_with_target, validate_with_target, DiscoveryWarning,
};

#[doc(inline)]
pub use builder::{
    assess_clarity, build_skill, derive_name, init_skill, interactive_build, BuildResult,
    ClarityAssessment, LlmProvider, SkillSpec, SkillTemplate,
};
