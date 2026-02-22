//! Skill tester and previewer for evaluation-driven development.
//!
//! Simulates how Claude would discover and activate a skill given a sample
//! user query. Shows what metadata would be injected into the system prompt
//! and identifies potential issues (description mismatch, broken references,
//! token budget).

use std::path::Path;

use crate::diagnostics::Diagnostic;
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

/// Format a test result as human-readable text.
#[must_use]
pub fn format_test_result(result: &TestResult) -> String {
    let mut out = String::new();

    // Aligned label width (widest label is "Description:" at 12 chars + 1 padding).
    const W: usize = 13;

    out.push_str(&format!("{:<W$} {}\n", "Skill:", result.name));
    out.push_str(&format!("{:<W$} \"{}\"\n", "Query:", result.query));
    out.push_str(&format!("{:<W$} {}\n", "Description:", result.description));
    out.push('\n');

    // Query match assessment.
    let match_label = match &result.query_match {
        QueryMatch::Strong => "STRONG ✓ — description aligns well with query",
        QueryMatch::Weak => "WEAK ⚠ — some overlap, but description may not trigger reliably",
        QueryMatch::None => "NONE ✗ — description does not match the test query",
    };
    out.push_str(&format!(
        "{:<W$} {match_label} (score: {:.2})\n",
        "Activation:", result.score
    ));

    // Token budget.
    out.push_str(&format!(
        "{:<W$} ~{} tokens\n",
        "Tokens:", result.estimated_tokens
    ));
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

/// Normalize a word by stripping common English suffixes.
///
/// This is a minimal stemmer, not a full Porter/Snowball implementation.
/// It handles the most common cases to improve Jaccard overlap.
fn stem(word: &str) -> String {
    let w = word.to_lowercase();
    // Order matters: check longer suffixes first.
    for suffix in &[
        "ting", "sing", "zing", "ning", "ring", "ses", "ies", "ing", "ed", "es", "s",
    ] {
        if w.len() > suffix.len() + 2 {
            if let Some(root) = w.strip_suffix(suffix) {
                return root.to_string();
            }
        }
    }
    w
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

/// Extract a trigger phrase from a description.
///
/// Scans for lines starting with "Use when" or "Use this when"
/// (case-insensitive) and returns the full line text if found.
fn extract_trigger(description: &str) -> Option<String> {
    for line in description.lines() {
        let trimmed = line.trim();
        let lower = trimmed.to_lowercase();
        if lower.starts_with("use when") || lower.starts_with("use this when") {
            return Some(trimmed.to_string());
        }
    }
    None
}

/// Compute a weighted match score between a query and a skill.
///
/// Uses a three-component weighted formula:
/// - **0.5 × description overlap** (fraction of query tokens found in description)
/// - **0.3 × trigger match**: 1.0 if any query token appears in the
///   skill's trigger phrase ("Use when..."), 0.0 otherwise
/// - **0.2 × name match**: 1.0 if any query token is a substring of the
///   skill name, 0.0 otherwise
///
/// Returns the [`QueryMatch`] category and the numeric score (0.0–1.0).
/// Strong ≥ 0.4, Weak ≥ 0.15, None < 0.15.
fn compute_query_match(query: &str, name: &str, description: &str) -> (QueryMatch, f64) {
    let query_tokens = tokenize(query);

    if query_tokens.is_empty() {
        return (QueryMatch::None, 0.0);
    }

    let desc_tokens = tokenize(description);

    // Description overlap: fraction of query tokens found in description tokens.
    // This measures recall (how many query terms are covered) rather than
    // Jaccard (which penalizes for extra description tokens).
    let query_set: std::collections::HashSet<&str> =
        query_tokens.iter().map(|s| s.as_str()).collect();
    let desc_set: std::collections::HashSet<&str> =
        desc_tokens.iter().map(|s| s.as_str()).collect();
    let intersection = query_set.intersection(&desc_set).count();
    let desc_overlap = if query_set.is_empty() {
        0.0
    } else {
        intersection as f64 / query_set.len() as f64
    };

    // Trigger match: 1.0 if any query token appears in the trigger phrase.
    let trigger_score = if let Some(trigger) = extract_trigger(description) {
        let trigger_lower = trigger.to_lowercase();
        if query_tokens
            .iter()
            .any(|t| trigger_lower.contains(t.as_str()))
        {
            1.0
        } else {
            0.0
        }
    } else {
        0.0
    };

    // Name match: 1.0 if any query token is a substring of the skill name.
    let name_lower = name.to_lowercase();
    let name_score = if query_tokens.iter().any(|t| name_lower.contains(t.as_str())) {
        1.0
    } else {
        0.0
    };

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
        assert_eq!(result.query_match, QueryMatch::Strong);
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
}
