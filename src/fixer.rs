//! Auto-fix application for fixable diagnostics.
//!
//! Reads a SKILL.md file, applies fixes for diagnostics that have
//! suggestions, and writes the result back. Currently supports fixing
//! frontmatter fields only (name and description).

use std::path::Path;
use std::sync::LazyLock;

use regex::Regex;

use crate::diagnostics::{Diagnostic, E002, E003, E006, E012};
use crate::errors::Result;
use crate::parser::find_skill_md;

/// Regex for matching the `name` field line in frontmatter.
static NAME_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?m)^name:\s*(.*)$").expect("name regex must compile"));

/// Regex for matching the `description` field line in frontmatter.
static DESCRIPTION_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?m)^description:\s*(.*)$").expect("description regex must compile")
});

/// Regex for matching XML/HTML tags.
static TAG_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"<[a-zA-Z/][^>]*>").expect("tag regex must compile"));

/// Apply automatic fixes to a SKILL.md file based on diagnostics.
///
/// Only fixes diagnostics that have a suggestion and are in the fixable
/// set (E002, E003, E006, E012). Returns the number of fixes applied.
///
/// # Errors
///
/// Returns `AigentError::Io` if the file cannot be read or written.
/// Returns `AigentError::Parse` if the file cannot be parsed.
pub fn apply_fixes(dir: &Path, diagnostics: &[Diagnostic]) -> Result<usize> {
    let path = find_skill_md(dir).ok_or_else(|| crate::errors::AigentError::Parse {
        message: "SKILL.md not found".to_string(),
    })?;

    let content = std::fs::read_to_string(&path)?;
    let mut modified = content.clone();
    let mut fix_count = 0;

    for diag in diagnostics {
        if diag.suggestion.is_none() {
            continue;
        }
        let before = modified.clone();
        match diag.code {
            E002 => {
                // Truncate name at hyphen boundary.
                if let Some(truncated) = extract_quoted_value(&diag.suggestion) {
                    modified = fix_frontmatter_field(&modified, "name", &truncated);
                }
            }
            E003 => {
                // Lowercase the entire name.
                if diag
                    .suggestion
                    .as_deref()
                    .is_some_and(|s| s.starts_with("Use lowercase:"))
                {
                    modified = lowercase_name_in_frontmatter(&modified);
                }
            }
            E006 => {
                // Collapse consecutive hyphens.
                if let Some(collapsed) = extract_quoted_value(&diag.suggestion) {
                    modified = fix_frontmatter_field(&modified, "name", &collapsed);
                }
            }
            E012 => {
                // Strip XML tags from description.
                modified = strip_xml_from_description(&modified);
            }
            _ => {}
        }
        if modified != before {
            fix_count += 1;
        }
    }

    if fix_count > 0 && modified != content {
        std::fs::write(&path, &modified)?;
    }

    Ok(fix_count)
}

/// Extract a single-quoted value from a suggestion string.
///
/// e.g., `"Truncate to: 'my-skill'"` → `Some("my-skill")`
fn extract_quoted_value(suggestion: &Option<String>) -> Option<String> {
    let s = suggestion.as_deref()?;
    let start = s.find('\'')?;
    let end = s.rfind('\'')?;
    if start < end {
        Some(s[start + 1..end].to_string())
    } else {
        None
    }
}

/// Replace a frontmatter field value in SKILL.md content.
fn fix_frontmatter_field(content: &str, field: &str, new_value: &str) -> String {
    let re = match field {
        "name" => &*NAME_RE,
        "description" => &*DESCRIPTION_RE,
        _ => return content.to_string(),
    };
    re.replace(content, format!("{field}: {new_value}"))
        .to_string()
}

/// Lowercase the `name` field value in frontmatter.
fn lowercase_name_in_frontmatter(content: &str) -> String {
    NAME_RE
        .replace(content, |caps: &regex::Captures| {
            format!("name: {}", caps[1].to_lowercase())
        })
        .to_string()
}

/// Strip XML/HTML tags from the `description` field in frontmatter.
fn strip_xml_from_description(content: &str) -> String {
    DESCRIPTION_RE
        .replace(content, |caps: &regex::Captures| {
            let cleaned = TAG_RE.replace_all(&caps[1], "").to_string();
            format!("description: {cleaned}")
        })
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostics::Severity;
    use std::fs;
    use tempfile::tempdir;

    /// Create a skill dir with content, return (TempDir, PathBuf).
    fn make_skill_dir(name: &str, content: &str) -> (tempfile::TempDir, std::path::PathBuf) {
        let parent = tempdir().unwrap();
        let dir = parent.path().join(name);
        fs::create_dir(&dir).unwrap();
        fs::write(dir.join("SKILL.md"), content).unwrap();
        (parent, dir)
    }

    #[test]
    fn extract_quoted_value_basic() {
        let s = Some("Truncate to: 'my-skill'".to_string());
        assert_eq!(extract_quoted_value(&s), Some("my-skill".to_string()));
    }

    #[test]
    fn extract_quoted_value_none() {
        assert_eq!(extract_quoted_value(&None), None);
    }

    #[test]
    fn extract_quoted_value_no_quotes() {
        let s = Some("No quotes here".to_string());
        assert_eq!(extract_quoted_value(&s), None);
    }

    #[test]
    fn fix_frontmatter_field_replaces_name() {
        let content = "---\nname: old-name\ndescription: desc\n---\n";
        let result = fix_frontmatter_field(content, "name", "new-name");
        assert!(result.contains("name: new-name"));
        assert!(!result.contains("old-name"));
    }

    #[test]
    fn lowercase_name_in_frontmatter_lowercases() {
        let content = "---\nname: MySkill\ndescription: desc\n---\n";
        let result = lowercase_name_in_frontmatter(content);
        assert!(result.contains("name: myskill"));
    }

    #[test]
    fn strip_xml_from_description_removes_tags() {
        let content = "---\nname: test\ndescription: A <script>alert</script> skill\n---\n";
        let result = strip_xml_from_description(content);
        assert!(result.contains("description: A alert skill"));
        assert!(!result.contains("<script>"));
    }

    #[test]
    fn apply_fixes_e003_uppercase() {
        let (_parent, dir) = make_skill_dir(
            "myskill",
            "---\nname: MySkill\ndescription: A valid skill\n---\n",
        );
        let diags =
            vec![
                Diagnostic::new(Severity::Error, E003, "name contains uppercase characters")
                    .with_field("name")
                    .with_suggestion("Use lowercase: 'myskill'"),
            ];

        let count = apply_fixes(&dir, &diags).unwrap();
        assert_eq!(count, 1);

        let content = fs::read_to_string(dir.join("SKILL.md")).unwrap();
        assert!(
            content.contains("name: myskill"),
            "name should be lowercased: {content}"
        );
    }

    #[test]
    fn apply_fixes_e002_truncate_name() {
        // Create a name longer than 64 characters with hyphens for truncation.
        let long_name = format!("a-{}-z", "b".repeat(62));
        let content = format!("---\nname: {long_name}\ndescription: A valid skill\n---\n");
        let (_parent, dir) = make_skill_dir("truncated", &content);
        let diags = vec![
            Diagnostic::new(Severity::Error, E002, "name exceeds 64 characters")
                .with_field("name")
                .with_suggestion("Truncate to: 'a'"),
        ];

        let count = apply_fixes(&dir, &diags).unwrap();
        assert_eq!(count, 1);

        let result = fs::read_to_string(dir.join("SKILL.md")).unwrap();
        assert!(
            result.contains("name: a"),
            "name should be truncated: {result}"
        );
    }

    #[test]
    fn apply_fixes_e006_consecutive_hyphens() {
        let (_parent, dir) =
            make_skill_dir("my-skill", "---\nname: my--skill\ndescription: desc\n---\n");
        let diags =
            vec![
                Diagnostic::new(Severity::Error, E006, "name contains consecutive hyphens")
                    .with_field("name")
                    .with_suggestion("Remove consecutive hyphens: 'my-skill'"),
            ];

        let count = apply_fixes(&dir, &diags).unwrap();
        assert_eq!(count, 1);

        let content = fs::read_to_string(dir.join("SKILL.md")).unwrap();
        assert!(
            content.contains("name: my-skill"),
            "hyphens should be collapsed: {content}"
        );
    }

    #[test]
    fn apply_fixes_e012_xml_tags() {
        let (_parent, dir) = make_skill_dir(
            "test",
            "---\nname: test\ndescription: A <b>bold</b> skill\n---\n",
        );
        let diags =
            vec![
                Diagnostic::new(Severity::Error, E012, "description contains XML/HTML tags")
                    .with_field("description")
                    .with_suggestion("Remove XML tags from description"),
            ];

        let count = apply_fixes(&dir, &diags).unwrap();
        assert_eq!(count, 1);

        let content = fs::read_to_string(dir.join("SKILL.md")).unwrap();
        assert!(
            content.contains("description: A bold skill"),
            "XML tags should be removed: {content}"
        );
    }

    #[test]
    fn apply_fixes_no_fixable_diagnostics() {
        let (_parent, dir) = make_skill_dir("test", "---\nname: test\ndescription: desc\n---\n");
        let diags = vec![
            Diagnostic::new(Severity::Error, E002, "name exceeds 64 characters").with_field("name"),
        ];
        // No suggestion → not fixable.
        let count = apply_fixes(&dir, &diags).unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn apply_fixes_returns_zero_for_empty_diags() {
        let (_parent, dir) = make_skill_dir("test", "---\nname: test\ndescription: desc\n---\n");
        let count = apply_fixes(&dir, &[]).unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn apply_fixes_missing_skill_md_returns_error() {
        let parent = tempdir().unwrap();
        let dir = parent.path().join("no-skill");
        fs::create_dir(&dir).unwrap();
        let result = apply_fixes(&dir, &[]);
        assert!(result.is_err(), "should fail if SKILL.md not found");
    }

    #[test]
    fn apply_fixes_duplicate_e003_counts_one() {
        // Defensive test: if two E003 diagnostics are provided for the same
        // field, the fixer should only count one actual change.
        let (_parent, dir) = make_skill_dir(
            "myskill",
            "---\nname: MySkill\ndescription: A valid skill\n---\n",
        );
        let diags = vec![
            Diagnostic::new(Severity::Error, E003, "name contains uppercase characters")
                .with_field("name")
                .with_suggestion("Use lowercase: 'myskill'"),
            Diagnostic::new(Severity::Error, E003, "name contains uppercase characters")
                .with_field("name")
                .with_suggestion("Use lowercase: 'myskill'"),
        ];

        let count = apply_fixes(&dir, &diags).unwrap();
        assert_eq!(count, 1, "only one actual change should be counted");

        let content = fs::read_to_string(dir.join("SKILL.md")).unwrap();
        assert!(
            content.contains("name: myskill"),
            "name should be lowercased: {content}"
        );
    }
}
