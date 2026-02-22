use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde_yaml_ng::Value;

use crate::errors::{AigentError, Result};
use crate::fs_util::is_regular_file;
use crate::models::SkillProperties;

/// Maximum file size for SKILL.md and related files (1 MiB).
const MAX_FILE_SIZE: u64 = 1_048_576;

/// Reads a file with a size check, returning an error if the file exceeds 1 MiB.
///
/// This prevents memory exhaustion from maliciously large files.
pub(crate) fn read_file_checked(path: &Path) -> Result<String> {
    let metadata = std::fs::metadata(path).map_err(|e| AigentError::Parse {
        message: format!("cannot read {}: {e}", path.display()),
    })?;
    if metadata.len() > MAX_FILE_SIZE {
        return Err(AigentError::Parse {
            message: format!("file exceeds 1 MiB size limit: {}", path.display()),
        });
    }
    std::fs::read_to_string(path).map_err(|e| AigentError::Parse {
        message: format!("cannot read {}: {e}", path.display()),
    })
}

/// Locate SKILL.md in a directory (prefer uppercase over lowercase).
#[must_use]
pub fn find_skill_md(dir: &Path) -> Option<PathBuf> {
    let uppercase = dir.join("SKILL.md");
    if is_regular_file(&uppercase) {
        return Some(uppercase);
    }
    let lowercase = dir.join("skill.md");
    if is_regular_file(&lowercase) {
        return Some(lowercase);
    }
    None
}

/// Extract YAML frontmatter between `---` delimiters.
///
/// Returns `(metadata_map, body_text)`.
///
/// Delimiter matching uses `trim_end()`, so trailing whitespace on `---` lines
/// is tolerated (e.g., `"---  "` or `"---\t"`). Leading whitespace is **not**
/// trimmed — `"  ---"` does not match.
///
/// All mapping keys must be strings; non-string keys (e.g., integer `42:`) are
/// rejected as a `Parse` error.
///
/// # Errors
///
/// - `AigentError::Parse` if the content does not start with `---`, the closing
///   `---` delimiter is missing, the YAML parses to a non-mapping value, or a
///   mapping key is not a string.
/// - `AigentError::Yaml` if the YAML between delimiters has syntax errors
///   (propagated naturally via `?` to preserve line/column info).
pub fn parse_frontmatter(content: &str) -> Result<(HashMap<String, Value>, String)> {
    let mut lines = content.lines().enumerate();

    // Step 1: Verify content starts with `---`.
    match lines.next() {
        Some((_, line)) if line.trim_end() == "---" => {}
        _ => {
            return Err(AigentError::Parse {
                message: "content does not start with `---`".to_string(),
            });
        }
    }

    // Step 2: Find closing `---` delimiter.
    let mut yaml_end_line = None;
    for (i, line) in &mut lines {
        if line.trim_end() == "---" {
            yaml_end_line = Some(i);
            break;
        }
    }
    let yaml_end_line = yaml_end_line.ok_or_else(|| AigentError::Parse {
        message: "closing `---` delimiter not found".to_string(),
    })?;

    // Step 3: Extract YAML between delimiters and parse.
    // Collect lines 1..yaml_end_line from original content.
    let yaml_str: String = content
        .lines()
        .skip(1)
        .take(yaml_end_line - 1)
        .collect::<Vec<_>>()
        .join("\n");

    // The `?` operator converts serde_yaml_ng::Error → AigentError::Yaml via #[from].
    let parsed: Value = serde_yaml_ng::from_str(&yaml_str)?;

    // Step 4: Verify parsed value is a mapping.
    let mapping = match parsed {
        Value::Mapping(m) => m,
        _ => {
            return Err(AigentError::Parse {
                message: "frontmatter YAML is not a mapping".to_string(),
            });
        }
    };

    // Step 5: Convert to HashMap<String, Value>, rejecting non-string keys.
    let mut map = HashMap::new();
    for (k, v) in mapping {
        let key = match k {
            Value::String(s) => s,
            other => {
                return Err(AigentError::Parse {
                    message: format!("frontmatter contains non-string key: {other:?}"),
                });
            }
        };
        map.insert(key, v);
    }

    // Step 6: Extract body (everything after closing `---`).
    // Calculate byte offset of the body.
    let body_lines: Vec<&str> = content.lines().skip(yaml_end_line + 1).collect();
    let body = if body_lines.is_empty() {
        String::new()
    } else {
        // Join with newlines; preserve trailing newline if original had one.
        let joined = body_lines.join("\n");
        if content.ends_with('\n') {
            format!("{joined}\n")
        } else {
            joined
        }
    };

    Ok((map, body))
}

/// Parse optional YAML frontmatter from markdown content.
///
/// If the content starts with `---`, delegates to [`parse_frontmatter`].
/// Otherwise, returns an empty metadata map and the full content as body.
///
/// Used by command file validation where frontmatter is optional.
pub fn parse_optional_frontmatter(content: &str) -> Result<(HashMap<String, Value>, String)> {
    if content.starts_with("---") {
        parse_frontmatter(content)
    } else {
        Ok((HashMap::new(), content.to_string()))
    }
}

/// Known frontmatter keys that map to typed `SkillProperties` fields.
///
/// Used by both the parser (to extract known fields) and the validator
/// (to detect unexpected metadata keys). Single source of truth.
pub const KNOWN_KEYS: &[&str] = &[
    "name",
    "description",
    "license",
    "compatibility",
    "allowed-tools",
];

/// Claude Code extension fields (recognized with `--target claude-code`).
///
/// These fields are not part of the base Anthropic specification but are
/// recognized by Claude Code. Placed alongside `KNOWN_KEYS` as both define
/// known metadata fields.
pub const CLAUDE_CODE_KEYS: &[&str] = &[
    "disable-model-invocation",
    "user-invocable",
    "context",
    "agent",
    "model",
    "hooks",
    "argument-hint",
];

/// Extract a required string field from metadata.
///
/// Returns `AigentError::Validation` if the key is missing or not a string.
fn require_string(
    metadata: &HashMap<String, Value>,
    key: &str,
) -> std::result::Result<String, AigentError> {
    use crate::diagnostics::{Diagnostic, Severity, E000};
    match metadata.get(key) {
        Some(Value::String(s)) => Ok(s.clone()),
        Some(_) => Err(AigentError::Validation {
            errors: vec![Diagnostic::new(
                Severity::Error,
                E000,
                format!("`{key}` must be a string"),
            )],
        }),
        None => Err(AigentError::Validation {
            errors: vec![Diagnostic::new(
                Severity::Error,
                E000,
                format!("missing required field `{key}`"),
            )],
        }),
    }
}

/// Extract an optional string field from metadata.
///
/// Returns `AigentError::Validation` if the key exists but is not a string.
fn optional_string(
    metadata: &HashMap<String, Value>,
    key: &str,
) -> std::result::Result<Option<String>, AigentError> {
    use crate::diagnostics::{Diagnostic, Severity, E000};
    match metadata.get(key) {
        Some(Value::String(s)) => Ok(Some(s.clone())),
        Some(_) => Err(AigentError::Validation {
            errors: vec![Diagnostic::new(
                Severity::Error,
                E000,
                format!("`{key}` must be a string"),
            )],
        }),
        None => Ok(None),
    }
}

/// Full pipeline: find file → read → parse → validate required fields → return properties.
///
/// # Errors
///
/// - `AigentError::Parse` if no SKILL.md is found or frontmatter is malformed.
/// - `AigentError::Io` if the file cannot be read.
/// - `AigentError::Yaml` if the YAML has syntax errors.
/// - `AigentError::Validation` if required fields are missing or have wrong types.
pub fn read_properties(dir: &Path) -> Result<SkillProperties> {
    // Step 1: Find SKILL.md.
    let path = find_skill_md(dir).ok_or_else(|| AigentError::Parse {
        message: "SKILL.md not found in directory".to_string(),
    })?;

    // Step 2: Read file with size check.
    let content = read_file_checked(&path)?;

    // Step 3: Parse frontmatter.
    let (mut metadata, _body) = parse_frontmatter(&content)?;

    // Step 4: Extract and validate required fields.
    let name = require_string(&metadata, "name")?;
    let description = require_string(&metadata, "description")?;

    // Step 5: Extract optional string fields.
    let license = optional_string(&metadata, "license")?;
    let compatibility = optional_string(&metadata, "compatibility")?;
    let allowed_tools = optional_string(&metadata, "allowed-tools")?;

    // Step 6: Remove known keys; remaining entries become metadata.
    for key in KNOWN_KEYS {
        metadata.remove(*key);
    }

    // Step 7: If metadata is empty, set to None.
    let extra = if metadata.is_empty() {
        None
    } else {
        Some(metadata)
    };

    // Step 8: Construct and return.
    Ok(SkillProperties {
        name,
        description,
        license,
        compatibility,
        allowed_tools,
        metadata: extra,
    })
}

/// Read the markdown body (post-frontmatter) from a skill directory.
///
/// Locates the SKILL.md file, reads its content, parses the frontmatter,
/// and returns the body portion. Returns `Ok(String::new())` when the
/// file has valid frontmatter but no body after the closing `---`.
///
/// # Errors
///
/// - `AigentError::Parse` if no SKILL.md is found in the directory.
/// - `AigentError::Parse` if the file cannot be read or exceeds 1 MiB.
/// - `AigentError::Yaml` or `AigentError::Parse` if frontmatter parsing fails.
pub fn read_body(dir: &Path) -> Result<String> {
    let path = find_skill_md(dir).ok_or_else(|| AigentError::Parse {
        message: "no SKILL.md found".to_string(),
    })?;
    let content = read_file_checked(&path)?;
    let (_, body) = parse_frontmatter(&content)?;
    Ok(body)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    /// Write a SKILL.md (uppercase) into a temp dir and return the dir.
    fn write_skill_md(content: &str) -> tempfile::TempDir {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("SKILL.md"), content).unwrap();
        dir
    }

    // ── find_skill_md tests ──────────────────────────────────────────

    #[test]
    fn find_skill_md_uppercase_exists() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("SKILL.md"), "---\n---\n").unwrap();
        let result = find_skill_md(dir.path());
        assert_eq!(result, Some(dir.path().join("SKILL.md")));
    }

    #[test]
    fn find_skill_md_lowercase_only() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("skill.md"), "---\n---\n").unwrap();
        let result = find_skill_md(dir.path());
        // On case-insensitive filesystems (macOS), the uppercase check
        // succeeds even for a lowercase file, so we just verify it's found.
        assert!(result.is_some());
        let path = result.unwrap();
        assert!(path.is_file());
    }

    #[test]
    fn find_skill_md_prefers_uppercase() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("SKILL.md"), "upper").unwrap();
        // On case-insensitive filesystems both names point to the same file.
        // We verify that when the uppercase file exists, the uppercase path
        // is returned regardless.
        let result = find_skill_md(dir.path());
        assert_eq!(result, Some(dir.path().join("SKILL.md")));
    }

    #[test]
    fn find_skill_md_neither_exists() {
        let dir = tempdir().unwrap();
        let result = find_skill_md(dir.path());
        assert!(result.is_none());
    }

    #[cfg(unix)]
    #[test]
    fn find_skill_md_ignores_symlink() {
        let dir = tempdir().unwrap();
        // Create a real SKILL.md elsewhere and symlink to it.
        let target = dir.path().join("real_skill.md");
        fs::write(&target, "---\nname: test\n---\n").unwrap();
        let link = dir.path().join("SKILL.md");
        std::os::unix::fs::symlink(&target, &link).unwrap();
        // find_skill_md should ignore the symlink and return None.
        let result = find_skill_md(dir.path());
        assert!(result.is_none(), "symlinked SKILL.md should be ignored");
    }

    // ── parse_frontmatter tests ──────────────────────────────────────

    #[test]
    fn parse_frontmatter_valid_with_body() {
        let content = "---\nname: my-skill\ndescription: A skill\n---\n# Body\n\nHello world\n";
        let (meta, body) = parse_frontmatter(content).unwrap();
        assert_eq!(meta["name"], Value::String("my-skill".to_string()));
        assert_eq!(meta["description"], Value::String("A skill".to_string()));
        assert!(body.contains("# Body"));
        assert!(body.contains("Hello world"));
    }

    #[test]
    fn parse_frontmatter_valid_with_empty_body() {
        let content = "---\nname: test\n---\n";
        let (meta, body) = parse_frontmatter(content).unwrap();
        assert_eq!(meta["name"], Value::String("test".to_string()));
        assert!(body.is_empty());
    }

    #[test]
    fn parse_frontmatter_no_opening_delimiter() {
        let content = "name: test\n---\n";
        let err = parse_frontmatter(content).unwrap_err();
        assert!(matches!(err, AigentError::Parse { .. }));
        assert!(err.to_string().contains("does not start with"));
    }

    #[test]
    fn parse_frontmatter_missing_closing_delimiter() {
        let content = "---\nname: test\ndescription: foo\n";
        let err = parse_frontmatter(content).unwrap_err();
        assert!(matches!(err, AigentError::Parse { .. }));
        assert!(err.to_string().contains("closing"));
    }

    #[test]
    fn parse_frontmatter_invalid_yaml_syntax() {
        let content = "---\n: :\n  :\n   :\n---\n";
        let err = parse_frontmatter(content).unwrap_err();
        assert!(matches!(err, AigentError::Yaml(_)));
    }

    #[test]
    fn parse_frontmatter_non_mapping_yaml() {
        let content = "---\n- item1\n- item2\n---\n";
        let err = parse_frontmatter(content).unwrap_err();
        assert!(matches!(err, AigentError::Parse { .. }));
        assert!(err.to_string().contains("not a mapping"));
    }

    #[test]
    fn parse_frontmatter_nested_values() {
        let content = "---\nname: test\nconfig:\n  debug: true\n  level: 3\n---\n";
        let (meta, _) = parse_frontmatter(content).unwrap();
        let config = &meta["config"];
        assert!(config.is_mapping());
    }

    #[test]
    fn parse_frontmatter_kebab_case_preserved() {
        let content = "---\nallowed-tools: Bash, Read\n---\n";
        let (meta, _) = parse_frontmatter(content).unwrap();
        assert_eq!(
            meta["allowed-tools"],
            Value::String("Bash, Read".to_string())
        );
        // Verify the key is kebab-case, not snake_case.
        assert!(meta.contains_key("allowed-tools"));
        assert!(!meta.contains_key("allowed_tools"));
    }

    #[test]
    fn parse_frontmatter_trailing_whitespace_on_delimiters() {
        // `trim_end()` means trailing spaces/tabs on `---` lines are tolerated.
        let content = "---  \nname: test\n---\t\nBody\n";
        let (meta, body) = parse_frontmatter(content).unwrap();
        assert_eq!(meta["name"], Value::String("test".to_string()));
        assert!(body.contains("Body"));
    }

    #[test]
    fn parse_frontmatter_non_string_key_rejected() {
        // Integer keys in YAML are rejected as a Parse error.
        let content = "---\n42: value\n---\n";
        let err = parse_frontmatter(content).unwrap_err();
        assert!(matches!(err, AigentError::Parse { .. }));
        assert!(err.to_string().contains("non-string key"));
    }

    #[test]
    fn parse_frontmatter_dashes_in_multiline_value() {
        // A `---` indented inside a YAML multiline value should NOT be
        // treated as the closing delimiter.
        let content = "---\nname: test\ndescription: >\n  This has\n  ---\n  dashes inside\n---\n";
        let (meta, _) = parse_frontmatter(content).unwrap();
        assert_eq!(meta["name"], Value::String("test".to_string()));
        let desc = meta["description"].as_str().unwrap();
        assert!(desc.contains("---"));
    }

    // ── read_properties tests ────────────────────────────────────────

    #[test]
    fn read_properties_valid_all_fields() {
        let content = "\
---
name: my-skill
description: A test skill
license: MIT
compatibility: claude-3
allowed-tools: Bash, Read
env: prod
---
# Body
";
        let dir = write_skill_md(content);
        let props = read_properties(dir.path()).unwrap();
        assert_eq!(props.name, "my-skill");
        assert_eq!(props.description, "A test skill");
        assert_eq!(props.license, Some("MIT".to_string()));
        assert_eq!(props.compatibility, Some("claude-3".to_string()));
        assert_eq!(props.allowed_tools, Some("Bash, Read".to_string()));
        // Extra metadata should contain "env".
        let meta = props.metadata.as_ref().unwrap();
        assert_eq!(meta["env"], Value::String("prod".to_string()));
    }

    #[test]
    fn read_properties_allowed_tools_parsed() {
        let content = "---\nname: test\ndescription: desc\nallowed-tools: Bash, Read\n---\n";
        let dir = write_skill_md(content);
        let props = read_properties(dir.path()).unwrap();
        assert_eq!(props.allowed_tools, Some("Bash, Read".to_string()));
    }

    #[test]
    fn read_properties_known_keys_absent_from_metadata() {
        let content = "\
---
name: test
description: desc
license: MIT
compatibility: claude-3
allowed-tools: Bash
custom-key: value
---
";
        let dir = write_skill_md(content);
        let props = read_properties(dir.path()).unwrap();
        let meta = props.metadata.as_ref().unwrap();
        // Known keys must NOT appear in metadata.
        assert!(!meta.contains_key("name"));
        assert!(!meta.contains_key("description"));
        assert!(!meta.contains_key("license"));
        assert!(!meta.contains_key("compatibility"));
        assert!(!meta.contains_key("allowed-tools"));
        // Unknown key should be in metadata.
        assert!(meta.contains_key("custom-key"));
    }

    #[test]
    fn read_properties_missing_skill_md() {
        let dir = tempdir().unwrap();
        let err = read_properties(dir.path()).unwrap_err();
        assert!(matches!(err, AigentError::Parse { .. }));
        assert!(err.to_string().contains("not found"));
    }

    #[test]
    fn read_properties_missing_name() {
        let content = "---\ndescription: desc\n---\n";
        let dir = write_skill_md(content);
        let err = read_properties(dir.path()).unwrap_err();
        assert!(matches!(err, AigentError::Validation { .. }));
        assert!(err.to_string().contains("name"));
    }

    #[test]
    fn read_properties_missing_description() {
        let content = "---\nname: test\n---\n";
        let dir = write_skill_md(content);
        let err = read_properties(dir.path()).unwrap_err();
        assert!(matches!(err, AigentError::Validation { .. }));
        assert!(err.to_string().contains("description"));
    }

    #[test]
    fn read_properties_required_fields_only() {
        let content = "---\nname: test\ndescription: desc\n---\n";
        let dir = write_skill_md(content);
        let props = read_properties(dir.path()).unwrap();
        assert_eq!(props.name, "test");
        assert_eq!(props.description, "desc");
        assert!(props.license.is_none());
        assert!(props.compatibility.is_none());
        assert!(props.allowed_tools.is_none());
        assert!(props.metadata.is_none());
    }

    #[test]
    fn read_properties_empty_metadata_is_none() {
        // When there are no unknown keys, metadata should be None, not Some({}).
        let content = "---\nname: test\ndescription: desc\nlicense: MIT\n---\n";
        let dir = write_skill_md(content);
        let props = read_properties(dir.path()).unwrap();
        assert!(props.metadata.is_none());
    }

    // ── read_file_checked tests ───────────────────────────────────────

    #[test]
    fn read_file_checked_succeeds_for_normal_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("normal.md");
        fs::write(&path, "hello world").unwrap();
        let content = read_file_checked(&path).unwrap();
        assert_eq!(content, "hello world");
    }

    #[test]
    fn read_file_checked_rejects_oversized_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("huge.md");
        // Create a file that exceeds 1 MiB (1_048_576 bytes).
        let data = vec![b'x'; 1_048_577];
        fs::write(&path, &data).unwrap();
        let err = read_file_checked(&path).unwrap_err();
        assert!(matches!(err, AigentError::Parse { .. }));
        assert!(
            err.to_string().contains("file exceeds 1 MiB size limit"),
            "unexpected error message: {err}"
        );
    }

    #[test]
    fn read_file_checked_allows_exactly_1mib() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("exact.md");
        let data = vec![b'x'; 1_048_576];
        fs::write(&path, &data).unwrap();
        // Exactly 1 MiB should succeed (the check is >, not >=).
        let result = read_file_checked(&path);
        assert!(result.is_ok());
    }

    #[test]
    fn read_file_checked_returns_error_for_nonexistent_file() {
        let path = std::path::Path::new("/nonexistent/path/that/does/not/exist.md");
        let err = read_file_checked(path).unwrap_err();
        assert!(matches!(err, AigentError::Parse { .. }));
        assert!(
            err.to_string().contains("cannot read"),
            "unexpected error message: {err}"
        );
    }

    // ── read_body tests ───────────────────────────────────────────────

    #[test]
    fn read_body_valid_skill_returns_body() {
        let content = "---\nname: test\ndescription: desc\n---\n# Body\n\nHello world\n";
        let dir = write_skill_md(content);
        let body = read_body(dir.path()).unwrap();
        assert!(body.contains("# Body"));
        assert!(body.contains("Hello world"));
    }

    #[test]
    fn read_body_no_skill_md_returns_err() {
        let dir = tempdir().unwrap();
        let err = read_body(dir.path()).unwrap_err();
        assert!(matches!(err, AigentError::Parse { .. }));
    }

    #[test]
    fn read_body_empty_body_returns_ok_empty() {
        let content = "---\nname: test\ndescription: desc\n---\n";
        let dir = write_skill_md(content);
        let body = read_body(dir.path()).unwrap();
        assert!(body.is_empty());
    }

    #[test]
    fn read_body_error_message_contains_no_skill_md() {
        let dir = tempdir().unwrap();
        let err = read_body(dir.path()).unwrap_err();
        assert!(
            err.to_string().contains("no SKILL.md found"),
            "error message should contain 'no SKILL.md found', got: {}",
            err
        );
    }
}
