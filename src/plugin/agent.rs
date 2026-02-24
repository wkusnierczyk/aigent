//! Agent file (`.md`) validation.

use std::path::Path;
use std::sync::LazyLock;

use regex::Regex;

use crate::diagnostics::{
    Diagnostic, Severity, A001, A002, A003, A004, A005, A006, A007, A008, A009, A010,
};

/// Regex for valid kebab-case agent names.
static KEBAB_CASE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[a-z][a-z0-9]*(-[a-z0-9]+)*$").expect("kebab-case regex"));

/// Required frontmatter fields for agent files.
const REQUIRED_FIELDS: &[&str] = &["name", "description", "model", "color"];

/// Valid model values for agents.
const VALID_MODELS: &[&str] = &["inherit", "sonnet", "opus", "haiku"];

/// Valid color values for agents.
const VALID_COLORS: &[&str] = &["blue", "cyan", "green", "yellow", "magenta", "red"];

/// Generic agent names that warrant a warning.
const GENERIC_NAMES: &[&str] = &["helper", "assistant", "agent", "tool"];

/// Validate an agent `.md` file at the given path.
///
/// Returns a list of diagnostics. Never panics — parse errors are reported
/// as diagnostics.
#[must_use]
pub fn validate_agent(path: &Path) -> Vec<Diagnostic> {
    let mut diags = Vec::new();

    let content = match crate::parser::read_file_checked(path) {
        Ok(c) => c,
        Err(e) => {
            diags.push(Diagnostic::new(
                Severity::Error,
                A001,
                format!("cannot read agent file: {e}"),
            ));
            return diags;
        }
    };

    // A001: Frontmatter must exist
    if !content.trim_start().starts_with("---") {
        diags.push(Diagnostic::new(
            Severity::Error,
            A001,
            "agent file missing frontmatter (no `---` delimiters)",
        ));
        return diags;
    }

    let (metadata, body) = match crate::parser::parse_frontmatter(&content) {
        Ok(result) => result,
        Err(e) => {
            diags.push(Diagnostic::new(
                Severity::Error,
                A001,
                format!("invalid frontmatter: {e}"),
            ));
            return diags;
        }
    };

    // A002: Required fields
    for field in REQUIRED_FIELDS {
        if !metadata.contains_key(*field) {
            diags.push(
                Diagnostic::new(
                    Severity::Error,
                    A002,
                    format!("missing required field `{field}`"),
                )
                .with_field(field),
            );
        }
    }

    // Validate name if present
    if let Some(name_val) = metadata.get("name") {
        if let Some(name) = name_val.as_str() {
            // A003: Name must be kebab-case
            if !KEBAB_CASE_RE.is_match(name) {
                diags.push(
                    Diagnostic::new(
                        Severity::Error,
                        A003,
                        format!("`name` is not valid kebab-case: \"{name}\""),
                    )
                    .with_field("name")
                    .with_suggestion("Use lowercase letters, digits, and hyphens"),
                );
            }

            // A004: Generic name warning
            if GENERIC_NAMES.contains(&name) {
                diags.push(
                    Diagnostic::new(
                        Severity::Warning,
                        A004,
                        format!("`name` is too generic: \"{name}\""),
                    )
                    .with_field("name")
                    .with_suggestion(
                        "Use a descriptive name (e.g., \"code-reviewer\" instead of \"helper\")",
                    ),
                );
            }

            // A005: Name length 3–50 chars
            let len = name.len();
            if !(3..=50).contains(&len) {
                diags.push(
                    Diagnostic::new(
                        Severity::Error,
                        A005,
                        format!("`name` length {len} is outside 3–50 chars"),
                    )
                    .with_field("name"),
                );
            }
        }
    }

    // A006: Description length 10–5000 chars
    if let Some(desc_val) = metadata.get("description") {
        if let Some(desc) = desc_val.as_str() {
            let len = desc.len();
            if !(10..=5000).contains(&len) {
                diags.push(
                    Diagnostic::new(
                        Severity::Error,
                        A006,
                        format!("`description` length {len} is outside 10–5000 chars"),
                    )
                    .with_field("description"),
                );
            }
        }
    }

    // A007: Model must be one of valid values
    if let Some(model_val) = metadata.get("model") {
        if let Some(model) = model_val.as_str() {
            if !VALID_MODELS.contains(&model) {
                diags.push(
                    Diagnostic::new(
                        Severity::Error,
                        A007,
                        format!("`model` is not valid: \"{model}\""),
                    )
                    .with_field("model")
                    .with_suggestion(format!("Valid models: {}", VALID_MODELS.join(", "))),
                );
            }
        }
    }

    // A008: Color must be one of valid values
    if let Some(color_val) = metadata.get("color") {
        if let Some(color) = color_val.as_str() {
            if !VALID_COLORS.contains(&color) {
                diags.push(
                    Diagnostic::new(
                        Severity::Error,
                        A008,
                        format!("`color` is not valid: \"{color}\""),
                    )
                    .with_field("color")
                    .with_suggestion(format!("Valid colors: {}", VALID_COLORS.join(", "))),
                );
            }
        }
    }

    // A009/A010: System prompt (body) checks
    let body_trimmed = body.trim();
    if body_trimmed.is_empty() || body_trimmed.len() < 20 {
        diags.push(
            Diagnostic::new(
                Severity::Error,
                A009,
                format!(
                    "system prompt is {} (minimum 20 chars)",
                    if body_trimmed.is_empty() {
                        "missing".to_string()
                    } else {
                        format!("too short ({} chars)", body_trimmed.len())
                    }
                ),
            )
            .with_suggestion("Add a detailed system prompt describing the agent's behavior"),
        );
    } else if body_trimmed.len() > 10_000 {
        diags.push(
            Diagnostic::new(
                Severity::Warning,
                A010,
                format!(
                    "system prompt is {} chars (recommended max 10,000)",
                    body_trimmed.len()
                ),
            )
            .with_suggestion("Consider splitting into shorter sections or using reference files"),
        );
    }

    diags
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    fn write_agent(content: &str) -> (tempfile::TempDir, std::path::PathBuf) {
        let dir = tempdir().unwrap();
        let path = dir.path().join("my-agent.md");
        fs::write(&path, content).unwrap();
        (dir, path)
    }

    fn valid_agent() -> String {
        "---\nname: code-reviewer\ndescription: Reviews code for bugs and quality issues\nmodel: sonnet\ncolor: blue\n---\nYou are a code reviewer. Analyze code for bugs, security vulnerabilities, and quality issues. Provide actionable feedback.\n".to_string()
    }

    #[test]
    fn valid_agent_no_errors() {
        let (_dir, path) = write_agent(&valid_agent());
        let diags = validate_agent(&path);
        let errors: Vec<_> = diags.iter().filter(|d| d.is_error()).collect();
        assert!(errors.is_empty(), "unexpected errors: {errors:?}");
    }

    #[test]
    fn missing_frontmatter_a001() {
        let (_dir, path) = write_agent("Just a plain file with no frontmatter delimiters.");
        let diags = validate_agent(&path);
        assert!(diags.iter().any(|d| d.code == A001));
    }

    #[test]
    fn invalid_yaml_a001() {
        let (_dir, path) = write_agent("---\n: invalid: yaml:\n---\nBody text here for the agent.");
        let diags = validate_agent(&path);
        assert!(diags.iter().any(|d| d.code == A001));
    }

    #[test]
    fn missing_required_fields_a002() {
        let (_dir, path) = write_agent("---\nname: test-agent\n---\nThis is a system prompt for the agent that is long enough.");
        let diags = validate_agent(&path);
        let a002s: Vec<_> = diags.iter().filter(|d| d.code == A002).collect();
        // Missing: description, model, color
        assert_eq!(a002s.len(), 3);
    }

    #[test]
    fn name_not_kebab_case_a003() {
        let (_dir, path) = write_agent(
            "---\nname: MyAgent\ndescription: A test agent for validation\nmodel: sonnet\ncolor: blue\n---\nThis is a system prompt for the agent that is long enough.",
        );
        let diags = validate_agent(&path);
        assert!(diags.iter().any(|d| d.code == A003));
    }

    #[test]
    fn generic_name_a004() {
        let (_dir, path) = write_agent(
            "---\nname: helper\ndescription: A test agent for validation\nmodel: sonnet\ncolor: blue\n---\nThis is a system prompt for the agent that is long enough.",
        );
        let diags = validate_agent(&path);
        assert!(diags.iter().any(|d| d.code == A004));
    }

    #[test]
    fn name_too_short_a005() {
        let (_dir, path) = write_agent(
            "---\nname: ab\ndescription: A test agent for validation\nmodel: sonnet\ncolor: blue\n---\nThis is a system prompt for the agent that is long enough.",
        );
        let diags = validate_agent(&path);
        assert!(diags.iter().any(|d| d.code == A005));
    }

    #[test]
    fn name_too_long_a005() {
        let long_name = format!("a{}", "-b".repeat(25)); // 51 chars
        let (_dir, path) = write_agent(&format!(
            "---\nname: {long_name}\ndescription: A test agent for validation\nmodel: sonnet\ncolor: blue\n---\nThis is a system prompt for the agent that is long enough."
        ));
        let diags = validate_agent(&path);
        assert!(diags.iter().any(|d| d.code == A005));
    }

    #[test]
    fn description_too_short_a006() {
        let (_dir, path) = write_agent(
            "---\nname: test-agent\ndescription: Short\nmodel: sonnet\ncolor: blue\n---\nThis is a system prompt for the agent that is long enough.",
        );
        let diags = validate_agent(&path);
        assert!(diags.iter().any(|d| d.code == A006));
    }

    #[test]
    fn invalid_model_a007() {
        let (_dir, path) = write_agent(
            "---\nname: test-agent\ndescription: A test agent for validation\nmodel: gpt-4\ncolor: blue\n---\nThis is a system prompt for the agent that is long enough.",
        );
        let diags = validate_agent(&path);
        assert!(diags.iter().any(|d| d.code == A007));
    }

    #[test]
    fn inherit_model_valid() {
        let (_dir, path) = write_agent(
            "---\nname: test-agent\ndescription: A test agent for validation\nmodel: inherit\ncolor: blue\n---\nThis is a system prompt for the agent that is long enough.",
        );
        let diags = validate_agent(&path);
        assert!(!diags.iter().any(|d| d.code == A007));
    }

    #[test]
    fn invalid_color_a008() {
        let (_dir, path) = write_agent(
            "---\nname: test-agent\ndescription: A test agent for validation\nmodel: sonnet\ncolor: orange\n---\nThis is a system prompt for the agent that is long enough.",
        );
        let diags = validate_agent(&path);
        assert!(diags.iter().any(|d| d.code == A008));
    }

    #[test]
    fn missing_body_a009() {
        let (_dir, path) = write_agent(
            "---\nname: test-agent\ndescription: A test agent for validation\nmodel: sonnet\ncolor: blue\n---\n",
        );
        let diags = validate_agent(&path);
        assert!(diags.iter().any(|d| d.code == A009));
    }

    #[test]
    fn short_body_a009() {
        let (_dir, path) = write_agent(
            "---\nname: test-agent\ndescription: A test agent for validation\nmodel: sonnet\ncolor: blue\n---\nToo short.",
        );
        let diags = validate_agent(&path);
        assert!(diags.iter().any(|d| d.code == A009));
    }

    #[test]
    fn long_body_a010() {
        let long_body = "x".repeat(10_001);
        let (_dir, path) = write_agent(&format!(
            "---\nname: test-agent\ndescription: A test agent for validation\nmodel: sonnet\ncolor: blue\n---\n{long_body}"
        ));
        let diags = validate_agent(&path);
        assert!(diags.iter().any(|d| d.code == A010));
    }

    #[test]
    fn nonexistent_file_returns_a001() {
        let diags = validate_agent(Path::new("/nonexistent/agent.md"));
        assert!(diags.iter().any(|d| d.code == A001));
    }

    #[test]
    fn all_valid_models_accepted() {
        for model in VALID_MODELS {
            let (_dir, path) = write_agent(&format!(
                "---\nname: test-agent\ndescription: A test agent for validation\nmodel: {model}\ncolor: blue\n---\nThis is a system prompt for the agent that is long enough."
            ));
            let diags = validate_agent(&path);
            assert!(
                !diags.iter().any(|d| d.code == A007),
                "model {model} should be valid"
            );
        }
    }

    #[test]
    fn all_valid_colors_accepted() {
        for color in VALID_COLORS {
            let (_dir, path) = write_agent(&format!(
                "---\nname: test-agent\ndescription: A test agent for validation\nmodel: sonnet\ncolor: {color}\n---\nThis is a system prompt for the agent that is long enough."
            ));
            let diags = validate_agent(&path);
            assert!(
                !diags.iter().any(|d| d.code == A008),
                "color {color} should be valid"
            );
        }
    }
}
