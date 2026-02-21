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
    // Split into frontmatter and body.
    if !original.starts_with("---") {
        return Err(AigentError::Parse {
            message: "SKILL.md must start with --- delimiter".into(),
        });
    }

    let after_first = &original[3..];
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
    let mut interleaved_comments: Vec<&YamlBlock> = Vec::new();
    let mut seen_key = false;

    for block in &blocks {
        match block {
            YamlBlock::Comment(_) => {
                if !seen_key {
                    // Comments before any key are header comments.
                    header_comments.push(block);
                } else {
                    // Comments between keys stay anchored (not attached to any key).
                    interleaved_comments.push(block);
                }
            }
            YamlBlock::Key { name, .. } => {
                seen_key = true;
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

    // Emit interleaved comments in their original order, anchored between
    // known and unknown keys.
    for block in &interleaved_comments {
        if let YamlBlock::Comment(text) = block {
            lines.push(text.clone());
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

        if line.starts_with('#') {
            if let Some((name, lines)) = current_key.take() {
                // Flush preceding key block before the standalone comment.
                blocks.push(YamlBlock::Key {
                    name,
                    raw: lines.join("\n"),
                });
            }
            blocks.push(YamlBlock::Comment(line.to_string()));
            continue;
        }

        if line.starts_with(' ') {
            // Indented line — continuation of current key.
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
    fn standalone_comment_between_keys_stays_in_position() {
        // Comment between name and description should not travel with name
        // when keys are reordered. It should appear after name (canonical order),
        // anchored between known keys and unknown keys.
        let input = "---\ndescription: Does things\n# About the name\nname: my-skill\n---\nBody.\n";
        let result = format_content(input).unwrap();
        let lines: Vec<&str> = result.lines().collect();
        let name_pos = lines.iter().position(|l| l.starts_with("name:")).unwrap();
        let desc_pos = lines
            .iter()
            .position(|l| l.starts_with("description:"))
            .unwrap();
        let comment_pos = lines.iter().position(|l| *l == "# About the name").unwrap();
        // Keys reordered canonically: name before description.
        assert!(
            name_pos < desc_pos,
            "name should come before description after reorder"
        );
        // Comment is anchored between known keys and unknown keys (after all known keys).
        assert!(
            comment_pos > name_pos,
            "comment should appear after name (not attached to description)"
        );
    }

    #[test]
    fn inline_comment_stays_with_key() {
        // Inline comments (after values on the same line) are part of the key
        // line itself, not standalone comments. They should move with the key.
        let input =
            "---\nmetadata:\n  version: '1.0'\nname: my-skill  # the name\ndescription: Does things\n---\nBody.\n";
        let result = format_content(input).unwrap();
        assert!(
            result.contains("name: my-skill  # the name"),
            "inline comment should stay with its key value"
        );
    }

    #[test]
    fn multiple_consecutive_standalone_comments_preserved() {
        let input =
            "---\nname: my-skill\n# line 1\n# line 2\ndescription: Does things\n---\nBody.\n";
        let result = format_content(input).unwrap();
        assert!(
            result.contains("# line 1"),
            "first standalone comment should be preserved"
        );
        assert!(
            result.contains("# line 2"),
            "second standalone comment should be preserved"
        );
        // Both comments should appear in the output.
        let lines: Vec<&str> = result.lines().collect();
        let c1_pos = lines.iter().position(|l| *l == "# line 1").unwrap();
        let c2_pos = lines.iter().position(|l| *l == "# line 2").unwrap();
        assert_eq!(
            c2_pos,
            c1_pos + 1,
            "consecutive comments should remain adjacent"
        );
    }

    #[test]
    fn indented_comment_stays_with_preceding_key() {
        // An indented comment (e.g., inside a multiline block) should stay
        // with the preceding key, not be treated as standalone.
        let input = "---\nname: my-skill\ndescription: |\n  A description.\n  # This is inside the block.\nmetadata:\n  version: '1.0'\n---\nBody.\n";
        let result = format_content(input).unwrap();
        let lines: Vec<&str> = result.lines().collect();
        let desc_pos = lines
            .iter()
            .position(|l| l.starts_with("description:"))
            .unwrap();
        let indented_comment_pos = lines
            .iter()
            .position(|l| l.contains("# This is inside the block."))
            .unwrap();
        let meta_pos = lines
            .iter()
            .position(|l| l.starts_with("metadata:"))
            .unwrap();
        assert!(
            indented_comment_pos > desc_pos && indented_comment_pos < meta_pos,
            "indented comment should remain between description and metadata"
        );
    }

    #[test]
    fn header_comment_above_first_key_preserved() {
        // Regression test (A3): comments above the first key should remain
        // at the top of the frontmatter after formatting.
        let input = "---\n# This is a skill file\nname: my-skill\ndescription: Does things\nmetadata:\n  version: '1.0'\n---\nBody.\n";
        let result = format_content(input).unwrap();
        let lines: Vec<&str> = result.lines().collect();
        // The header comment should be above the first key (name).
        let comment_pos = lines
            .iter()
            .position(|l| *l == "# This is a skill file")
            .unwrap();
        let name_pos = lines.iter().position(|l| l.starts_with("name:")).unwrap();
        assert!(
            comment_pos < name_pos,
            "header comment (pos {comment_pos}) should appear before name (pos {name_pos})"
        );
        // And the comment should be right after the opening ---.
        assert_eq!(
            comment_pos, 1,
            "header comment should be on line 1 (after ---)"
        );
    }

    #[test]
    fn comment_handling_is_idempotent() {
        // Formatting content with comments should be idempotent.
        let input =
            "---\n# Header\nname: my-skill\n# Between\ndescription: Does things\n---\nBody.\n";
        let first = format_content(input).unwrap();
        let second = format_content(&first).unwrap();
        assert_eq!(
            first, second,
            "formatting with comments should be idempotent"
        );
    }
}
