//! SKILL.md formatting: canonical key ordering, consistent quoting, and markdown cleanup.
//!
//! The formatter normalizes SKILL.md files without changing their semantic content.
//! It is idempotent — running it twice produces no further changes.

use std::path::Path;

use crate::errors::{AigentError, Result};
use crate::parser::find_skill_md;

/// Result of formatting a single skill.
#[derive(Debug)]
pub struct FormatResult {
    /// Whether the formatted content differs from the original.
    pub changed: bool,
    /// The formatted content (full SKILL.md file).
    pub content: String,
}

/// Canonical key ordering for YAML frontmatter.
///
/// Keys are emitted in this order. Keys not in this list are appended
/// alphabetically after the known keys.
const KEY_ORDER: &[&str] = &[
    "name",
    "description",
    "instructions",
    "compatibility",
    "context",
    "allowed-tools",
    "license",
    "metadata",
];

/// Format a SKILL.md file in place.
///
/// Returns a [`FormatResult`] indicating whether the file was changed
/// and containing the formatted content. The caller decides whether to
/// write the result back to disk.
///
/// # Errors
///
/// Returns an error if the SKILL.md file cannot be found or read,
/// or if the frontmatter is malformed (no `---` delimiters).
pub fn format_skill(dir: &Path) -> Result<FormatResult> {
    let path = find_skill_md(dir).ok_or_else(|| AigentError::Parse {
        message: "no SKILL.md found".into(),
    })?;
    let original = std::fs::read_to_string(&path)?;

    let content = format_content(&original)?;
    let changed = content != original;

    Ok(FormatResult { changed, content })
}

/// Format SKILL.md content (for testing without filesystem access).
///
/// # Errors
///
/// Returns an error if the content lacks valid `---` frontmatter delimiters.
pub fn format_content(original: &str) -> Result<String> {
    // Normalize CRLF to LF so byte-offset arithmetic works correctly.
    let content = original.replace("\r\n", "\n");

    // Split into frontmatter and body.
    if !content.starts_with("---") {
        return Err(AigentError::Parse {
            message: "SKILL.md must start with --- delimiter".into(),
        });
    }

    let after_first = &content[3..];
    let close_pos = after_first.find("\n---\n").or_else(|| {
        // Handle case where closing --- is at end of file.
        if after_first.ends_with("\n---") {
            Some(after_first.len() - 4)
        } else {
            None
        }
    });

    let close_pos = close_pos.ok_or_else(|| AigentError::Parse {
        message: "missing closing --- delimiter".into(),
    })?;

    // Skip the \n after opening ---; handle empty frontmatter (close_pos == 0).
    let yaml_str = if close_pos > 0 {
        &after_first[1..close_pos]
    } else {
        ""
    };
    let body_start = close_pos + 5; // skip \n---\n
    let body = if body_start <= after_first.len() {
        &after_first[body_start..]
    } else {
        ""
    };

    let formatted_yaml = format_frontmatter(yaml_str);
    let formatted_body = format_body(body);

    Ok(format!("---\n{formatted_yaml}\n---\n{formatted_body}"))
}

/// Format YAML frontmatter with canonical key ordering.
///
/// Preserves values exactly as-is (including multiline blocks, quoting,
/// and comments). Only reorders top-level keys.
fn format_frontmatter(yaml: &str) -> String {
    let blocks = parse_yaml_blocks(yaml);

    // Separate into known-order keys, unknown keys, and comments.
    let mut ordered: Vec<(usize, &YamlBlock)> = Vec::new();
    let mut unknown: Vec<&YamlBlock> = Vec::new();
    let mut header_comments: Vec<&YamlBlock> = Vec::new();

    for block in &blocks {
        match block {
            YamlBlock::Comment(_) => {
                // Collect comments that appear before any key.
                if ordered.is_empty() && unknown.is_empty() {
                    header_comments.push(block);
                }
                // Comments between keys are attached to the following key
                // in the original order — we preserve them by keeping them
                // in the unknown list.
            }
            YamlBlock::Key { name, .. } => {
                if let Some(pos) = KEY_ORDER.iter().position(|k| k == name) {
                    ordered.push((pos, block));
                } else {
                    unknown.push(block);
                }
            }
        }
    }

    // Sort known keys by canonical position.
    ordered.sort_by_key(|(pos, _)| *pos);

    // Sort unknown keys alphabetically.
    unknown.sort_by(|a, b| {
        let name_a = match a {
            YamlBlock::Key { name, .. } => name.as_str(),
            YamlBlock::Comment(_) => "",
        };
        let name_b = match b {
            YamlBlock::Key { name, .. } => name.as_str(),
            YamlBlock::Comment(_) => "",
        };
        name_a.cmp(name_b)
    });

    let mut lines = Vec::new();

    // Emit header comments first.
    for block in &header_comments {
        if let YamlBlock::Comment(text) = block {
            lines.push(text.clone());
        }
    }

    // Emit known-order keys.
    for (_, block) in &ordered {
        if let YamlBlock::Key { raw, .. } = block {
            lines.push(raw.clone());
        }
    }

    // Emit unknown keys after known keys.
    for block in &unknown {
        match block {
            YamlBlock::Key { raw, .. } => lines.push(raw.clone()),
            YamlBlock::Comment(text) => lines.push(text.clone()),
        }
    }

    // Clean up trailing whitespace on each line.
    let cleaned: Vec<String> = lines
        .iter()
        .flat_map(|block| block.lines().map(|l| l.trim_end().to_string()))
        .collect();

    cleaned.join("\n")
}

/// A parsed YAML block — either a top-level key (with its continuation lines)
/// or a standalone comment.
#[derive(Debug)]
enum YamlBlock {
    /// A top-level key with its full raw text (key line + continuation lines).
    Key {
        /// The key name (e.g., "name", "description").
        name: String,
        /// The full raw text including continuation lines.
        raw: String,
    },
    /// A standalone comment line.
    Comment(String),
}

/// Parse YAML text into blocks of top-level keys and comments.
///
/// A top-level key starts at column 0 with `key:` syntax. Continuation
/// lines start with whitespace (indented values, multiline scalars).
fn parse_yaml_blocks(yaml: &str) -> Vec<YamlBlock> {
    let mut blocks = Vec::new();
    let mut current_key: Option<(String, Vec<String>)> = None;

    for line in yaml.lines() {
        if line.is_empty() {
            // Blank lines are continuation of current block.
            if let Some((_, ref mut lines)) = current_key {
                lines.push(String::new());
            }
            continue;
        }

        if line.starts_with('#') && current_key.is_none() {
            // Standalone comment before any key.
            blocks.push(YamlBlock::Comment(line.to_string()));
            continue;
        }

        if line.starts_with(' ') || line.starts_with('#') {
            // Indented line or inline comment — continuation of current key.
            if let Some((_, ref mut lines)) = current_key {
                lines.push(line.to_string());
            }
            continue;
        }

        // New top-level key.
        if let Some((name, lines)) = current_key.take() {
            blocks.push(YamlBlock::Key {
                name,
                raw: lines.join("\n"),
            });
        }

        // Extract key name from `key:` or `key: value`.
        let key_name = line.split(':').next().unwrap_or("").trim().to_string();

        current_key = Some((key_name, vec![line.to_string()]));
    }

    // Flush remaining block.
    if let Some((name, lines)) = current_key {
        blocks.push(YamlBlock::Key {
            name,
            raw: lines.join("\n"),
        });
    }

    blocks
}

/// Format the markdown body.
///
/// Normalizations:
/// - Remove trailing whitespace from each line.
/// - Ensure file ends with exactly one newline.
/// - Collapse 3+ consecutive blank lines into 2.
fn format_body(body: &str) -> String {
    if body.is_empty() {
        return String::from("\n");
    }

    let lines: Vec<String> = body.lines().map(|l| l.trim_end().to_string()).collect();

    // Collapse consecutive blank lines (max 2).
    let mut result = Vec::new();
    let mut blank_count = 0;

    for line in &lines {
        if line.is_empty() {
            blank_count += 1;
            if blank_count <= 2 {
                result.push(String::new());
            }
        } else {
            blank_count = 0;
            result.push(line.clone());
        }
    }

    // Remove trailing blank lines.
    while result.last().is_some_and(|l| l.is_empty()) {
        result.pop();
    }

    // End with exactly one newline.
    let mut out = result.join("\n");
    out.push('\n');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_reorders_keys_canonically() {
        let input = "---\nmetadata:\n  version: '1.0'\nname: my-skill\ndescription: Does things\n---\nBody.\n";
        let result = format_content(input).unwrap();
        let lines: Vec<&str> = result.lines().collect();
        // name should come before metadata.
        let name_pos = lines.iter().position(|l| l.starts_with("name:")).unwrap();
        let meta_pos = lines
            .iter()
            .position(|l| l.starts_with("metadata:"))
            .unwrap();
        assert!(
            name_pos < meta_pos,
            "name (pos {name_pos}) should come before metadata (pos {meta_pos})"
        );
    }

    #[test]
    fn format_preserves_values() {
        let input = "---\nname: my-skill\ndescription: >-\n  A multiline description\n  that spans two lines\n---\nBody.\n";
        let result = format_content(input).unwrap();
        assert!(result.contains("description: >-"));
        assert!(result.contains("A multiline description"));
        assert!(result.contains("that spans two lines"));
    }

    #[test]
    fn format_is_idempotent() {
        let input = "---\nname: my-skill\ndescription: Does things\ncompatibility: claude-code\nmetadata:\n  version: '1.0'\n---\nBody text here.\n";
        let first = format_content(input).unwrap();
        let second = format_content(&first).unwrap();
        assert_eq!(first, second, "formatting should be idempotent");
    }

    #[test]
    fn format_removes_trailing_whitespace() {
        let input = "---\nname: my-skill   \ndescription: Does things  \n---\nBody text.   \n";
        let result = format_content(input).unwrap();
        for line in result.lines() {
            assert_eq!(
                line,
                line.trim_end(),
                "line should have no trailing whitespace: {line:?}"
            );
        }
    }

    #[test]
    fn format_ensures_single_trailing_newline() {
        let input = "---\nname: my-skill\ndescription: Does things\n---\nBody.\n\n\n";
        let result = format_content(input).unwrap();
        assert!(
            result.ends_with("Body.\n"),
            "should end with single newline"
        );
    }

    #[test]
    fn format_collapses_excessive_blank_lines() {
        let input = "---\nname: my-skill\ndescription: Does things\n---\nParagraph 1.\n\n\n\n\nParagraph 2.\n";
        let result = format_content(input).unwrap();
        assert!(
            !result.contains("\n\n\n\n"),
            "should collapse to max 2 blank lines"
        );
        assert!(result.contains("Paragraph 1.\n\n\nParagraph 2."));
    }

    #[test]
    fn format_unknown_keys_sorted_alphabetically() {
        let input =
            "---\nname: my-skill\ndescription: Does things\nzebra: yes\nalpha: no\n---\nBody.\n";
        let result = format_content(input).unwrap();
        let lines: Vec<&str> = result.lines().collect();
        let alpha_pos = lines.iter().position(|l| l.starts_with("alpha:")).unwrap();
        let zebra_pos = lines.iter().position(|l| l.starts_with("zebra:")).unwrap();
        assert!(
            alpha_pos < zebra_pos,
            "alpha should come before zebra (alphabetical)"
        );
    }

    #[test]
    fn format_error_on_missing_delimiters() {
        let result = format_content("no frontmatter here");
        assert!(result.is_err());
    }

    #[test]
    fn format_error_on_missing_closing_delimiter() {
        let result = format_content("---\nname: my-skill\ndescription: Does things\n");
        assert!(result.is_err());
    }

    #[test]
    fn format_preserves_comments() {
        let input = "---\n# Header comment\nname: my-skill\ndescription: Does things\n---\nBody.\n";
        let result = format_content(input).unwrap();
        assert!(
            result.contains("# Header comment"),
            "comments should be preserved"
        );
    }

    #[test]
    fn format_empty_body_gets_newline() {
        let input = "---\nname: my-skill\ndescription: Does things\n---\n";
        let result = format_content(input).unwrap();
        assert!(result.ends_with("---\n\n"), "empty body should get newline");
    }

    #[test]
    fn format_empty_frontmatter_does_not_panic() {
        let input = "---\n---\nBody.\n";
        let result = format_content(input).unwrap();
        assert!(result.contains("---\n\n---\n"));
    }

    #[test]
    fn format_crlf_produces_lf_output() {
        let crlf = "---\r\nname: my-skill\r\ndescription: A skill\r\n---\r\n\r\nBody text.\r\n";
        let result = format_content(crlf).unwrap();
        assert!(
            !result.contains("\r\n"),
            "output should not contain CRLF line endings"
        );
        assert!(result.contains("name: my-skill"));
        assert!(result.contains("description: A skill"));
        assert!(result.contains("Body text.\n"));
    }

    #[test]
    fn format_mixed_lf_crlf_normalizes_to_lf() {
        let mixed = "---\nname: my-skill\r\ndescription: A skill\n---\r\n\nBody text.\r\n";
        let result = format_content(mixed).unwrap();
        assert!(
            !result.contains("\r\n"),
            "output should not contain any CRLF after normalization"
        );
        assert!(result.contains("name: my-skill"));
        assert!(result.contains("description: A skill"));
    }

    #[test]
    fn format_lf_input_unchanged_by_normalization() {
        let lf = "---\nname: my-skill\ndescription: A skill\n---\nBody text.\n";
        let result = format_content(lf).unwrap();
        assert_eq!(result, lf, "LF-only input should produce identical output");
    }

    #[test]
    fn format_crlf_is_idempotent_after_normalization() {
        let crlf = "---\r\nname: my-skill\r\ndescription: A skill\r\n---\r\n\r\nBody text.\r\n";
        let first = format_content(crlf).unwrap();
        let second = format_content(&first).unwrap();
        assert_eq!(
            first, second,
            "formatting should be idempotent after CRLF normalization"
        );
    }
}
