//! Cross-skill conflict detection for skill collections.
//!
//! Analyzes collections of skills for potential conflicts: name collisions,
//! description similarity, and token budget overruns. Uses diagnostic codes
//! C001–C003.

use std::collections::HashSet;

use crate::diagnostics::{Diagnostic, Severity, C001, C002, C003};
use crate::prompt::{estimate_tokens, SkillEntry};

/// Default similarity threshold for description overlap detection.
///
/// Two skills with a Jaccard similarity above this threshold are flagged
/// as potentially conflicting. This is a heuristic value — use
/// `--similarity-threshold` to adjust.
const DEFAULT_SIMILARITY_THRESHOLD: f64 = 0.7;

/// Token budget warning threshold.
///
/// Total estimated token usage above this threshold triggers a C003 warning.
const TOKEN_BUDGET_THRESHOLD: usize = 4000;

/// Detect conflicts across a collection of skills.
///
/// Runs three checks:
/// - C001: Name collisions (same name in different locations)
/// - C002: Description similarity above threshold
/// - C003: Total token budget exceeded
///
/// # Arguments
///
/// * `entries` - Skill entries to check
///
/// # Returns
///
/// A list of warning diagnostics. Empty means no conflicts detected.
#[must_use]
pub fn detect_conflicts(entries: &[SkillEntry]) -> Vec<Diagnostic> {
    detect_conflicts_with_threshold(entries, DEFAULT_SIMILARITY_THRESHOLD)
}

/// Detect conflicts with a custom similarity threshold.
///
/// Same as [`detect_conflicts`] but allows overriding the Jaccard similarity
/// threshold for C002 checks.
#[must_use]
pub fn detect_conflicts_with_threshold(
    entries: &[SkillEntry],
    similarity_threshold: f64,
) -> Vec<Diagnostic> {
    let mut diags = Vec::new();
    diags.extend(check_name_collisions(entries));
    diags.extend(check_description_similarity(entries, similarity_threshold));
    diags.extend(check_token_budget(entries));
    diags
}

/// C001: Check for name collisions across skill directories.
fn check_name_collisions(entries: &[SkillEntry]) -> Vec<Diagnostic> {
    let mut diags = Vec::new();
    let mut seen: HashSet<&str> = HashSet::new();

    for entry in entries {
        if !seen.insert(&entry.name) {
            diags.push(
                Diagnostic::new(
                    Severity::Warning,
                    C001,
                    format!(
                        "name collision: '{}' appears in multiple locations",
                        entry.name
                    ),
                )
                .with_field("name")
                .with_suggestion("Rename one of the conflicting skills"),
            );
        }
    }

    diags
}

/// C002: Check for description similarity between skills.
///
/// Uses Jaccard similarity (word overlap ratio) to detect skills that
/// might trigger on the same queries. Pre-tokenizes descriptions once
/// before the O(n^2) comparison loop to avoid repeated per-pair allocations.
fn check_description_similarity(entries: &[SkillEntry], threshold: f64) -> Vec<Diagnostic> {
    // Pre-tokenize once: O(n)
    let token_sets: Vec<HashSet<String>> =
        entries.iter().map(|e| tokenize(&e.description)).collect();

    // Compare pairs: O(n^2) but no per-pair allocation
    let mut diags = Vec::new();
    for i in 0..entries.len() {
        for j in (i + 1)..entries.len() {
            let sim = jaccard_from_sets(&token_sets[i], &token_sets[j]);
            if sim >= threshold {
                diags.push(
                    Diagnostic::new(
                        Severity::Warning,
                        C002,
                        format!(
                            "description overlap ({:.0}%): '{}' and '{}'",
                            sim * 100.0,
                            entries[i].name,
                            entries[j].name,
                        ),
                    )
                    .with_field("description")
                    .with_suggestion("Differentiate descriptions to avoid activation conflicts"),
                );
            }
        }
    }

    diags
}

/// C003: Check total token budget across all skills.
fn check_token_budget(entries: &[SkillEntry]) -> Vec<Diagnostic> {
    let total: usize = entries.iter().map(estimate_entry_tokens).sum();

    if total > TOKEN_BUDGET_THRESHOLD {
        vec![
            Diagnostic::new(
                Severity::Warning,
                C003,
                format!(
                    "total estimated tokens ({total}) exceed budget threshold ({TOKEN_BUDGET_THRESHOLD})"
                ),
            )
            .with_field("collection")
            .with_suggestion("Remove or consolidate skills to reduce token usage"),
        ]
    } else {
        vec![]
    }
}

/// Estimate tokens for a single skill entry.
///
/// Estimates from name + description only, since those are the fields
/// injected into the system prompt. The full SKILL.md body is not
/// part of `SkillEntry`.
fn estimate_entry_tokens(entry: &SkillEntry) -> usize {
    estimate_tokens(&entry.name) + estimate_tokens(&entry.description)
}

/// Tokenize a string into a set of lowercased words.
///
/// Splits on whitespace, trims non-alphanumeric characters, lowercases,
/// and collects into a `HashSet`.
fn tokenize(s: &str) -> HashSet<String> {
    s.split_whitespace()
        .map(|w| {
            w.trim_matches(|c: char| !c.is_alphanumeric())
                .to_lowercase()
        })
        .filter(|w| !w.is_empty())
        .collect()
}

/// Compute Jaccard similarity between two pre-tokenized sets.
///
/// Returns 0.0 if both sets are empty.
fn jaccard_from_sets(a: &HashSet<String>, b: &HashSet<String>) -> f64 {
    let intersection = a.intersection(b).count();
    let union = a.union(b).count();
    if union == 0 {
        0.0
    } else {
        intersection as f64 / union as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Compute Jaccard similarity between two strings (test-only convenience).
    ///
    /// Wrapper around [`tokenize`] and [`jaccard_from_sets`].
    fn jaccard_similarity(a: &str, b: &str) -> f64 {
        let set_a = tokenize(a);
        let set_b = tokenize(b);
        jaccard_from_sets(&set_a, &set_b)
    }

    /// Create a SkillEntry for testing.
    fn make_entry(name: &str, description: &str) -> SkillEntry {
        SkillEntry {
            name: name.to_string(),
            description: description.to_string(),
            location: format!("skills/{name}"),
        }
    }

    // ── C001: Name collisions ────────────────────────────────────────

    #[test]
    fn c001_duplicate_names() {
        let entries = vec![
            make_entry("my-skill", "First skill"),
            make_entry("my-skill", "Second skill"),
        ];
        let diags = detect_conflicts(&entries);
        assert!(
            diags.iter().any(|d| d.code == C001),
            "expected C001 for duplicate names, got: {diags:?}",
        );
    }

    #[test]
    fn c001_unique_names_no_collision() {
        let entries = vec![
            make_entry("skill-a", "First skill"),
            make_entry("skill-b", "Second skill"),
        ];
        let diags = detect_conflicts(&entries);
        assert!(
            !diags.iter().any(|d| d.code == C001),
            "expected no C001 for unique names, got: {diags:?}",
        );
    }

    // ── C002: Description similarity ─────────────────────────────────

    #[test]
    fn c002_similar_descriptions() {
        let entries = vec![
            make_entry(
                "skill-a",
                "Processes PDF files and generates detailed reports",
            ),
            make_entry(
                "skill-b",
                "Processes PDF files and generates detailed summaries",
            ),
        ];
        let diags = detect_conflicts(&entries);
        assert!(
            diags.iter().any(|d| d.code == C002),
            "expected C002 for similar descriptions, got: {diags:?}",
        );
    }

    #[test]
    fn c002_distinct_descriptions_no_overlap() {
        let entries = vec![
            make_entry("skill-a", "Processes PDF files"),
            make_entry("skill-b", "Manages database connections"),
        ];
        let diags = detect_conflicts(&entries);
        assert!(
            !diags.iter().any(|d| d.code == C002),
            "expected no C002 for distinct descriptions, got: {diags:?}",
        );
    }

    #[test]
    fn c002_custom_threshold() {
        let entries = vec![
            make_entry("skill-a", "Processes files"),
            make_entry("skill-b", "Processes documents"),
        ];
        // With threshold 0.3, this should trigger (1 common word out of 3 unique = 0.33)
        let diags = detect_conflicts_with_threshold(&entries, 0.3);
        assert!(
            diags.iter().any(|d| d.code == C002),
            "expected C002 with low threshold, got: {diags:?}",
        );
    }

    // ── C003: Token budget ───────────────────────────────────────────

    #[test]
    fn c003_exceeds_budget() {
        // Each description ≈ 10000 chars → ~2500 tokens each → ~5000 total > 4000 threshold.
        let large_description = "word ".repeat(10000);
        let entries = vec![
            make_entry("skill-a", &large_description),
            make_entry("skill-b", &large_description),
        ];
        let diags = detect_conflicts(&entries);
        assert!(
            diags.iter().any(|d| d.code == C003),
            "expected C003 for large token budget, got: {diags:?}",
        );
    }

    #[test]
    fn c003_within_budget_no_warning() {
        let entries = vec![
            make_entry("skill-a", "A small skill"),
            make_entry("skill-b", "Another skill"),
        ];
        let diags = detect_conflicts(&entries);
        assert!(
            !diags.iter().any(|d| d.code == C003),
            "expected no C003 for small skills, got: {diags:?}",
        );
    }

    // ── Jaccard similarity ───────────────────────────────────────────

    #[test]
    fn jaccard_identical_strings() {
        let sim = jaccard_similarity("hello world", "hello world");
        assert!((sim - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn jaccard_completely_different() {
        let sim = jaccard_similarity("hello world", "foo bar");
        assert!(sim < f64::EPSILON);
    }

    #[test]
    fn jaccard_partial_overlap() {
        let sim = jaccard_similarity("hello world", "hello there");
        // intersection={hello}, union={hello, world, there} → 1/3 ≈ 0.33
        assert!((sim - 1.0 / 3.0).abs() < 0.01);
    }

    #[test]
    fn jaccard_case_insensitive() {
        let sim = jaccard_similarity("PDF Files", "pdf files");
        assert!(
            (sim - 1.0).abs() < f64::EPSILON,
            "expected 1.0 for case-only difference, got: {sim}",
        );
    }

    #[test]
    fn jaccard_empty_strings() {
        let sim = jaccard_similarity("", "");
        assert!(sim < f64::EPSILON);
    }

    // ── Diagnostic metadata ──────────────────────────────────────────

    #[test]
    fn all_conflict_diagnostics_are_warnings() {
        let entries = vec![
            make_entry("my-skill", "Processes PDF files"),
            make_entry("my-skill", "Processes PDF files"),
        ];
        let diags = detect_conflicts(&entries);
        assert!(
            diags.iter().all(|d| d.is_warning()),
            "all conflict diagnostics should be warnings: {diags:?}",
        );
    }

    #[test]
    fn diagnostics_have_fields_and_suggestions() {
        let entries = vec![
            make_entry("my-skill", "Same description"),
            make_entry("my-skill", "Same description"),
        ];
        let diags = detect_conflicts(&entries);
        assert!(
            diags.iter().all(|d| d.field.is_some()),
            "all diagnostics should have fields: {diags:?}",
        );
        assert!(
            diags.iter().all(|d| d.suggestion.is_some()),
            "all diagnostics should have suggestions: {diags:?}",
        );
    }

    // ── Empty collection ─────────────────────────────────────────────

    #[test]
    fn empty_collection_no_conflicts() {
        let diags = detect_conflicts(&[]);
        assert!(diags.is_empty());
    }

    #[test]
    fn single_entry_no_conflicts() {
        let entries = vec![make_entry("my-skill", "A skill")];
        let diags = detect_conflicts(&entries);
        assert!(diags.is_empty());
    }

    // ── Tokenize ────────────────────────────────────────────────────────

    #[test]
    fn tokenize_produces_expected_word_set() {
        let tokens = tokenize("Hello World hello");
        let expected: HashSet<String> = ["hello", "world"].iter().map(|s| s.to_string()).collect();
        assert_eq!(tokens, expected);
    }

    #[test]
    fn tokenize_empty_string_returns_empty_set() {
        let tokens = tokenize("");
        assert!(tokens.is_empty());
    }

    // ── jaccard_from_sets ───────────────────────────────────────────────

    #[test]
    fn jaccard_from_sets_matches_jaccard_similarity() {
        let pairs = vec![
            ("hello world", "hello world"),
            ("hello world", "foo bar"),
            ("hello world", "hello there"),
            ("PDF Files", "pdf files"),
            ("", ""),
        ];
        for (a, b) in pairs {
            let from_str = jaccard_similarity(a, b);
            let set_a = tokenize(a);
            let set_b = tokenize(b);
            let from_sets = jaccard_from_sets(&set_a, &set_b);
            assert!(
                (from_str - from_sets).abs() < f64::EPSILON,
                "mismatch for ({a:?}, {b:?}): jaccard_similarity={from_str}, jaccard_from_sets={from_sets}",
            );
        }
    }

    #[test]
    fn jaccard_from_sets_two_empty_sets_returns_zero() {
        let empty: HashSet<String> = HashSet::new();
        assert!(jaccard_from_sets(&empty, &empty) < f64::EPSILON);
    }
}
