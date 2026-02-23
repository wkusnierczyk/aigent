//! Skill tester and previewer for evaluation-driven development.
//!
//! Simulates how Claude would discover and activate a skill given a sample
//! user query. Shows what metadata would be injected into the system prompt
//! and identifies potential issues (description mismatch, broken references,
//! token budget).

use std::collections::HashSet;
use std::path::Path;

use rust_stemmers::{Algorithm, Stemmer};

use crate::diagnostics::Diagnostic;
use crate::linter::TRIGGER_PHRASES;
use crate::models::SkillProperties;
use crate::parser::read_properties;
use crate::prompt::estimate_tokens;
use crate::structure::validate_structure;
use crate::validator::validate;
use crate::Result;

/// Result of testing a skill against a sample query.
#[derive(Debug)]
pub struct TestResult {
    /// Skill name from frontmatter.
    pub name: String,
    /// Skill description from frontmatter.
    pub description: String,
    /// The test query provided by the user.
    pub query: String,
    /// Whether the description appears relevant to the query.
    pub query_match: QueryMatch,
    /// Numeric match score (0.0–1.0) from the weighted formula.
    pub score: f64,
    /// Estimated token cost of the skill's prompt footprint.
    pub estimated_tokens: usize,
    /// Validation diagnostics (errors + warnings).
    pub diagnostics: Vec<Diagnostic>,
    /// Structure diagnostics (missing references, etc.).
    pub structure_diagnostics: Vec<Diagnostic>,
    /// Parsed properties for display purposes.
    pub properties: SkillProperties,
}

/// Describes how well the skill description matches a test query.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueryMatch {
    /// Strong match: weighted score ≥ 0.4.
    Strong,
    /// Weak match: weighted score ≥ 0.15.
    Weak,
    /// No match: weighted score < 0.15.
    None,
}

/// Test a skill against a sample user query.
///
/// Simulates skill discovery by checking:
/// 1. Whether the description is relevant to the query (word overlap)
/// 2. Whether the skill passes validation (metadata + structure)
/// 3. The estimated token cost
///
/// # Arguments
///
/// * `dir` - Path to the skill directory
/// * `query` - A sample user query to test activation against
///
/// # Errors
///
/// Returns an error if the SKILL.md cannot be read or parsed.
pub fn test_skill(dir: &Path, query: &str) -> Result<TestResult> {
    let properties = read_properties(dir)?;

    // Compute weighted match score and category.
    let (query_match, score) =
        compute_query_match(query, &properties.name, &properties.description);

    // Estimate token footprint: name + description (what goes into system prompt).
    let estimated_tokens =
        estimate_tokens(&properties.name) + estimate_tokens(&properties.description);

    // Run standard validation.
    let diagnostics = validate(dir);

    // Run structure validation.
    let structure_diagnostics = validate_structure(dir);

    Ok(TestResult {
        name: properties.name.clone(),
        description: properties.description.clone(),
        query: query.to_string(),
        query_match,
        score,
        estimated_tokens,
        diagnostics,
        structure_diagnostics,
        properties,
    })
}

/// Default terminal width for wrapping probe output.
const DEFAULT_WIDTH: usize = 80;

/// Format a labeled line, wrapping long values so continuation lines align
/// to the value column. Uses character counts (not byte offsets) so that
/// multibyte UTF-8 content (e.g., `✓`, `⚠`, `—`) never causes a panic.
fn fmt_field(out: &mut String, label: &str, value: &str, col: usize, width: usize) {
    let prefix = format!("{:<col$} ", label);
    let indent = col + 1; // spaces for continuation lines
    let max_val = width.saturating_sub(indent);
    if max_val == 0 || value.chars().count() + indent <= width {
        out.push_str(&prefix);
        out.push_str(value);
        out.push('\n');
        return;
    }
    // Collect char indices for safe slicing on character boundaries.
    let chars: Vec<(usize, char)> = value.char_indices().collect();
    let mut char_pos = 0; // index into `chars`
    let mut first = true;
    while char_pos < chars.len() {
        // Skip leading spaces at break boundaries to avoid blank lines.
        if !first {
            while char_pos < chars.len() && chars[char_pos].1 == ' ' {
                char_pos += 1;
            }
            if char_pos >= chars.len() {
                break;
            }
        }
        if first {
            out.push_str(&prefix);
        } else {
            for _ in 0..indent {
                out.push(' ');
            }
        }
        let remaining_chars = chars.len() - char_pos;
        if remaining_chars <= max_val {
            let byte_start = chars[char_pos].0;
            out.push_str(&value[byte_start..]);
            out.push('\n');
            break;
        }
        // Find the last space within max_val characters.
        let end = char_pos + max_val;
        let break_char = (char_pos..end)
            .rev()
            .find(|&i| chars[i].1 == ' ')
            .unwrap_or(end);
        let byte_start = chars[char_pos].0;
        let byte_end = chars[break_char].0;
        out.push_str(&value[byte_start..byte_end]);
        out.push('\n');
        char_pos = break_char;
        first = false;
    }
}

/// Format a test result as human-readable text.
#[must_use]
pub fn format_test_result(result: &TestResult) -> String {
    format_test_result_width(result, DEFAULT_WIDTH)
}

/// Format a test result with a specific terminal width (for testing).
#[must_use]
pub(crate) fn format_test_result_width(result: &TestResult, width: usize) -> String {
    let mut out = String::new();

    // Aligned label width (widest label is "Description:" at 12 chars + 1 padding).
    const W: usize = 13;

    fmt_field(&mut out, "Skill:", &result.name, W, width);
    fmt_field(
        &mut out,
        "Query:",
        &format!("\"{}\"", result.query),
        W,
        width,
    );
    fmt_field(&mut out, "Description:", &result.description, W, width);
    out.push('\n');

    // Query match assessment.
    let match_label = match &result.query_match {
        QueryMatch::Strong => "STRONG ✓ — description aligns well with query",
        QueryMatch::Weak => "WEAK ⚠ — some overlap, but description may not trigger reliably",
        QueryMatch::None => "NONE ✗ — description does not match the test query",
    };
    fmt_field(
        &mut out,
        "Activation:",
        &format!("{match_label} (score: {:.2})", result.score),
        W,
        width,
    );

    // Token budget.
    fmt_field(
        &mut out,
        "Tokens:",
        &format!("~{} tokens", result.estimated_tokens),
        W,
        width,
    );
    out.push('\n');

    // Validation results.
    let errors: Vec<_> = result.diagnostics.iter().filter(|d| d.is_error()).collect();
    let warnings: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.is_warning())
        .collect();

    if errors.is_empty() && warnings.is_empty() && result.structure_diagnostics.is_empty() {
        out.push_str("Validation: PASS — no issues found\n");
    } else {
        if !errors.is_empty() {
            out.push_str(&format!("Validation errors ({}):\n", errors.len()));
            for d in &errors {
                out.push_str(&format!("  {d}\n"));
            }
        }
        if !warnings.is_empty() {
            out.push_str(&format!("Validation warnings ({}):\n", warnings.len()));
            for d in &warnings {
                out.push_str(&format!("  {d}\n"));
            }
        }
        if !result.structure_diagnostics.is_empty() {
            out.push_str(&format!(
                "Structure issues ({}):\n",
                result.structure_diagnostics.len()
            ));
            for d in &result.structure_diagnostics {
                out.push_str(&format!("  {d}\n"));
            }
        }
    }

    out
}

/// Common English stopwords excluded from token matching.
const STOPWORDS: &[&str] = &[
    "a", "an", "the", "is", "are", "was", "were", "of", "to", "in", "for", "on", "with", "and",
    "or", "but", "not", "it", "this", "that",
];

/// Synonym groups for common skill-domain terms.
///
/// When a query token matches any word in a group, all words in that group
/// are added to the expanded query set. Applied to query tokens only (not
/// description) to improve recall without inflating description tokens.
const SYNONYM_GROUPS: &[&[&str]] = &[
    &["valid", "check", "verifi", "lint"],
    &["pars", "extract", "read"],
    &["format", "style", "clean"],
    &["build", "assembl", "compil"],
    &["test", "probe", "evalu"],
    &["creat", "generat", "new"],
    &["fix", "repair", "correct"],
    &["upgrad", "improv", "enhanc"],
    &["document", "describ", "explain"],
    &["delet", "remov", "strip"],
    &["instal", "setup", "configur"],
    &["deploy", "publish", "releas"],
    &["search", "find", "discov", "locat"],
    &["transform", "convert", "process"],
    &["analyz", "inspect", "review"],
];

/// Stem a word using the Snowball English stemmer.
fn stem(word: &str) -> String {
    let stemmer = Stemmer::create(Algorithm::English);
    stemmer.stem(&word.to_lowercase()).into_owned()
}

/// Tokenize a string into lowercase, stemmed words with punctuation stripped
/// and stopwords removed.
fn tokenize(text: &str) -> Vec<String> {
    text.split_whitespace()
        .map(|w| {
            let cleaned = w
                .trim_matches(|c: char| !c.is_alphanumeric())
                .to_lowercase();
            stem(&cleaned)
        })
        .filter(|w| !w.is_empty() && !STOPWORDS.contains(&w.as_str()))
        .collect()
}

/// Expand query tokens with synonyms from `SYNONYM_GROUPS`.
///
/// For each query token that matches a word in any synonym group,
/// all other words in that group are added to the result set.
/// Original tokens are always preserved.
fn expand_synonyms(tokens: &[String]) -> HashSet<String> {
    let mut expanded: HashSet<String> = tokens.iter().cloned().collect();
    for token in tokens {
        for group in SYNONYM_GROUPS {
            if group.contains(&token.as_str()) {
                for &syn in *group {
                    expanded.insert(syn.to_string());
                }
            }
        }
    }
    expanded
}

/// Extract a trigger phrase from a description.
///
/// Scans for lines containing any of the shared `TRIGGER_PHRASES`
/// (case-insensitive) and returns the full line text if found.
fn extract_trigger(description: &str) -> Option<String> {
    for line in description.lines() {
        let trimmed = line.trim();
        let lower = trimmed.to_lowercase();
        if TRIGGER_PHRASES.iter().any(|p| lower.contains(p)) {
            return Some(trimmed.to_string());
        }
    }
    None
}

/// Compute a weighted match score between a query and a skill.
///
/// Uses a three-component weighted formula:
/// - **0.5 × description overlap** (fraction of query tokens found in description,
///   with synonym expansion on the query side)
/// - **0.3 × trigger score** (fraction of query tokens found in the trigger phrase)
/// - **0.2 × name score** (fraction of query tokens found as substrings of the name)
///
/// Returns the [`QueryMatch`] category and the numeric score (0.0–1.0).
/// Strong ≥ 0.4, Weak ≥ 0.15, None < 0.15.
fn compute_query_match(query: &str, name: &str, description: &str) -> (QueryMatch, f64) {
    let query_tokens = tokenize(query);

    if query_tokens.is_empty() {
        return (QueryMatch::None, 0.0);
    }

    let desc_tokens = tokenize(description);

    // Expand query tokens with synonyms for better recall.
    let expanded_query = expand_synonyms(&query_tokens);

    // Description overlap: fraction of expanded query tokens found in description tokens.
    let desc_set: HashSet<&str> = desc_tokens.iter().map(|s| s.as_str()).collect();
    let intersection = expanded_query
        .iter()
        .filter(|t| desc_set.contains(t.as_str()))
        .count();
    let desc_overlap = if expanded_query.is_empty() {
        0.0
    } else {
        intersection as f64 / expanded_query.len() as f64
    };

    // Trigger score: fraction of query tokens found in the trigger phrase.
    let trigger_score = if let Some(trigger) = extract_trigger(description) {
        let trigger_tokens = tokenize(&trigger);
        let trigger_set: HashSet<&str> = trigger_tokens.iter().map(|s| s.as_str()).collect();
        let matched = query_tokens
            .iter()
            .filter(|t| trigger_set.contains(t.as_str()))
            .count();
        matched as f64 / query_tokens.len() as f64
    } else {
        0.0
    };

    // Name score: fraction of query tokens found as substrings of the skill name.
    let name_lower = name.to_lowercase();
    let matched = query_tokens
        .iter()
        .filter(|t| name_lower.contains(t.as_str()))
        .count();
    let name_score = matched as f64 / query_tokens.len() as f64;

    // Weighted formula.
    let score = 0.5 * desc_overlap + 0.3 * trigger_score + 0.2 * name_score;

    let category = if score >= 0.4 {
        QueryMatch::Strong
    } else if score >= 0.15 {
        QueryMatch::Weak
    } else {
        QueryMatch::None
    };

    (category, score)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    /// Create a skill dir with given frontmatter + body.
    fn make_skill(
        name: &str,
        description: &str,
        body: &str,
    ) -> (tempfile::TempDir, std::path::PathBuf) {
        let parent = tempdir().unwrap();
        let dir = parent.path().join(name);
        fs::create_dir(&dir).unwrap();
        fs::write(
            dir.join("SKILL.md"),
            format!("---\nname: {name}\ndescription: {description}\n---\n{body}\n"),
        )
        .unwrap();
        (parent, dir)
    }

    // ── Query matching ───────────────────────────────────────────────

    #[test]
    fn strong_match_when_query_words_in_description() {
        let (m, score) = compute_query_match(
            "process PDF files",
            "pdf-processor",
            "Processes PDF files and generates detailed reports",
        );
        assert_eq!(m, QueryMatch::Strong);
        assert!(score >= 0.4, "score {score} should be ≥ 0.4");
    }

    #[test]
    fn weak_match_with_partial_overlap() {
        let (m, score) = compute_query_match(
            "generate database migration scripts quickly",
            "pdf-processor",
            "Processes PDF files and generates detailed reports",
        );
        assert!(
            matches!(m, QueryMatch::Weak | QueryMatch::None),
            "expected Weak or None for partial overlap, got {m:?} (score: {score})"
        );
    }

    #[test]
    fn no_match_with_unrelated_query() {
        let (m, score) = compute_query_match(
            "deploy kubernetes cluster",
            "pdf-processor",
            "Processes PDF files and generates detailed reports",
        );
        assert_eq!(m, QueryMatch::None);
        assert!(score < 0.15, "score {score} should be < 0.15");
    }

    #[test]
    fn empty_query_is_no_match() {
        let (m, score) = compute_query_match("", "some-skill", "Some description");
        assert_eq!(m, QueryMatch::None);
        assert_eq!(score, 0.0);
    }

    #[test]
    fn case_insensitive_matching() {
        let (m, _score) = compute_query_match(
            "PDF PROCESSING",
            "pdf-processor",
            "Processes pdf files and generates reports",
        );
        assert!(
            matches!(m, QueryMatch::Strong | QueryMatch::Weak),
            "expected Strong or Weak for case-insensitive match, got {m:?}"
        );
    }

    // ── Weighted scoring specific tests ──────────────────────────────

    #[test]
    fn trigger_phrase_boosts_score() {
        // Use identical base descriptions + same extra words to isolate the trigger effect.
        // The trigger bonus (0.3) should outweigh any Jaccard dilution from extra tokens.
        let (_, score_with_trigger) = compute_query_match(
            "lint javascript",
            "unrelated-name",
            "Analyzes syntax patterns. Use when you want to lint javascript files.",
        );
        let (_, score_without_trigger) = compute_query_match(
            "lint javascript",
            "unrelated-name",
            "Analyzes syntax patterns in various source files.",
        );
        assert!(
            score_with_trigger > score_without_trigger,
            "trigger phrase should boost score: {score_with_trigger} vs {score_without_trigger}"
        );
    }

    #[test]
    fn name_match_boosts_score() {
        let (_, score_name_match) = compute_query_match(
            "process pdf",
            "pdf-processor",
            "Handles document transformation tasks.",
        );
        let (_, score_no_name) = compute_query_match(
            "process pdf",
            "document-handler",
            "Handles document transformation tasks.",
        );
        assert!(
            score_name_match > score_no_name,
            "name match should boost score: {score_name_match} vs {score_no_name}"
        );
    }

    #[test]
    fn all_zero_inputs_produce_zero_score() {
        let (m, score) = compute_query_match(
            "xylophone zephyr",
            "unrelated-name",
            "Completely unrelated description about cooking pasta.",
        );
        assert_eq!(m, QueryMatch::None);
        assert_eq!(score, 0.0, "totally unrelated query should score 0.0");
    }

    // ── test_skill integration ───────────────────────────────────────

    #[test]
    fn test_skill_returns_result_for_valid_skill() {
        let (_parent, dir) = make_skill(
            "pdf-tool",
            "Processes PDF files and extracts text content",
            "Body content here.",
        );
        let result = test_skill(&dir, "process some PDF files").unwrap();
        assert_eq!(result.name, "pdf-tool");
        assert!(
            matches!(result.query_match, QueryMatch::Strong | QueryMatch::Weak),
            "expected Strong or Weak for relevant query, got {:?} (score: {})",
            result.query_match,
            result.score,
        );
        assert!(result.estimated_tokens > 0);
    }

    #[test]
    fn test_skill_reports_validation_issues() {
        let parent = tempdir().unwrap();
        let dir = parent.path().join("bad-skill");
        fs::create_dir(&dir).unwrap();
        // Missing description → validation error.
        fs::write(dir.join("SKILL.md"), "---\nname: bad-skill\n---\nBody.\n").unwrap();
        let result = test_skill(&dir, "anything");
        // Should fail because description is required.
        assert!(result.is_err());
    }

    #[test]
    fn test_skill_detects_structure_issues() {
        let (_parent, dir) = make_skill(
            "ref-skill",
            "Skill with broken reference",
            "See [guide](nonexistent.md) for details.",
        );
        let result = test_skill(&dir, "guide reference").unwrap();
        assert!(
            !result.structure_diagnostics.is_empty(),
            "expected structure diagnostics for broken reference",
        );
    }

    // ── format_test_result ───────────────────────────────────────────

    #[test]
    fn format_includes_skill_name_and_query() {
        let (_parent, dir) =
            make_skill("format-test", "A test skill for formatting output", "Body.");
        let result = test_skill(&dir, "test formatting").unwrap();
        let text = format_test_result(&result);
        assert!(text.contains("format-test"));
        assert!(text.contains("test formatting"));
    }

    #[test]
    fn format_shows_activation_status() {
        let (_parent, dir) = make_skill("activation-test", "Processes PDF files quickly", "Body.");
        let result = test_skill(&dir, "deploy kubernetes cluster").unwrap();
        let text = format_test_result(&result);
        assert!(text.contains("NONE"));
    }

    #[test]
    fn format_shows_pass_for_clean_skill() {
        let (_parent, dir) = make_skill(
            "clean-skill",
            "A clean skill that passes validation",
            "Body content.",
        );
        let result = test_skill(&dir, "clean skill").unwrap();
        let text = format_test_result(&result);
        assert!(text.contains("PASS"));
    }

    // ── fmt_field wrapping ──────────────────────────────────────────

    #[test]
    fn fmt_field_short_value_no_wrap() {
        let mut out = String::new();
        fmt_field(&mut out, "Label:", "short", 13, 80);
        assert_eq!(out, "Label:        short\n");
    }

    #[test]
    fn fmt_field_long_value_wraps_aligned() {
        let mut out = String::new();
        // Width 40, col 13 → 14 chars for indent, 26 chars for value per line.
        fmt_field(
            &mut out,
            "Description:",
            "Validates AI agent skill definitions against the spec",
            13,
            40,
        );
        let lines: Vec<&str> = out.lines().collect();
        assert!(lines.len() > 1, "expected wrapping, got: {out:?}");
        // All continuation lines must start with 14 spaces.
        for line in &lines[1..] {
            assert!(
                line.starts_with("              "),
                "continuation not aligned: {line:?}",
            );
        }
    }

    #[test]
    fn fmt_field_multibyte_utf8_no_panic() {
        let mut out = String::new();
        // Value contains multibyte chars (✓=3 bytes, ⚠=3, —=3).
        // At width 50, indent 14, max_val=36 chars — slicing must use
        // char boundaries, not byte offsets, to avoid a panic.
        fmt_field(
            &mut out,
            "Activation:",
            "WEAK ⚠ — some overlap, but description may not trigger reliably (score: 0.33)",
            13,
            50,
        );
        let lines: Vec<&str> = out.lines().collect();
        assert!(lines.len() > 1, "expected wrapping, got: {out:?}");
        for line in &lines[1..] {
            assert!(
                line.starts_with("              "),
                "continuation not aligned: {line:?}",
            );
        }
    }

    #[test]
    fn fmt_field_char_count_not_byte_len() {
        let mut out = String::new();
        // "café" is 4 chars but 5 bytes (é = 2 bytes).
        // With width=20, indent=7, max_val=13: "café latte warm" is 15 chars,
        // triggers wrapping. Byte-based slicing would panic or break incorrectly.
        fmt_field(&mut out, "Item:", "café latte warm drink", 6, 20);
        assert!(!out.is_empty(), "should produce output without panic",);
        // Verify no line exceeds the width in characters.
        for line in out.lines() {
            assert!(
                line.chars().count() <= 20,
                "line exceeds width: {line:?} ({} chars)",
                line.chars().count(),
            );
        }
    }

    #[test]
    fn fmt_field_consecutive_spaces_no_blank_lines() {
        let mut out = String::new();
        fmt_field(&mut out, "Label:", "word   word   word   end", 6, 18);
        for line in out.lines() {
            let trimmed = line.trim();
            assert!(!trimmed.is_empty(), "blank continuation line: {out:?}");
        }
    }

    #[test]
    fn format_test_result_wraps_description() {
        let long_desc = "Validates AI agent skill definitions (SKILL.md files) against \
            the Anthropic agent skill specification and checks all fields";
        let (_parent, dir) = make_skill("wrap-test", long_desc, "Body content.");
        let result = test_skill(&dir, "validate skill").unwrap();
        let text = format_test_result_width(&result, 60);
        let desc_lines: Vec<&str> = text
            .lines()
            .skip_while(|l| !l.starts_with("Description:"))
            .take_while(|l| !l.is_empty())
            .collect();
        assert!(
            desc_lines.len() > 1,
            "description should wrap at width 60: {desc_lines:?}",
        );
        for line in &desc_lines[1..] {
            assert!(
                line.starts_with("              "),
                "continuation not aligned: {line:?}",
            );
        }
    }
}
