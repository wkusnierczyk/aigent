//! Command file (`.md`) validation.

use std::path::Path;

use crate::diagnostics::{Diagnostic, Severity, K001, K002, K003, K004, K005, K006, K007};

/// Valid model values for commands (no `inherit`).
const VALID_MODELS: &[&str] = &["sonnet", "opus", "haiku"];

/// Common English verbs for the "starts with verb" heuristic (K004).
const COMMON_VERBS: &[&str] = &[
    "add",
    "analyze",
    "apply",
    "build",
    "check",
    "clean",
    "commit",
    "configure",
    "convert",
    "copy",
    "create",
    "debug",
    "delete",
    "deploy",
    "describe",
    "detect",
    "display",
    "download",
    "edit",
    "enable",
    "execute",
    "export",
    "extract",
    "fetch",
    "find",
    "fix",
    "format",
    "generate",
    "get",
    "help",
    "import",
    "initialize",
    "insert",
    "inspect",
    "install",
    "launch",
    "lint",
    "list",
    "load",
    "log",
    "manage",
    "merge",
    "migrate",
    "monitor",
    "move",
    "open",
    "optimize",
    "output",
    "parse",
    "patch",
    "perform",
    "plan",
    "preview",
    "print",
    "process",
    "publish",
    "pull",
    "push",
    "query",
    "read",
    "refactor",
    "reload",
    "remove",
    "rename",
    "repair",
    "replace",
    "report",
    "reset",
    "resolve",
    "restart",
    "restore",
    "retrieve",
    "review",
    "run",
    "save",
    "scan",
    "search",
    "send",
    "serve",
    "set",
    "setup",
    "show",
    "sort",
    "start",
    "stop",
    "submit",
    "summarize",
    "sync",
    "test",
    "trace",
    "transform",
    "trigger",
    "uninstall",
    "update",
    "upgrade",
    "upload",
    "validate",
    "verify",
    "view",
    "watch",
    "write",
];

/// Validate a command `.md` file at the given path.
///
/// Commands have optional frontmatter. Returns a list of diagnostics.
/// Never panics — parse errors are reported as diagnostics.
#[must_use]
pub fn validate_command(path: &Path) -> Vec<Diagnostic> {
    let mut diags = Vec::new();

    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            diags.push(Diagnostic::new(
                Severity::Error,
                K001,
                format!("cannot read command file: {e}"),
            ));
            return diags;
        }
    };

    // K001: Parse frontmatter (optional — only errors if `---` present but invalid YAML)
    let (metadata, body) = match crate::parser::parse_optional_frontmatter(&content) {
        Ok(result) => result,
        Err(e) => {
            diags.push(Diagnostic::new(
                Severity::Error,
                K001,
                format!("frontmatter syntax error: {e}"),
            ));
            return diags;
        }
    };

    let has_frontmatter = !metadata.is_empty();

    // Only validate frontmatter fields that are present
    if has_frontmatter {
        // K002: Description exceeds 60 chars
        if let Some(desc_val) = metadata.get("description") {
            if let Some(desc) = desc_val.as_str() {
                if desc.len() > 60 {
                    diags.push(
                        Diagnostic::new(
                            Severity::Warning,
                            K002,
                            format!("`description` is {} chars (recommended max 60)", desc.len()),
                        )
                        .with_field("description"),
                    );
                }

                // K004: Description should start with a verb
                let first_word = desc.split_whitespace().next().unwrap_or("");
                let first_lower = first_word.to_lowercase();
                if !first_word.is_empty() && !COMMON_VERBS.contains(&first_lower.as_str()) {
                    diags.push(
                        Diagnostic::new(
                            Severity::Warning,
                            K004,
                            format!(
                                "`description` does not start with a verb: \"{first_word}\""
                            ),
                        )
                        .with_field("description")
                        .with_suggestion(
                            "Start with an imperative verb (e.g., \"Run tests\", \"Generate docs\")",
                        ),
                    );
                }
            }
        } else {
            // K007: Missing description (info, not error)
            diags.push(
                Diagnostic::new(
                    Severity::Info,
                    K007,
                    "missing `description` field (recommended for discoverability)",
                )
                .with_field("description"),
            );
        }

        // K003: Model must be one of valid values
        if let Some(model_val) = metadata.get("model") {
            if let Some(model) = model_val.as_str() {
                if !VALID_MODELS.contains(&model) {
                    diags.push(
                        Diagnostic::new(
                            Severity::Error,
                            K003,
                            format!("`model` is not valid: \"{model}\""),
                        )
                        .with_field("model")
                        .with_suggestion(format!("Valid models: {}", VALID_MODELS.join(", "))),
                    );
                }
            }
        }

        // K006: allowed-tools format check
        if let Some(tools_val) = metadata.get("allowed-tools") {
            // allowed-tools should be a sequence of strings
            if let serde_yaml_ng::Value::Sequence(seq) = tools_val {
                for item in seq {
                    if !item.is_string() {
                        diags.push(
                            Diagnostic::new(
                                Severity::Warning,
                                K006,
                                "items in `allowed-tools` must be strings",
                            )
                            .with_field("allowed-tools"),
                        );
                        break;
                    }
                }
            } else if !tools_val.is_string() {
                // Single string is acceptable, but other types are not
                diags.push(
                    Diagnostic::new(
                        Severity::Warning,
                        K006,
                        "`allowed-tools` should be a list of tool names",
                    )
                    .with_field("allowed-tools"),
                );
            }
        }
    }

    // K005: Body must not be empty
    if body.trim().is_empty() {
        diags.push(
            Diagnostic::new(Severity::Error, K005, "command body is empty")
                .with_suggestion("Add the command prompt text after the frontmatter"),
        );
    }

    diags
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    fn write_command(content: &str) -> (tempfile::TempDir, std::path::PathBuf) {
        let dir = tempdir().unwrap();
        let path = dir.path().join("my-command.md");
        fs::write(&path, content).unwrap();
        (dir, path)
    }

    #[test]
    fn valid_command_with_frontmatter() {
        let (_dir, path) = write_command(
            "---\ndescription: Run project tests\nmodel: sonnet\n---\nRun the test suite and report results.\n",
        );
        let diags = validate_command(&path);
        let errors: Vec<_> = diags.iter().filter(|d| d.is_error()).collect();
        assert!(errors.is_empty(), "unexpected errors: {errors:?}");
    }

    #[test]
    fn valid_command_without_frontmatter() {
        let (_dir, path) = write_command("Just a plain command prompt with instructions.\n");
        let diags = validate_command(&path);
        let errors: Vec<_> = diags.iter().filter(|d| d.is_error()).collect();
        assert!(errors.is_empty(), "unexpected errors: {errors:?}");
    }

    #[test]
    fn invalid_yaml_k001() {
        let (_dir, path) = write_command("---\n: bad: yaml\n---\nBody text.\n");
        let diags = validate_command(&path);
        assert!(diags.iter().any(|d| d.code == K001));
    }

    #[test]
    fn long_description_k002() {
        let long_desc = "a".repeat(61);
        let (_dir, path) = write_command(&format!(
            "---\ndescription: {long_desc}\n---\nCommand body text.\n"
        ));
        let diags = validate_command(&path);
        assert!(diags.iter().any(|d| d.code == K002));
    }

    #[test]
    fn description_at_60_no_k002() {
        let desc = "a".repeat(60);
        let (_dir, path) = write_command(&format!(
            "---\ndescription: {desc}\n---\nCommand body text.\n"
        ));
        let diags = validate_command(&path);
        assert!(!diags.iter().any(|d| d.code == K002));
    }

    #[test]
    fn invalid_model_k003() {
        let (_dir, path) =
            write_command("---\nmodel: gpt-4\ndescription: Run tests\n---\nBody text.\n");
        let diags = validate_command(&path);
        assert!(diags.iter().any(|d| d.code == K003));
    }

    #[test]
    fn inherit_model_invalid_for_commands_k003() {
        let (_dir, path) =
            write_command("---\nmodel: inherit\ndescription: Run tests\n---\nBody text.\n");
        let diags = validate_command(&path);
        assert!(diags.iter().any(|d| d.code == K003));
    }

    #[test]
    fn valid_models_no_k003() {
        for model in VALID_MODELS {
            let (_dir, path) = write_command(&format!(
                "---\nmodel: {model}\ndescription: Run tests\n---\nBody text.\n"
            ));
            let diags = validate_command(&path);
            assert!(
                !diags.iter().any(|d| d.code == K003),
                "model {model} should be valid"
            );
        }
    }

    #[test]
    fn description_not_starting_with_verb_k004() {
        let (_dir, path) = write_command("---\ndescription: The project tests\n---\nBody text.\n");
        let diags = validate_command(&path);
        assert!(diags.iter().any(|d| d.code == K004));
    }

    #[test]
    fn description_starting_with_verb_no_k004() {
        let (_dir, path) = write_command("---\ndescription: Run project tests\n---\nBody text.\n");
        let diags = validate_command(&path);
        assert!(!diags.iter().any(|d| d.code == K004));
    }

    #[test]
    fn empty_body_k005() {
        let (_dir, path) = write_command("---\ndescription: Run tests\n---\n");
        let diags = validate_command(&path);
        assert!(diags.iter().any(|d| d.code == K005));
    }

    #[test]
    fn empty_body_no_frontmatter_k005() {
        let (_dir, path) = write_command("");
        let diags = validate_command(&path);
        assert!(diags.iter().any(|d| d.code == K005));
    }

    #[test]
    fn invalid_allowed_tools_k006() {
        let (_dir, path) =
            write_command("---\ndescription: Run tests\nallowed-tools: 42\n---\nBody text.\n");
        let diags = validate_command(&path);
        assert!(diags.iter().any(|d| d.code == K006));
    }

    #[test]
    fn valid_allowed_tools_list() {
        let (_dir, path) = write_command(
            "---\ndescription: Run tests\nallowed-tools:\n  - Bash\n  - Read\n---\nBody text.\n",
        );
        let diags = validate_command(&path);
        assert!(!diags.iter().any(|d| d.code == K006));
    }

    #[test]
    fn missing_description_k007() {
        let (_dir, path) = write_command("---\nmodel: sonnet\n---\nBody text.\n");
        let diags = validate_command(&path);
        assert!(diags.iter().any(|d| d.code == K007));
    }

    #[test]
    fn no_frontmatter_no_k007() {
        // No frontmatter at all — K007 should not fire (it's about missing description
        // when frontmatter IS present)
        let (_dir, path) = write_command("Just command text.\n");
        let diags = validate_command(&path);
        assert!(!diags.iter().any(|d| d.code == K007));
    }

    #[test]
    fn nonexistent_file_returns_k001() {
        let diags = validate_command(Path::new("/nonexistent/command.md"));
        assert!(diags.iter().any(|d| d.code == K001));
    }
}
