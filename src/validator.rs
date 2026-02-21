use std::collections::HashMap;
use std::path::Path;
use std::sync::LazyLock;

use regex::Regex;
use serde_yaml_ng::Value;
use unicode_normalization::UnicodeNormalization;

use crate::diagnostics::{
    Diagnostic, Severity, ValidationTarget, E000, E001, E002, E003, E004, E005, E006, E007, E009,
    E010, E011, E012, E013, E014, E015, E016, E017, E018, W001, W002,
};
use crate::fs_util::{is_regular_dir, is_regular_file};
use crate::parser::{
    find_skill_md, parse_frontmatter, read_file_checked, CLAUDE_CODE_KEYS, KNOWN_KEYS,
};

/// A warning collected during skill discovery when a path cannot be read or parsed.
#[derive(Debug, Clone)]
pub struct DiscoveryWarning {
    /// The path that caused the warning.
    pub path: std::path::PathBuf,
    /// Human-readable description of the issue.
    pub message: String,
}

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

/// Returns the set of known keys for the given validation target.
#[must_use]
pub fn known_keys_for(target: ValidationTarget) -> Vec<&'static str> {
    match target {
        ValidationTarget::Standard => KNOWN_KEYS.to_vec(),
        ValidationTarget::ClaudeCode => {
            let mut keys = KNOWN_KEYS.to_vec();
            keys.extend_from_slice(CLAUDE_CODE_KEYS);
            keys
        }
        ValidationTarget::Permissive => vec![],
    }
}

/// Collapse consecutive hyphens into a single hyphen.
fn collapse_hyphens(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut prev_hyphen = false;
    for c in s.chars() {
        if c == '-' {
            if !prev_hyphen {
                result.push(c);
            }
            prev_hyphen = true;
        } else {
            result.push(c);
            prev_hyphen = false;
        }
    }
    result
}

/// Validate a skill name after NFKC normalization.
fn validate_name(name: &str, dir: Option<&Path>) -> Vec<Diagnostic> {
    let mut diags = Vec::new();
    let normalized: String = name.nfkc().collect();

    // 1. Non-empty.
    if normalized.is_empty() {
        diags.push(
            Diagnostic::new(Severity::Error, E001, "name must not be empty").with_field("name"),
        );
        return diags;
    }

    // 2. Max length.
    if normalized.chars().count() > 64 {
        // Find the last hyphen at or before position 64 to truncate cleanly.
        let truncated: String = {
            let s: String = normalized.chars().take(64).collect();
            if let Some(pos) = s.rfind('-') {
                s[..pos].to_string()
            } else {
                s
            }
        };
        diags.push(
            Diagnostic::new(Severity::Error, E002, "name exceeds 64 characters")
                .with_field("name")
                .with_suggestion(format!("Truncate to: '{truncated}'")),
        );
    }

    // 3. Character validation: a-z, 0-9, hyphen, or alphabetic non-uppercase.
    //    Collect invalid chars and uppercase chars separately so we can emit
    //    one E003 diagnostic per category rather than one per character.
    let mut invalid_chars: Vec<char> = Vec::new();
    let mut has_uppercase = false;
    for c in normalized.chars() {
        if c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' {
            continue;
        }
        if c.is_alphabetic() && !c.is_uppercase() {
            continue;
        }
        if c.is_uppercase() {
            has_uppercase = true;
        } else {
            invalid_chars.push(c);
        }
    }
    // Emit one diagnostic for uppercase characters.
    if has_uppercase {
        let lowered = normalized.to_lowercase();
        diags.push(
            Diagnostic::new(Severity::Error, E003, "name contains uppercase characters")
                .with_field("name")
                .with_suggestion(format!("Use lowercase: '{lowered}'")),
        );
    }
    // Emit one diagnostic per truly invalid (non-uppercase) character.
    for c in invalid_chars {
        diags.push(
            Diagnostic::new(
                Severity::Error,
                E003,
                format!("name contains invalid character: '{c}'"),
            )
            .with_field("name"),
        );
    }

    // 4. No leading hyphen.
    if normalized.starts_with('-') {
        diags.push(
            Diagnostic::new(Severity::Error, E004, "name must not start with a hyphen")
                .with_field("name"),
        );
    }

    // 5. No trailing hyphen.
    if normalized.ends_with('-') {
        diags.push(
            Diagnostic::new(Severity::Error, E005, "name must not end with a hyphen")
                .with_field("name"),
        );
    }

    // 6. No consecutive hyphens.
    if normalized.contains("--") {
        let collapsed = collapse_hyphens(&normalized);
        diags.push(
            Diagnostic::new(Severity::Error, E006, "name contains consecutive hyphens")
                .with_field("name")
                .with_suggestion(format!("Remove consecutive hyphens: '{collapsed}'")),
        );
    }

    // 7. Reserved words — checked as hyphen-delimited segments.
    for word in RESERVED_WORDS {
        if normalized.split('-').any(|seg| seg == *word) {
            diags.push(
                Diagnostic::new(
                    Severity::Error,
                    E007,
                    format!("name contains reserved word: '{word}'"),
                )
                .with_field("name"),
            );
        }
    }

    // 8. Directory name match.
    if let Some(dir) = dir {
        if let Some(dir_name) = dir.file_name().and_then(|n| n.to_str()) {
            let dir_normalized: String = dir_name.nfkc().collect();
            if normalized != dir_normalized {
                diags.push(
                    Diagnostic::new(
                        Severity::Error,
                        E009,
                        format!(
                            "name '{normalized}' does not match directory name '{dir_normalized}'"
                        ),
                    )
                    .with_field("name"),
                );
            }
        }
    }

    diags
}

/// Validate a skill description.
fn validate_description(description: &str) -> Vec<Diagnostic> {
    let mut diags = Vec::new();

    if description.is_empty() {
        diags.push(
            Diagnostic::new(Severity::Error, E010, "description must not be empty")
                .with_field("description"),
        );
        return diags;
    }

    if description.chars().count() > 1024 {
        diags.push(
            Diagnostic::new(Severity::Error, E011, "description exceeds 1024 characters")
                .with_field("description"),
        );
    }

    if contains_xml_tags(description) {
        diags.push(
            Diagnostic::new(Severity::Error, E012, "description contains XML/HTML tags")
                .with_field("description")
                .with_suggestion("Remove XML tags from description"),
        );
    }

    diags
}

/// Validate a compatibility string.
fn validate_compatibility(compatibility: &str) -> Vec<Diagnostic> {
    let mut diags = Vec::new();

    if compatibility.chars().count() > 500 {
        diags.push(
            Diagnostic::new(
                Severity::Error,
                E013,
                "compatibility exceeds 500 characters",
            )
            .with_field("compatibility"),
        );
    }

    diags
}

/// Validate skill metadata against the Anthropic specification.
///
/// Expects raw `parse_frontmatter` output — the full `HashMap` before
/// known-key extraction. **Not** suitable for use on
/// `SkillProperties.metadata` (which has known keys already removed).
///
/// Returns a list of diagnostics (empty = valid).
#[must_use]
pub fn validate_metadata(metadata: &HashMap<String, Value>, dir: Option<&Path>) -> Vec<Diagnostic> {
    validate_metadata_with_target(metadata, dir, ValidationTarget::Standard)
}

/// Validate skill metadata with a specific validation target profile.
///
/// The `target` controls which fields are considered known:
/// - `Standard`: only Anthropic specification fields
/// - `ClaudeCode`: specification fields plus Claude Code extension fields
/// - `Permissive`: no unknown-field warnings
#[must_use]
pub fn validate_metadata_with_target(
    metadata: &HashMap<String, Value>,
    dir: Option<&Path>,
    target: ValidationTarget,
) -> Vec<Diagnostic> {
    let mut diags = Vec::new();

    // 1. Validate name.
    match metadata.get("name") {
        Some(Value::String(name)) => {
            diags.extend(validate_name(name, dir));
        }
        Some(_) => diags.push(
            Diagnostic::new(Severity::Error, E014, "`name` must be a string").with_field("name"),
        ),
        None => diags.push(
            Diagnostic::new(Severity::Error, E017, "missing required field `name`")
                .with_field("name"),
        ),
    }

    // 2. Validate description.
    match metadata.get("description") {
        Some(Value::String(desc)) => {
            diags.extend(validate_description(desc));
        }
        Some(_) => diags.push(
            Diagnostic::new(Severity::Error, E015, "`description` must be a string")
                .with_field("description"),
        ),
        None => diags.push(
            Diagnostic::new(
                Severity::Error,
                E018,
                "missing required field `description`",
            )
            .with_field("description"),
        ),
    }

    // 3. Validate compatibility if present.
    if let Some(val) = metadata.get("compatibility") {
        match val {
            Value::String(s) => diags.extend(validate_compatibility(s)),
            _ => diags.push(
                Diagnostic::new(Severity::Error, E016, "`compatibility` must be a string")
                    .with_field("compatibility"),
            ),
        }
    }

    // 4. Warn about unexpected metadata keys (sorted for deterministic output).
    if target != ValidationTarget::Permissive {
        let known = known_keys_for(target);
        let mut keys: Vec<_> = metadata.keys().collect();
        keys.sort();
        for key in keys {
            if !known.contains(&key.as_str()) {
                diags.push(
                    Diagnostic::new(
                        Severity::Warning,
                        W001,
                        format!("unexpected metadata field: '{key}'"),
                    )
                    .with_field("metadata"),
                );
            }
        }
    }

    diags
}

/// Validate a skill directory: find SKILL.md, parse, and check all rules.
///
/// Returns a list of diagnostics (empty = valid).
#[must_use]
pub fn validate(dir: &Path) -> Vec<Diagnostic> {
    validate_with_target(dir, ValidationTarget::Standard)
}

/// Validate a skill directory with a specific validation target profile.
///
/// Returns a list of diagnostics (empty = valid).
#[must_use]
pub fn validate_with_target(dir: &Path, target: ValidationTarget) -> Vec<Diagnostic> {
    // 1. Find SKILL.md.
    let path = match find_skill_md(dir) {
        Some(p) => p,
        None => return vec![Diagnostic::new(Severity::Error, E000, "SKILL.md not found")],
    };

    // 2. Read the file (with size check).
    let content = match read_file_checked(&path) {
        Ok(c) => c,
        Err(e) => return vec![Diagnostic::new(Severity::Error, E000, e.to_string())],
    };

    // 3. Parse frontmatter.
    let (metadata, body) = match parse_frontmatter(&content) {
        Ok(result) => result,
        Err(e) => return vec![Diagnostic::new(Severity::Error, E000, e.to_string())],
    };

    // 4. Validate metadata.
    let mut diags = validate_metadata_with_target(&metadata, Some(dir), target);

    // 5. Body-length warning.
    let line_count = body.lines().count();
    if line_count > 500 {
        diags.push(
            Diagnostic::new(
                Severity::Warning,
                W002,
                format!("body exceeds 500 lines ({line_count} lines)"),
            )
            .with_field("body"),
        );
    }

    diags
}

/// Maximum recursion depth for skill discovery.
const MAX_DISCOVERY_DEPTH: usize = 10;

/// Discover all skill directories under a root path.
///
/// Walks the directory tree recursively, finding all `SKILL.md` or
/// `skill.md` files. Returns the parent directory of each found file.
/// Skips hidden directories (names starting with `.`) and stops
/// recursing beyond [`MAX_DISCOVERY_DEPTH`] levels.
#[must_use]
pub fn discover_skills(root: &Path) -> Vec<std::path::PathBuf> {
    let mut dirs = Vec::new();
    discover_skills_recursive(root, &mut dirs, 0);
    dirs.sort();
    dirs
}

/// Recursive helper for `discover_skills`.
///
/// Stops recursing when `depth` exceeds [`MAX_DISCOVERY_DEPTH`].
fn discover_skills_recursive(dir: &Path, results: &mut Vec<std::path::PathBuf>, depth: usize) {
    if depth > MAX_DISCOVERY_DEPTH {
        return;
    }

    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    let mut has_skill_md = false;
    let mut subdirs = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            if is_regular_file(&path) && (name == "SKILL.md" || name == "skill.md") {
                has_skill_md = true;
            }
            if is_regular_dir(&path) && !name.starts_with('.') {
                subdirs.push(path);
            }
        }
    }

    if has_skill_md {
        results.push(dir.to_path_buf());
    }

    for subdir in subdirs {
        discover_skills_recursive(&subdir, results, depth + 1);
    }
}

/// Discover skill directories, collecting warnings for paths that could not be read.
///
/// Returns `(skill_paths, warnings)`. The original [`discover_skills()`] function
/// remains unchanged for backward compatibility.
#[must_use]
pub fn discover_skills_verbose(root: &Path) -> (Vec<std::path::PathBuf>, Vec<DiscoveryWarning>) {
    let mut skills = Vec::new();
    let mut warnings = Vec::new();
    discover_skills_recursive_verbose(root, &mut skills, &mut warnings, 0);
    skills.sort();
    (skills, warnings)
}

/// Recursive helper for `discover_skills_verbose`.
fn discover_skills_recursive_verbose(
    dir: &Path,
    results: &mut Vec<std::path::PathBuf>,
    warnings: &mut Vec<DiscoveryWarning>,
    depth: usize,
) {
    if depth > MAX_DISCOVERY_DEPTH {
        return;
    }
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) => {
            warnings.push(DiscoveryWarning {
                path: dir.to_path_buf(),
                message: format!("cannot read directory: {e}"),
            });
            return;
        }
    };

    let mut has_skill_md = false;
    let mut subdirs = Vec::new();

    for entry_result in entries {
        match entry_result {
            Ok(entry) => {
                let path = entry.path();
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if is_regular_file(&path) && (name == "SKILL.md" || name == "skill.md") {
                        has_skill_md = true;
                    }
                    if is_regular_dir(&path) && !name.starts_with('.') {
                        subdirs.push(path);
                    }
                }
            }
            Err(e) => {
                warnings.push(DiscoveryWarning {
                    path: dir.to_path_buf(),
                    message: format!("cannot read directory entry: {e}"),
                });
            }
        }
    }

    if has_skill_md {
        results.push(dir.to_path_buf());
    }

    for subdir in subdirs {
        discover_skills_recursive_verbose(&subdir, results, warnings, depth + 1);
    }
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
        let diags = validate_metadata(&meta, None);
        assert!(diags.is_empty(), "expected no diagnostics, got: {diags:?}");
    }

    #[test]
    fn missing_name() {
        let meta = make_metadata(&[("description", "desc")]);
        let diags = validate_metadata(&meta, None);
        assert!(diags.iter().any(|d| d.message.contains("name")));
    }

    #[test]
    fn missing_description() {
        let meta = make_metadata(&[("name", "test")]);
        let diags = validate_metadata(&meta, None);
        assert!(diags.iter().any(|d| d.message.contains("description")));
    }

    #[test]
    fn empty_name() {
        let meta = make_metadata(&[("name", ""), ("description", "desc")]);
        let diags = validate_metadata(&meta, None);
        assert!(diags
            .iter()
            .any(|d| d.message.contains("name must not be empty")));
    }

    #[test]
    fn empty_description() {
        let meta = make_metadata(&[("name", "test"), ("description", "")]);
        let diags = validate_metadata(&meta, None);
        assert!(diags
            .iter()
            .any(|d| d.message.contains("description must not be empty")));
    }

    #[test]
    fn name_too_long() {
        let long_name: String = "a".repeat(65);
        let meta = make_metadata(&[("name", &long_name), ("description", "desc")]);
        let diags = validate_metadata(&meta, None);
        assert!(diags
            .iter()
            .any(|d| d.message.contains("exceeds 64 characters")));
    }

    #[test]
    fn name_exactly_64_chars() {
        let name: String = "a".repeat(64);
        let meta = make_metadata(&[("name", &name), ("description", "desc")]);
        let diags = validate_metadata(&meta, None);
        let errors: Vec<_> = diags.iter().filter(|d| d.is_error()).collect();
        assert!(errors.is_empty(), "expected no errors, got: {errors:?}");
    }

    #[test]
    fn name_with_uppercase() {
        let meta = make_metadata(&[("name", "MySkill"), ("description", "desc")]);
        let diags = validate_metadata(&meta, None);
        assert!(diags.iter().any(|d| d.message.contains("uppercase")));
        // Should be a single E003 diagnostic, not one per character.
        let e003_count = diags.iter().filter(|d| d.code == E003).count();
        assert_eq!(e003_count, 1, "expected single E003, got {e003_count}");
    }

    #[test]
    fn name_with_leading_hyphen() {
        let meta = make_metadata(&[("name", "-my-skill"), ("description", "desc")]);
        let diags = validate_metadata(&meta, None);
        assert!(diags
            .iter()
            .any(|d| d.message.contains("must not start with a hyphen")));
    }

    #[test]
    fn name_with_trailing_hyphen() {
        let meta = make_metadata(&[("name", "my-skill-"), ("description", "desc")]);
        let diags = validate_metadata(&meta, None);
        assert!(diags
            .iter()
            .any(|d| d.message.contains("must not end with a hyphen")));
    }

    #[test]
    fn name_with_consecutive_hyphens() {
        let meta = make_metadata(&[("name", "my--skill"), ("description", "desc")]);
        let diags = validate_metadata(&meta, None);
        assert!(diags
            .iter()
            .any(|d| d.message.contains("consecutive hyphens")));
    }

    #[test]
    fn name_with_invalid_characters() {
        let meta = make_metadata(&[("name", "my_skill!"), ("description", "desc")]);
        let diags = validate_metadata(&meta, None);
        assert!(diags
            .iter()
            .any(|d| d.message.contains("invalid character")));
    }

    #[test]
    fn name_contains_reserved_anthropic() {
        let meta = make_metadata(&[("name", "my-anthropic-skill"), ("description", "desc")]);
        let diags = validate_metadata(&meta, None);
        assert!(diags
            .iter()
            .any(|d| d.message.contains("reserved word: 'anthropic'")));
    }

    #[test]
    fn name_contains_reserved_claude() {
        let meta = make_metadata(&[("name", "claude-helper"), ("description", "desc")]);
        let diags = validate_metadata(&meta, None);
        assert!(diags
            .iter()
            .any(|d| d.message.contains("reserved word: 'claude'")));
    }

    #[test]
    fn name_does_not_match_directory() {
        let (_parent, dir) = make_skill_dir(
            "other-name",
            "---\nname: my-skill\ndescription: desc\n---\n",
        );
        let meta = make_metadata(&[("name", "my-skill"), ("description", "desc")]);
        let diags = validate_metadata(&meta, Some(&dir));
        assert!(diags
            .iter()
            .any(|d| d.message.contains("does not match directory")));
    }

    #[test]
    fn name_matches_directory() {
        let (_parent, dir) =
            make_skill_dir("my-skill", "---\nname: my-skill\ndescription: desc\n---\n");
        let meta = make_metadata(&[("name", "my-skill"), ("description", "desc")]);
        let diags = validate_metadata(&meta, Some(&dir));
        let errors: Vec<_> = diags.iter().filter(|d| d.is_error()).collect();
        assert!(errors.is_empty(), "expected no errors, got: {errors:?}");
    }

    #[test]
    fn description_too_long() {
        let long_desc: String = "a".repeat(1025);
        let meta = make_metadata(&[("name", "test"), ("description", &long_desc)]);
        let diags = validate_metadata(&meta, None);
        assert!(diags
            .iter()
            .any(|d| d.message.contains("description exceeds 1024")));
    }

    #[test]
    fn description_exactly_1024_chars() {
        let desc: String = "a".repeat(1024);
        let meta = make_metadata(&[("name", "test"), ("description", &desc)]);
        let diags = validate_metadata(&meta, None);
        let errors: Vec<_> = diags.iter().filter(|d| d.is_error()).collect();
        assert!(errors.is_empty(), "expected no errors, got: {errors:?}");
    }

    #[test]
    fn description_with_xml_tags() {
        let meta = make_metadata(&[
            ("name", "test"),
            ("description", "A <script>alert</script> skill"),
        ]);
        let diags = validate_metadata(&meta, None);
        assert!(diags
            .iter()
            .any(|d| d.message.contains("description contains XML/HTML")));
    }

    #[test]
    fn name_with_xml_characters_rejected() {
        let meta = make_metadata(&[("name", "<img/>skill"), ("description", "desc")]);
        let diags = validate_metadata(&meta, None);
        assert!(
            diags
                .iter()
                .any(|d| d.message.contains("invalid character")),
            "expected invalid character error, got: {diags:?}"
        );
    }

    #[test]
    fn compatibility_too_long() {
        let long_compat: String = "a".repeat(501);
        let meta = make_metadata(&[
            ("name", "test"),
            ("description", "desc"),
            ("compatibility", &long_compat),
        ]);
        let diags = validate_metadata(&meta, None);
        assert!(diags
            .iter()
            .any(|d| d.message.contains("compatibility exceeds 500")));
    }

    #[test]
    fn compatibility_exactly_500_chars() {
        let compat: String = "a".repeat(500);
        let meta = make_metadata(&[
            ("name", "test"),
            ("description", "desc"),
            ("compatibility", &compat),
        ]);
        let diags = validate_metadata(&meta, None);
        let errors: Vec<_> = diags.iter().filter(|d| d.is_error()).collect();
        assert!(errors.is_empty(), "expected no errors, got: {errors:?}");
    }

    #[test]
    fn unexpected_metadata_field_warning() {
        let meta = make_metadata(&[
            ("name", "test"),
            ("description", "desc"),
            ("custom-field", "value"),
        ]);
        let diags = validate_metadata(&meta, None);
        let warnings: Vec<_> = diags.iter().filter(|d| d.is_warning()).collect();
        assert!(
            warnings.iter().any(|w| w.message.contains("custom-field")),
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
        let diags = validate_metadata(&meta, None);
        assert!(diags.is_empty(), "expected no messages, got: {diags:?}");
    }

    // ── i18n / Unicode tests ─────────────────────────────────────────

    #[test]
    fn chinese_characters_accepted() {
        let meta = make_metadata(&[("name", "技能工具"), ("description", "desc")]);
        let diags = validate_metadata(&meta, None);
        let errors: Vec<_> = diags.iter().filter(|d| d.is_error()).collect();
        assert!(errors.is_empty(), "expected no errors, got: {errors:?}");
    }

    #[test]
    fn russian_lowercase_with_hyphens_accepted() {
        let meta = make_metadata(&[("name", "навык-тест"), ("description", "desc")]);
        let diags = validate_metadata(&meta, None);
        let errors: Vec<_> = diags.iter().filter(|d| d.is_error()).collect();
        assert!(errors.is_empty(), "expected no errors, got: {errors:?}");
    }

    #[test]
    fn uppercase_cyrillic_rejected() {
        let meta = make_metadata(&[("name", "Навык"), ("description", "desc")]);
        let diags = validate_metadata(&meta, None);
        assert!(
            diags.iter().any(|d| d.code == E003),
            "expected E003 for uppercase Cyrillic, got: {diags:?}"
        );
    }

    #[test]
    fn nfkc_normalization_applied() {
        let meta = make_metadata(&[("name", "s\u{FB01}le"), ("description", "desc")]);
        let diags = validate_metadata(&meta, None);
        let errors: Vec<_> = diags.iter().filter(|d| d.is_error()).collect();
        assert!(
            errors.is_empty(),
            "NFKC-normalized 'sﬁle' → 'sfile' should be valid, got: {errors:?}"
        );
    }

    // ── validate (full pipeline) tests ───────────────────────────────

    #[test]
    fn validate_valid_skill_directory() {
        let (_parent, dir) = make_skill_dir(
            "my-skill",
            "---\nname: my-skill\ndescription: A valid skill\n---\n# Body\n",
        );
        let diags = validate(&dir);
        assert!(diags.is_empty(), "expected no diagnostics, got: {diags:?}");
    }

    #[test]
    fn validate_nonexistent_path() {
        let dir = std::path::Path::new("/nonexistent/path/that/does/not/exist");
        let diags = validate(dir);
        assert!(!diags.is_empty());
    }

    #[test]
    fn validate_missing_skill_md() {
        let dir = tempdir().unwrap();
        let diags = validate(dir.path());
        assert!(diags
            .iter()
            .any(|d| d.message.contains("SKILL.md not found")));
    }

    #[test]
    fn validate_body_over_500_lines_warning() {
        let body: String = (0..501)
            .map(|i| format!("line {i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let content = format!("---\nname: my-skill\ndescription: desc\n---\n{body}\n");
        let (_parent, dir) = make_skill_dir("my-skill", &content);
        let diags = validate(&dir);
        let warnings: Vec<_> = diags.iter().filter(|d| d.is_warning()).collect();
        assert!(
            warnings
                .iter()
                .any(|w| w.message.contains("body exceeds 500 lines")),
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
        let diags = validate(&dir);
        let warnings: Vec<_> = diags.iter().filter(|d| d.is_warning()).collect();
        assert!(
            warnings.is_empty(),
            "expected no warnings, got: {warnings:?}"
        );
    }

    #[test]
    fn validate_multiple_errors_collected() {
        let meta = make_metadata(&[("name", ""), ("description", "")]);
        let diags = validate_metadata(&meta, None);
        let errors: Vec<_> = diags.iter().filter(|d| d.is_error()).collect();
        assert!(
            errors.len() >= 2,
            "expected at least 2 errors, got: {errors:?}"
        );
    }

    // ── Reserved word edge case tests ────────────────────────────────

    #[test]
    fn reserved_word_as_substring_accepted() {
        let meta = make_metadata(&[("name", "claudette"), ("description", "desc")]);
        let diags = validate_metadata(&meta, None);
        let reserved_errors: Vec<_> = diags
            .iter()
            .filter(|d| d.message.contains("reserved word"))
            .collect();
        assert!(
            reserved_errors.is_empty(),
            "segment-based matching should accept 'claudette', got: {reserved_errors:?}"
        );
    }

    #[test]
    fn reserved_word_as_exact_segment_rejected() {
        let meta = make_metadata(&[("name", "my-claude-tool"), ("description", "desc")]);
        let diags = validate_metadata(&meta, None);
        assert!(
            diags
                .iter()
                .any(|d| d.message.contains("reserved word: 'claude'")),
            "expected reserved word error, got: {diags:?}"
        );
    }

    // ── Diagnostic structure tests ───────────────────────────────────

    #[test]
    fn diagnostics_have_error_codes() {
        let meta = make_metadata(&[("name", ""), ("description", "desc")]);
        let diags = validate_metadata(&meta, None);
        assert!(
            diags.iter().all(|d| !d.code.is_empty()),
            "all diagnostics should have error codes"
        );
    }

    #[test]
    fn diagnostics_have_fields() {
        let meta = make_metadata(&[("name", ""), ("description", "")]);
        let diags = validate_metadata(&meta, None);
        let errors: Vec<_> = diags.iter().filter(|d| d.is_error()).collect();
        assert!(
            errors.iter().all(|d| d.field.is_some()),
            "all error diagnostics should have field set"
        );
    }

    #[test]
    fn error_display_matches_original_format() {
        let meta = make_metadata(&[("name", ""), ("description", "desc")]);
        let diags = validate_metadata(&meta, None);
        let name_error = diags
            .iter()
            .find(|d| d.code == E001)
            .expect("should have E001");
        assert_eq!(name_error.to_string(), "name must not be empty");
    }

    #[test]
    fn warning_display_matches_original_format() {
        let meta = make_metadata(&[
            ("name", "test"),
            ("description", "desc"),
            ("custom-field", "value"),
        ]);
        let diags = validate_metadata(&meta, None);
        let warning = diags
            .iter()
            .find(|d| d.code == W001)
            .expect("should have W001");
        assert_eq!(
            warning.to_string(),
            "warning: unexpected metadata field: 'custom-field'"
        );
    }

    // ── ValidationTarget tests ───────────────────────────────────────

    #[test]
    fn claude_code_field_no_warning_with_claude_code_target() {
        let meta = make_metadata(&[
            ("name", "test"),
            ("description", "desc"),
            ("argument-hint", "[file]"),
        ]);
        let diags = validate_metadata_with_target(&meta, None, ValidationTarget::ClaudeCode);
        let warnings: Vec<_> = diags.iter().filter(|d| d.is_warning()).collect();
        assert!(
            warnings.is_empty(),
            "argument-hint should not warn with claude-code target, got: {warnings:?}"
        );
    }

    #[test]
    fn claude_code_field_warns_with_standard_target() {
        let meta = make_metadata(&[
            ("name", "test"),
            ("description", "desc"),
            ("argument-hint", "[file]"),
        ]);
        let diags = validate_metadata_with_target(&meta, None, ValidationTarget::Standard);
        let warnings: Vec<_> = diags.iter().filter(|d| d.is_warning()).collect();
        assert!(
            warnings.iter().any(|w| w.message.contains("argument-hint")),
            "argument-hint should warn with standard target, got: {warnings:?}"
        );
    }

    #[test]
    fn permissive_target_no_unknown_field_warnings() {
        let meta = make_metadata(&[
            ("name", "test"),
            ("description", "desc"),
            ("totally-custom", "value"),
            ("another-custom", "value"),
        ]);
        let diags = validate_metadata_with_target(&meta, None, ValidationTarget::Permissive);
        let warnings: Vec<_> = diags.iter().filter(|d| d.is_warning()).collect();
        assert!(
            warnings.is_empty(),
            "permissive target should have no warnings, got: {warnings:?}"
        );
    }

    #[test]
    fn unknown_field_warns_even_with_claude_code_target() {
        let meta = make_metadata(&[
            ("name", "test"),
            ("description", "desc"),
            ("truly-unknown-field", "value"),
        ]);
        let diags = validate_metadata_with_target(&meta, None, ValidationTarget::ClaudeCode);
        let warnings: Vec<_> = diags.iter().filter(|d| d.is_warning()).collect();
        assert!(
            warnings
                .iter()
                .any(|w| w.message.contains("truly-unknown-field")),
            "truly unknown field should warn even with claude-code target, got: {warnings:?}"
        );
    }

    #[test]
    fn validate_backward_compat_matches_standard() {
        let (_parent, dir) =
            make_skill_dir("my-skill", "---\nname: my-skill\ndescription: desc\n---\n");
        let standard = validate_with_target(&dir, ValidationTarget::Standard);
        let default = validate(&dir);
        assert_eq!(standard.len(), default.len());
    }

    // ── discover_skills tests ────────────────────────────────────────

    #[test]
    fn discover_skills_finds_skill_md() {
        let parent = tempdir().unwrap();
        let skill_dir = parent.path().join("my-skill");
        fs::create_dir(&skill_dir).unwrap();
        fs::write(skill_dir.join("SKILL.md"), "---\nname: test\n---\n").unwrap();
        let dirs = discover_skills(parent.path());
        assert_eq!(dirs.len(), 1);
        assert_eq!(dirs[0], skill_dir);
    }

    #[test]
    fn discover_skills_finds_nested() {
        let parent = tempdir().unwrap();
        let nested = parent.path().join("skills").join("my-skill");
        fs::create_dir_all(&nested).unwrap();
        fs::write(nested.join("SKILL.md"), "---\nname: test\n---\n").unwrap();
        let dirs = discover_skills(parent.path());
        assert_eq!(dirs.len(), 1);
        assert_eq!(dirs[0], nested);
    }

    #[test]
    fn discover_skills_skips_hidden_dirs() {
        let parent = tempdir().unwrap();
        let hidden = parent.path().join(".hidden");
        fs::create_dir(&hidden).unwrap();
        fs::write(hidden.join("SKILL.md"), "---\nname: test\n---\n").unwrap();
        let dirs = discover_skills(parent.path());
        assert!(dirs.is_empty(), "should skip hidden directories");
    }

    #[test]
    fn discover_skills_empty_dir() {
        let parent = tempdir().unwrap();
        let dirs = discover_skills(parent.path());
        assert!(dirs.is_empty());
    }

    #[test]
    fn discover_skills_multiple() {
        let parent = tempdir().unwrap();
        let skill_a = parent.path().join("skill-a");
        let skill_b = parent.path().join("skill-b");
        fs::create_dir(&skill_a).unwrap();
        fs::create_dir(&skill_b).unwrap();
        fs::write(skill_a.join("SKILL.md"), "---\nname: a\n---\n").unwrap();
        fs::write(skill_b.join("SKILL.md"), "---\nname: b\n---\n").unwrap();
        let dirs = discover_skills(parent.path());
        assert_eq!(dirs.len(), 2);
    }

    // ── discover_skills_verbose tests ─────────────────────────────────

    #[test]
    fn discover_skills_verbose_valid_directory() {
        let parent = tempdir().unwrap();
        let skill_dir = parent.path().join("my-skill");
        fs::create_dir(&skill_dir).unwrap();
        fs::write(skill_dir.join("SKILL.md"), "---\nname: test\n---\n").unwrap();
        let (dirs, warnings) = discover_skills_verbose(parent.path());
        assert_eq!(dirs.len(), 1);
        assert_eq!(dirs[0], skill_dir);
        assert!(
            warnings.is_empty(),
            "expected no warnings, got: {warnings:?}"
        );
    }

    #[test]
    fn discover_skills_verbose_unreadable_root() {
        let nonexistent = std::path::Path::new("/nonexistent/path/that/does/not/exist");
        let (dirs, warnings) = discover_skills_verbose(nonexistent);
        assert!(dirs.is_empty());
        assert_eq!(warnings.len(), 1);
        assert!(
            warnings[0].message.contains("cannot read directory"),
            "expected read error, got: {}",
            warnings[0].message
        );
        assert_eq!(warnings[0].path, nonexistent);
    }

    #[test]
    fn discover_skills_verbose_multiple_no_warnings() {
        let parent = tempdir().unwrap();
        let skill_a = parent.path().join("skill-a");
        let skill_b = parent.path().join("skill-b");
        fs::create_dir(&skill_a).unwrap();
        fs::create_dir(&skill_b).unwrap();
        fs::write(skill_a.join("SKILL.md"), "---\nname: a\n---\n").unwrap();
        fs::write(skill_b.join("SKILL.md"), "---\nname: b\n---\n").unwrap();
        let (dirs, warnings) = discover_skills_verbose(parent.path());
        assert_eq!(dirs.len(), 2);
        assert!(
            warnings.is_empty(),
            "expected no warnings, got: {warnings:?}"
        );
    }

    #[test]
    fn discover_skills_backward_compat() {
        let parent = tempdir().unwrap();
        let skill_dir = parent.path().join("my-skill");
        fs::create_dir(&skill_dir).unwrap();
        fs::write(skill_dir.join("SKILL.md"), "---\nname: test\n---\n").unwrap();
        let original = discover_skills(parent.path());
        let (verbose, _) = discover_skills_verbose(parent.path());
        assert_eq!(original, verbose, "verbose variant should match original");
    }

    // ── discover_skills depth tests ──────────────────────────────────

    #[test]
    fn discover_skills_normal_depth() {
        let parent = tempdir().unwrap();
        // Place a skill at depth 5 (well under the limit).
        let mut current = parent.path().to_path_buf();
        for i in 0..5 {
            current = current.join(format!("level-{i}"));
            fs::create_dir(&current).unwrap();
        }
        fs::write(current.join("SKILL.md"), "---\nname: deep\n---\n").unwrap();
        let dirs = discover_skills(parent.path());
        assert_eq!(dirs.len(), 1);
        assert_eq!(dirs[0], current);
    }

    #[test]
    fn discover_skills_stops_at_max_depth() {
        let parent = tempdir().unwrap();
        // Create nesting deeper than MAX_DISCOVERY_DEPTH and place a skill
        // beyond the limit. Discovery should not crash and should not find it.
        let mut current = parent.path().to_path_buf();
        for i in 0..15 {
            current = current.join(format!("level-{i}"));
            fs::create_dir(&current).unwrap();
        }
        fs::write(current.join("SKILL.md"), "---\nname: too-deep\n---\n").unwrap();
        let dirs = discover_skills(parent.path());
        // The skill is at depth 15, beyond MAX_DISCOVERY_DEPTH (10),
        // so it should not be discovered.
        assert!(
            dirs.is_empty(),
            "skill beyond max depth should not be found, got: {dirs:?}"
        );
    }
}
