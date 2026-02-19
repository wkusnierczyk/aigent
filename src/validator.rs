use std::collections::HashMap;
use std::path::Path;
use std::sync::LazyLock;

use regex::Regex;
use serde_yaml_ng::Value;
use unicode_normalization::UnicodeNormalization;

use crate::parser::{find_skill_md, parse_frontmatter, KNOWN_KEYS};

/// Reserved words that must not appear as hyphen-delimited segments in a skill name.
const RESERVED_WORDS: &[&str] = &["anthropic", "claude"];

/// Regex for detecting XML/HTML tags in strings.
///
/// Matches patterns like `<script>`, `</div>`, `<img/>`, etc.
/// Does not false-positive on `<` in expressions like `a < b`.
static XML_TAG_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"<[a-zA-Z/][^>]*>").expect("XML tag regex must compile"));

/// Returns `true` if the string contains XML/HTML tags.
fn contains_xml_tags(s: &str) -> bool {
    XML_TAG_RE.is_match(s)
}

/// Validate a skill name after NFKC normalization.
fn validate_name(name: &str, dir: Option<&Path>) -> Vec<String> {
    let mut errors = Vec::new();
    let normalized: String = name.nfkc().collect();

    // 1. Non-empty.
    if normalized.is_empty() {
        errors.push("name must not be empty".to_string());
        return errors;
    }

    // 2. Max length.
    if normalized.chars().count() > 64 {
        errors.push("name exceeds 64 characters".to_string());
    }

    // 3. Character validation: a-z, 0-9, hyphen, or alphabetic non-uppercase.
    for c in normalized.chars() {
        if c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' {
            continue;
        }
        if c.is_alphabetic() && !c.is_uppercase() {
            continue;
        }
        errors.push(format!("name contains invalid character: '{c}'"));
    }

    // 4. No XML tags in name.
    if contains_xml_tags(&normalized) {
        errors.push("name contains XML/HTML tags".to_string());
    }

    // 5. No leading hyphen.
    if normalized.starts_with('-') {
        errors.push("name must not start with a hyphen".to_string());
    }

    // 6. No trailing hyphen.
    if normalized.ends_with('-') {
        errors.push("name must not end with a hyphen".to_string());
    }

    // 7. No consecutive hyphens.
    if normalized.contains("--") {
        errors.push("name contains consecutive hyphens".to_string());
    }

    // 8. Reserved words — checked as hyphen-delimited segments.
    for word in RESERVED_WORDS {
        if normalized.split('-').any(|seg| seg == *word) {
            errors.push(format!("name contains reserved word: '{word}'"));
        }
    }

    // 9. Directory name match.
    if let Some(dir) = dir {
        if let Some(dir_name) = dir.file_name().and_then(|n| n.to_str()) {
            let dir_normalized: String = dir_name.nfkc().collect();
            if normalized != dir_normalized {
                errors.push(format!(
                    "name '{normalized}' does not match directory name '{dir_normalized}'"
                ));
            }
        }
    }

    errors
}

/// Validate a skill description.
fn validate_description(description: &str) -> Vec<String> {
    let mut errors = Vec::new();

    if description.is_empty() {
        errors.push("description must not be empty".to_string());
        return errors;
    }

    if description.chars().count() > 1024 {
        errors.push("description exceeds 1024 characters".to_string());
    }

    if contains_xml_tags(description) {
        errors.push("description contains XML/HTML tags".to_string());
    }

    errors
}

/// Validate a compatibility string.
fn validate_compatibility(compatibility: &str) -> Vec<String> {
    let mut errors = Vec::new();

    if compatibility.chars().count() > 500 {
        errors.push("compatibility exceeds 500 characters".to_string());
    }

    errors
}

/// Validate skill metadata against the Anthropic specification.
///
/// Expects raw `parse_frontmatter` output — the full `HashMap` before
/// known-key extraction. **Not** suitable for use on
/// `SkillProperties.metadata` (which has known keys already removed).
///
/// Returns a list of error/warning strings (empty = valid).
/// Warnings are prefixed with `"warning: "`.
#[must_use]
pub fn validate_metadata(metadata: &HashMap<String, Value>, dir: Option<&Path>) -> Vec<String> {
    let mut messages = Vec::new();

    // 1. Validate name.
    match metadata.get("name") {
        Some(Value::String(name)) => {
            messages.extend(validate_name(name, dir));
        }
        Some(_) => messages.push("`name` must be a string".to_string()),
        None => messages.push("missing required field `name`".to_string()),
    }

    // 2. Validate description.
    match metadata.get("description") {
        Some(Value::String(desc)) => {
            messages.extend(validate_description(desc));
        }
        Some(_) => messages.push("`description` must be a string".to_string()),
        None => messages.push("missing required field `description`".to_string()),
    }

    // 3. Validate compatibility if present.
    if let Some(val) = metadata.get("compatibility") {
        match val {
            Value::String(s) => messages.extend(validate_compatibility(s)),
            _ => messages.push("`compatibility` must be a string".to_string()),
        }
    }

    // 4. Warn about unexpected metadata keys.
    for key in metadata.keys() {
        if !KNOWN_KEYS.contains(&key.as_str()) {
            messages.push(format!("warning: unexpected metadata field: '{key}'"));
        }
    }

    messages
}

/// Validate a skill directory: find SKILL.md, parse, and check all rules.
///
/// Returns a list of error/warning strings (empty = valid).
/// Warnings are prefixed with `"warning: "`.
#[must_use]
pub fn validate(dir: &Path) -> Vec<String> {
    // 1. Find SKILL.md.
    let path = match find_skill_md(dir) {
        Some(p) => p,
        None => return vec!["SKILL.md not found".to_string()],
    };

    // 2. Read the file.
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) => return vec![format!("IO error: {e}")],
    };

    // 3. Parse frontmatter.
    let (metadata, body) = match parse_frontmatter(&content) {
        Ok(result) => result,
        Err(e) => return vec![e.to_string()],
    };

    // 4. Validate metadata.
    let mut messages = validate_metadata(&metadata, Some(dir));

    // 5. Body-length warning.
    let line_count = body.lines().count();
    if line_count > 500 {
        messages.push(format!(
            "warning: body exceeds 500 lines ({line_count} lines)"
        ));
    }

    messages
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    /// Build a metadata HashMap from key-value string pairs.
    fn make_metadata(pairs: &[(&str, &str)]) -> HashMap<String, Value> {
        pairs
            .iter()
            .map(|(k, v)| ((*k).to_string(), Value::String((*v).to_string())))
            .collect()
    }

    /// Create a temp dir with a named subdirectory containing a SKILL.md.
    /// Returns the parent TempDir (for lifetime) and the path to the subdirectory.
    fn make_skill_dir(name: &str, frontmatter: &str) -> (tempfile::TempDir, std::path::PathBuf) {
        let parent = tempdir().unwrap();
        let dir = parent.path().join(name);
        fs::create_dir(&dir).unwrap();
        fs::write(dir.join("SKILL.md"), frontmatter).unwrap();
        (parent, dir)
    }

    // ── validate_metadata tests ──────────────────────────────────────

    #[test]
    fn valid_metadata_all_fields() {
        let meta = make_metadata(&[
            ("name", "my-skill"),
            ("description", "A valid skill"),
            ("license", "MIT"),
            ("compatibility", "claude-3"),
            ("allowed-tools", "Bash, Read"),
        ]);
        let errors = validate_metadata(&meta, None);
        assert!(errors.is_empty(), "expected no errors, got: {errors:?}");
    }

    #[test]
    fn missing_name() {
        let meta = make_metadata(&[("description", "desc")]);
        let errors = validate_metadata(&meta, None);
        assert!(errors.iter().any(|e| e.contains("name")));
    }

    #[test]
    fn missing_description() {
        let meta = make_metadata(&[("name", "test")]);
        let errors = validate_metadata(&meta, None);
        assert!(errors.iter().any(|e| e.contains("description")));
    }

    #[test]
    fn empty_name() {
        let meta = make_metadata(&[("name", ""), ("description", "desc")]);
        let errors = validate_metadata(&meta, None);
        assert!(errors.iter().any(|e| e.contains("name must not be empty")));
    }

    #[test]
    fn empty_description() {
        let meta = make_metadata(&[("name", "test"), ("description", "")]);
        let errors = validate_metadata(&meta, None);
        assert!(errors
            .iter()
            .any(|e| e.contains("description must not be empty")));
    }

    #[test]
    fn name_too_long() {
        let long_name: String = "a".repeat(65);
        let meta = make_metadata(&[("name", &long_name), ("description", "desc")]);
        let errors = validate_metadata(&meta, None);
        assert!(errors.iter().any(|e| e.contains("exceeds 64 characters")));
    }

    #[test]
    fn name_exactly_64_chars() {
        let name: String = "a".repeat(64);
        let meta = make_metadata(&[("name", &name), ("description", "desc")]);
        let errors = validate_metadata(&meta, None);
        let real_errors: Vec<_> = errors
            .iter()
            .filter(|e| !e.starts_with("warning: "))
            .collect();
        assert!(
            real_errors.is_empty(),
            "expected no errors, got: {real_errors:?}"
        );
    }

    #[test]
    fn name_with_uppercase() {
        let meta = make_metadata(&[("name", "MySkill"), ("description", "desc")]);
        let errors = validate_metadata(&meta, None);
        assert!(errors.iter().any(|e| e.contains("invalid character")));
    }

    #[test]
    fn name_with_leading_hyphen() {
        let meta = make_metadata(&[("name", "-my-skill"), ("description", "desc")]);
        let errors = validate_metadata(&meta, None);
        assert!(errors
            .iter()
            .any(|e| e.contains("must not start with a hyphen")));
    }

    #[test]
    fn name_with_trailing_hyphen() {
        let meta = make_metadata(&[("name", "my-skill-"), ("description", "desc")]);
        let errors = validate_metadata(&meta, None);
        assert!(errors
            .iter()
            .any(|e| e.contains("must not end with a hyphen")));
    }

    #[test]
    fn name_with_consecutive_hyphens() {
        let meta = make_metadata(&[("name", "my--skill"), ("description", "desc")]);
        let errors = validate_metadata(&meta, None);
        assert!(errors.iter().any(|e| e.contains("consecutive hyphens")));
    }

    #[test]
    fn name_with_invalid_characters() {
        let meta = make_metadata(&[("name", "my_skill!"), ("description", "desc")]);
        let errors = validate_metadata(&meta, None);
        assert!(errors.iter().any(|e| e.contains("invalid character")));
    }

    #[test]
    fn name_contains_reserved_anthropic() {
        let meta = make_metadata(&[("name", "my-anthropic-skill"), ("description", "desc")]);
        let errors = validate_metadata(&meta, None);
        assert!(errors
            .iter()
            .any(|e| e.contains("reserved word: 'anthropic'")));
    }

    #[test]
    fn name_contains_reserved_claude() {
        let meta = make_metadata(&[("name", "claude-helper"), ("description", "desc")]);
        let errors = validate_metadata(&meta, None);
        assert!(errors.iter().any(|e| e.contains("reserved word: 'claude'")));
    }

    #[test]
    fn name_does_not_match_directory() {
        let (_parent, dir) = make_skill_dir(
            "other-name",
            "---\nname: my-skill\ndescription: desc\n---\n",
        );
        let meta = make_metadata(&[("name", "my-skill"), ("description", "desc")]);
        let errors = validate_metadata(&meta, Some(&dir));
        assert!(errors
            .iter()
            .any(|e| e.contains("does not match directory")));
    }

    #[test]
    fn name_matches_directory() {
        let (_parent, dir) =
            make_skill_dir("my-skill", "---\nname: my-skill\ndescription: desc\n---\n");
        let meta = make_metadata(&[("name", "my-skill"), ("description", "desc")]);
        let errors = validate_metadata(&meta, Some(&dir));
        let real_errors: Vec<_> = errors
            .iter()
            .filter(|e| !e.starts_with("warning: "))
            .collect();
        assert!(
            real_errors.is_empty(),
            "expected no errors, got: {real_errors:?}"
        );
    }

    #[test]
    fn description_too_long() {
        let long_desc: String = "a".repeat(1025);
        let meta = make_metadata(&[("name", "test"), ("description", &long_desc)]);
        let errors = validate_metadata(&meta, None);
        assert!(errors
            .iter()
            .any(|e| e.contains("description exceeds 1024")));
    }

    #[test]
    fn description_exactly_1024_chars() {
        let desc: String = "a".repeat(1024);
        let meta = make_metadata(&[("name", "test"), ("description", &desc)]);
        let errors = validate_metadata(&meta, None);
        let real_errors: Vec<_> = errors
            .iter()
            .filter(|e| !e.starts_with("warning: "))
            .collect();
        assert!(
            real_errors.is_empty(),
            "expected no errors, got: {real_errors:?}"
        );
    }

    #[test]
    fn description_with_xml_tags() {
        let meta = make_metadata(&[
            ("name", "test"),
            ("description", "A <script>alert</script> skill"),
        ]);
        let errors = validate_metadata(&meta, None);
        assert!(errors
            .iter()
            .any(|e| e.contains("description contains XML/HTML")));
    }

    #[test]
    fn name_with_xml_tags() {
        let meta = make_metadata(&[("name", "<img/>skill"), ("description", "desc")]);
        let errors = validate_metadata(&meta, None);
        assert!(errors.iter().any(|e| e.contains("name contains XML/HTML")));
    }

    #[test]
    fn compatibility_too_long() {
        let long_compat: String = "a".repeat(501);
        let meta = make_metadata(&[
            ("name", "test"),
            ("description", "desc"),
            ("compatibility", &long_compat),
        ]);
        let errors = validate_metadata(&meta, None);
        assert!(errors
            .iter()
            .any(|e| e.contains("compatibility exceeds 500")));
    }

    #[test]
    fn compatibility_exactly_500_chars() {
        let compat: String = "a".repeat(500);
        let meta = make_metadata(&[
            ("name", "test"),
            ("description", "desc"),
            ("compatibility", &compat),
        ]);
        let errors = validate_metadata(&meta, None);
        let real_errors: Vec<_> = errors
            .iter()
            .filter(|e| !e.starts_with("warning: "))
            .collect();
        assert!(
            real_errors.is_empty(),
            "expected no errors, got: {real_errors:?}"
        );
    }

    #[test]
    fn unexpected_metadata_field_warning() {
        let meta = make_metadata(&[
            ("name", "test"),
            ("description", "desc"),
            ("custom-field", "value"),
        ]);
        let errors = validate_metadata(&meta, None);
        let warnings: Vec<_> = errors
            .iter()
            .filter(|e| e.starts_with("warning: "))
            .collect();
        assert!(
            warnings.iter().any(|w| w.contains("custom-field")),
            "expected warning about custom-field, got: {warnings:?}"
        );
    }

    #[test]
    fn all_optional_fields_no_warning() {
        let meta = make_metadata(&[
            ("name", "test"),
            ("description", "desc"),
            ("license", "MIT"),
            ("compatibility", "claude-3"),
            ("allowed-tools", "Bash"),
        ]);
        let errors = validate_metadata(&meta, None);
        assert!(errors.is_empty(), "expected no messages, got: {errors:?}");
    }

    // ── i18n / Unicode tests ─────────────────────────────────────────

    #[test]
    fn chinese_characters_accepted() {
        let meta = make_metadata(&[("name", "技能工具"), ("description", "desc")]);
        let errors = validate_metadata(&meta, None);
        let real_errors: Vec<_> = errors
            .iter()
            .filter(|e| !e.starts_with("warning: "))
            .collect();
        assert!(
            real_errors.is_empty(),
            "expected no errors, got: {real_errors:?}"
        );
    }

    #[test]
    fn russian_lowercase_with_hyphens_accepted() {
        let meta = make_metadata(&[("name", "навык-тест"), ("description", "desc")]);
        let errors = validate_metadata(&meta, None);
        let real_errors: Vec<_> = errors
            .iter()
            .filter(|e| !e.starts_with("warning: "))
            .collect();
        assert!(
            real_errors.is_empty(),
            "expected no errors, got: {real_errors:?}"
        );
    }

    #[test]
    fn uppercase_cyrillic_rejected() {
        let meta = make_metadata(&[("name", "Навык"), ("description", "desc")]);
        let errors = validate_metadata(&meta, None);
        assert!(
            errors.iter().any(|e| e.contains("invalid character")),
            "expected error for uppercase Cyrillic, got: {errors:?}"
        );
    }

    #[test]
    fn nfkc_normalization_applied() {
        // ﬁ (U+FB01, Latin Small Ligature Fi) normalizes to "fi" under NFKC.
        let meta = make_metadata(&[("name", "s\u{FB01}le"), ("description", "desc")]);
        let errors = validate_metadata(&meta, None);
        let real_errors: Vec<_> = errors
            .iter()
            .filter(|e| !e.starts_with("warning: "))
            .collect();
        assert!(
            real_errors.is_empty(),
            "NFKC-normalized 'sﬁle' → 'sfile' should be valid, got: {real_errors:?}"
        );
    }

    // ── validate (full pipeline) tests ───────────────────────────────

    #[test]
    fn validate_valid_skill_directory() {
        let (_parent, dir) = make_skill_dir(
            "my-skill",
            "---\nname: my-skill\ndescription: A valid skill\n---\n# Body\n",
        );
        let errors = validate(&dir);
        assert!(errors.is_empty(), "expected no errors, got: {errors:?}");
    }

    #[test]
    fn validate_nonexistent_path() {
        let dir = std::path::Path::new("/nonexistent/path/that/does/not/exist");
        let errors = validate(dir);
        assert!(!errors.is_empty());
    }

    #[test]
    fn validate_missing_skill_md() {
        let dir = tempdir().unwrap();
        let errors = validate(dir.path());
        assert!(errors.iter().any(|e| e.contains("SKILL.md not found")));
    }

    #[test]
    fn validate_body_over_500_lines_warning() {
        let body: String = (0..501)
            .map(|i| format!("line {i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let content = format!("---\nname: my-skill\ndescription: desc\n---\n{body}\n");
        let (_parent, dir) = make_skill_dir("my-skill", &content);
        let errors = validate(&dir);
        let warnings: Vec<_> = errors
            .iter()
            .filter(|e| e.starts_with("warning: "))
            .collect();
        assert!(
            warnings
                .iter()
                .any(|w| w.contains("body exceeds 500 lines")),
            "expected body warning, got: {warnings:?}"
        );
    }

    #[test]
    fn validate_body_at_500_lines_no_warning() {
        let body: String = (0..500)
            .map(|i| format!("line {i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let content = format!("---\nname: my-skill\ndescription: desc\n---\n{body}\n");
        let (_parent, dir) = make_skill_dir("my-skill", &content);
        let errors = validate(&dir);
        let warnings: Vec<_> = errors
            .iter()
            .filter(|e| e.starts_with("warning: "))
            .collect();
        assert!(
            warnings.is_empty(),
            "expected no warnings, got: {warnings:?}"
        );
    }

    #[test]
    fn validate_multiple_errors_collected() {
        // Empty name + empty description = multiple errors in one pass.
        let meta = make_metadata(&[("name", ""), ("description", "")]);
        let errors = validate_metadata(&meta, None);
        let real_errors: Vec<_> = errors
            .iter()
            .filter(|e| !e.starts_with("warning: "))
            .collect();
        assert!(
            real_errors.len() >= 2,
            "expected at least 2 errors, got: {real_errors:?}"
        );
    }

    // ── Reserved word edge case tests ────────────────────────────────

    #[test]
    fn reserved_word_as_substring_accepted() {
        // "claudette" contains "claude" as substring but NOT as a segment.
        let meta = make_metadata(&[("name", "claudette"), ("description", "desc")]);
        let errors = validate_metadata(&meta, None);
        let reserved_errors: Vec<_> = errors
            .iter()
            .filter(|e| e.contains("reserved word"))
            .collect();
        assert!(
            reserved_errors.is_empty(),
            "segment-based matching should accept 'claudette', got: {reserved_errors:?}"
        );
    }

    #[test]
    fn reserved_word_as_exact_segment_rejected() {
        // "my-claude-tool" has "claude" as an exact hyphen-delimited segment.
        let meta = make_metadata(&[("name", "my-claude-tool"), ("description", "desc")]);
        let errors = validate_metadata(&meta, None);
        assert!(
            errors.iter().any(|e| e.contains("reserved word: 'claude'")),
            "expected reserved word error, got: {errors:?}"
        );
    }
}
