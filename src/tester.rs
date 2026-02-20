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
    /// Strong match: multiple query words appear in the description.
    Strong,
    /// Weak match: some query words appear but overlap is low.
    Weak,
    /// No match: the query and description share no meaningful words.
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

    // Compute query/description overlap.
    let query_match = compute_query_match(query, &properties.description);

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

    out.push_str(&format!("Skill: {}\n", result.name));
    out.push_str(&format!("Query: \"{}\"\n", result.query));
    out.push_str(&format!("Description: {}\n", result.description));
    out.push('\n');

    // Query match assessment.
    let match_label = match &result.query_match {
        QueryMatch::Strong => "STRONG ✓ — description aligns well with query",
        QueryMatch::Weak => "WEAK ⚠ — some overlap, but description may not trigger reliably",
        QueryMatch::None => "NONE ✗ — description does not match the test query",
    };
    out.push_str(&format!("Activation: {match_label}\n"));

    // Token budget.
    out.push_str(&format!(
        "Token footprint: ~{} tokens\n",
        result.estimated_tokens
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

/// Compute word overlap between a query and a skill description.
///
/// Tokenizes both into lowercase words and computes what fraction of query
/// words appear in the description. Returns [`QueryMatch::Strong`] if ≥50%
/// of query words match, [`QueryMatch::Weak`] if ≥20%, [`QueryMatch::None`]
/// otherwise.
fn compute_query_match(query: &str, description: &str) -> QueryMatch {
    let query_words: Vec<&str> = query
        .split_whitespace()
        .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric()))
        .filter(|w| !w.is_empty() && w.len() > 2) // skip short stopwords
        .collect();

    if query_words.is_empty() {
        return QueryMatch::None;
    }

    let desc_lower = description.to_lowercase();

    let matches = query_words
        .iter()
        .filter(|w| desc_lower.contains(&w.to_lowercase()))
        .count();

    let ratio = matches as f64 / query_words.len() as f64;

    if ratio >= 0.5 {
        QueryMatch::Strong
    } else if ratio >= 0.2 {
        QueryMatch::Weak
    } else {
        QueryMatch::None
    }
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
        let m = compute_query_match(
            "process PDF files",
            "Processes PDF files and generates detailed reports",
        );
        assert_eq!(m, QueryMatch::Strong);
    }

    #[test]
    fn weak_match_with_partial_overlap() {
        let m = compute_query_match(
            "generate database migration scripts quickly",
            "Processes PDF files and generates detailed reports",
        );
        assert_eq!(m, QueryMatch::Weak);
    }

    #[test]
    fn no_match_with_unrelated_query() {
        let m = compute_query_match(
            "deploy kubernetes cluster",
            "Processes PDF files and generates detailed reports",
        );
        assert_eq!(m, QueryMatch::None);
    }

    #[test]
    fn empty_query_is_no_match() {
        let m = compute_query_match("", "Some description");
        assert_eq!(m, QueryMatch::None);
    }

    #[test]
    fn case_insensitive_matching() {
        let m = compute_query_match(
            "PDF PROCESSING",
            "Processes pdf files and generates reports",
        );
        assert_eq!(m, QueryMatch::Strong);
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
