//! Semantic lint checks for SKILL.md quality improvement.
//!
//! Lint checks produce `Severity::Info` diagnostics — they never cause
//! validation failure. They detect patterns that deviate from Anthropic
//! best practices for agent skill definitions.

use std::sync::LazyLock;

use regex::Regex;

use crate::diagnostics::{Diagnostic, Severity};
use crate::models::SkillProperties;

// ── Info code constants ────────────────────────────────────────────────

/// Description uses first or second person.
pub const I001: &str = "I001";
/// Description lacks a trigger phrase ("Use when…").
pub const I002: &str = "I002";
/// Name does not use gerund form.
pub const I003: &str = "I003";
/// Name is overly generic.
pub const I004: &str = "I004";
/// Description is overly vague.
pub const I005: &str = "I005";

/// Generic name segments that indicate a non-descriptive skill name.
const GENERIC_SEGMENTS: &[&str] = &[
    "helper", "utils", "tools", "stuff", "thing", "misc", "general",
];

/// Trigger phrases that indicate when a skill should be used.
///
/// Shared across linter, tester, and upgrade modules to ensure consistent
/// trigger phrase detection.
pub const TRIGGER_PHRASES: &[&str] = &[
    "use when",
    "use for",
    "use this",
    "invoke when",
    "activate when",
];

/// Regex matching first/second person pronouns at word boundaries.
static PERSON_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(I|me|my|you|your)\b").expect("person pronoun regex must compile")
});

/// Run all semantic lint checks on parsed skill properties and body.
///
/// Returns a list of `Severity::Info` diagnostics. These never cause
/// validation failure — they are suggestions for improving skill quality.
#[must_use]
pub fn lint(properties: &SkillProperties, _body: &str) -> Vec<Diagnostic> {
    let mut diags = Vec::new();
    diags.extend(lint_description_person(&properties.description));
    diags.extend(lint_description_trigger(&properties.description));
    diags.extend(lint_name_gerund(&properties.name));
    diags.extend(lint_name_generic(&properties.name));
    diags.extend(lint_description_vague(&properties.description));
    diags
}

/// I001: Check if description uses first or second person.
///
/// Descriptions should be written in third person (e.g., "Processes PDFs"
/// not "I process PDFs" or "You can process PDFs").
fn lint_description_person(description: &str) -> Vec<Diagnostic> {
    if PERSON_RE.is_match(description) {
        vec![
            Diagnostic::new(Severity::Info, I001, "description uses first/second person")
                .with_field("description")
                .with_suggestion(
                    "Rewrite in third person — e.g., 'Processes PDFs' not 'I process PDFs'",
                ),
        ]
    } else {
        vec![]
    }
}

/// I002: Check if description contains a trigger phrase.
///
/// Good descriptions include guidance on when to activate the skill,
/// such as "Use when working with PDF files."
fn lint_description_trigger(description: &str) -> Vec<Diagnostic> {
    let lower = description.to_lowercase();
    let has_trigger = TRIGGER_PHRASES.iter().any(|p| lower.contains(p));
    if has_trigger {
        vec![]
    } else {
        vec![
            Diagnostic::new(Severity::Info, I002, "description lacks trigger phrase")
                .with_field("description")
                .with_suggestion("Add a trigger phrase — e.g., 'Use when working with PDF files.'"),
        ]
    }
}

/// I003: Check if the skill name uses gerund form.
///
/// Names like "processing-pdfs" are preferred over "pdf-processor" because
/// they describe what the skill does in active voice.
fn lint_name_gerund(name: &str) -> Vec<Diagnostic> {
    let first_segment = name.split('-').next().unwrap_or("");
    if first_segment.ends_with("ing") {
        vec![]
    } else {
        vec![
            Diagnostic::new(Severity::Info, I003, "name does not use gerund form")
                .with_field("name")
                .with_suggestion(
                    "Consider gerund form — e.g., 'processing-pdfs' instead of 'pdf-processor'",
                ),
        ]
    }
}

/// I004: Check if the skill name is overly generic.
///
/// Names like "helper", "utils", or "tools" don't describe what the skill
/// actually does.
fn lint_name_generic(name: &str) -> Vec<Diagnostic> {
    let first_segment = name.split('-').next().unwrap_or("");
    if GENERIC_SEGMENTS.contains(&first_segment) {
        vec![Diagnostic::new(
            Severity::Info,
            I004,
            format!("name is overly generic: '{first_segment}'"),
        )
        .with_field("name")
        .with_suggestion("Use a specific, descriptive name")]
    } else {
        vec![]
    }
}

/// I005: Check if the description is overly vague.
///
/// Descriptions should provide enough detail to understand what the skill
/// does. Very short descriptions (< 20 chars or < 4 words) are flagged.
fn lint_description_vague(description: &str) -> Vec<Diagnostic> {
    let word_count = description.split_whitespace().count();
    if description.chars().count() < 20 || word_count < 4 {
        vec![
            Diagnostic::new(Severity::Info, I005, "description is overly vague")
                .with_field("description")
                .with_suggestion("Add detail about what the skill does and when to use it"),
        ]
    } else {
        vec![]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: create a SkillProperties with the given name and description.
    fn make_props(name: &str, description: &str) -> SkillProperties {
        SkillProperties {
            name: name.to_string(),
            description: description.to_string(),
            license: None,
            compatibility: None,
            allowed_tools: None,
            metadata: None,
        }
    }

    // ── I001: First/second person ──────────────────────────────────────

    #[test]
    fn i001_first_person_triggers() {
        let diags = lint_description_person("I can help you process files");
        assert!(
            diags.iter().any(|d| d.code == I001),
            "expected I001, got: {diags:?}"
        );
    }

    #[test]
    fn i001_third_person_no_trigger() {
        let diags = lint_description_person("Processes files and generates reports");
        assert!(diags.is_empty(), "expected no I001, got: {diags:?}");
    }

    #[test]
    fn i001_your_triggers() {
        let diags = lint_description_person("Helps with your PDF files");
        assert!(
            diags.iter().any(|d| d.code == I001),
            "expected I001 for 'your', got: {diags:?}"
        );
    }

    #[test]
    fn i001_case_insensitive() {
        let diags = lint_description_person("MY FILES are processed");
        assert!(
            diags.iter().any(|d| d.code == I001),
            "expected I001 case-insensitive, got: {diags:?}"
        );
    }

    #[test]
    fn i001_pronoun_in_word_no_trigger() {
        // "mine" contains "I" but not at word boundary for "I"
        // "myself" starts with "my" — regex checks word boundary
        let diags = lint_description_person("Processes files automatically");
        assert!(diags.is_empty(), "expected no I001, got: {diags:?}");
    }

    // ── I002: Missing trigger phrase ───────────────────────────────────

    #[test]
    fn i002_no_trigger_phrase() {
        let diags = lint_description_trigger("Processes files");
        assert!(
            diags.iter().any(|d| d.code == I002),
            "expected I002, got: {diags:?}"
        );
    }

    #[test]
    fn i002_has_trigger_phrase() {
        let diags = lint_description_trigger("Processes files. Use when working with data.");
        assert!(diags.is_empty(), "expected no I002, got: {diags:?}");
    }

    #[test]
    fn i002_trigger_case_insensitive() {
        let diags = lint_description_trigger("Processes files. USE WHEN needed.");
        assert!(diags.is_empty(), "expected no I002, got: {diags:?}");
    }

    #[test]
    fn i002_use_for_trigger() {
        let diags = lint_description_trigger("Use for processing large datasets");
        assert!(diags.is_empty(), "expected no I002, got: {diags:?}");
    }

    // ── I003: Non-gerund name ──────────────────────────────────────────

    #[test]
    fn i003_non_gerund_triggers() {
        let diags = lint_name_gerund("pdf-processor");
        assert!(
            diags.iter().any(|d| d.code == I003),
            "expected I003, got: {diags:?}"
        );
    }

    #[test]
    fn i003_gerund_no_trigger() {
        let diags = lint_name_gerund("processing-pdfs");
        assert!(diags.is_empty(), "expected no I003, got: {diags:?}");
    }

    #[test]
    fn i003_single_gerund_word() {
        let diags = lint_name_gerund("linting");
        assert!(diags.is_empty(), "expected no I003, got: {diags:?}");
    }

    // ── I004: Generic name ─────────────────────────────────────────────

    #[test]
    fn i004_generic_name_triggers() {
        let diags = lint_name_generic("helper");
        assert!(
            diags.iter().any(|d| d.code == I004),
            "expected I004, got: {diags:?}"
        );
    }

    #[test]
    fn i004_specific_name_no_trigger() {
        let diags = lint_name_generic("processing-pdfs");
        assert!(diags.is_empty(), "expected no I004, got: {diags:?}");
    }

    #[test]
    fn i004_utils_triggers() {
        let diags = lint_name_generic("utils-collection");
        assert!(
            diags.iter().any(|d| d.code == I004),
            "expected I004 for 'utils', got: {diags:?}"
        );
    }

    #[test]
    fn i004_generic_as_non_first_segment_no_trigger() {
        // "helper" as second segment should not trigger
        let diags = lint_name_generic("pdf-helper");
        assert!(
            diags.is_empty(),
            "expected no I004 for non-first segment, got: {diags:?}"
        );
    }

    // ── I005: Vague description ────────────────────────────────────────

    #[test]
    fn i005_too_short_triggers() {
        let diags = lint_description_vague("Helps");
        assert!(
            diags.iter().any(|d| d.code == I005),
            "expected I005, got: {diags:?}"
        );
    }

    #[test]
    fn i005_detailed_no_trigger() {
        let diags = lint_description_vague(
            "Processes PDF files and generates detailed reports for analysis",
        );
        assert!(diags.is_empty(), "expected no I005, got: {diags:?}");
    }

    #[test]
    fn i005_few_words_triggers() {
        let diags = lint_description_vague("Does some stuff");
        assert!(
            diags.iter().any(|d| d.code == I005),
            "expected I005 for few words, got: {diags:?}"
        );
    }

    #[test]
    fn i005_exactly_4_words_20_chars_no_trigger() {
        // "word word word words" is 20 chars and 4 words
        let diags = lint_description_vague("word word word words");
        assert!(diags.is_empty(), "expected no I005, got: {diags:?}");
    }

    // ── Full lint pipeline ─────────────────────────────────────────────

    #[test]
    fn lint_all_checks_severity_info() {
        let props = make_props("helper", "Helps");
        let diags = lint(&props, "");
        assert!(
            diags.iter().all(|d| d.is_info()),
            "all lint diagnostics should be Info: {diags:?}"
        );
    }

    #[test]
    fn lint_perfect_skill_no_diagnostics() {
        let props = make_props(
            "processing-pdfs",
            "Processes PDF files and generates reports. Use when working with documents.",
        );
        let diags = lint(&props, "");
        assert!(
            diags.is_empty(),
            "perfect skill should have no lint issues: {diags:?}"
        );
    }

    #[test]
    fn lint_multiple_issues_collected() {
        let props = make_props("helper", "I help");
        let diags = lint(&props, "");
        // Should trigger: I001 (person), I002 (trigger), I003 (gerund), I004 (generic), I005 (vague)
        let codes: Vec<_> = diags.iter().map(|d| d.code).collect();
        assert!(codes.contains(&I001), "expected I001 in {codes:?}");
        assert!(codes.contains(&I002), "expected I002 in {codes:?}");
        assert!(codes.contains(&I003), "expected I003 in {codes:?}");
        assert!(codes.contains(&I004), "expected I004 in {codes:?}");
        assert!(codes.contains(&I005), "expected I005 in {codes:?}");
    }

    #[test]
    fn lint_diagnostics_have_fields() {
        let props = make_props("helper", "I help");
        let diags = lint(&props, "");
        assert!(
            diags.iter().all(|d| d.field.is_some()),
            "all lint diagnostics should have field set: {diags:?}"
        );
    }

    #[test]
    fn lint_diagnostics_have_suggestions() {
        let props = make_props("helper", "I help");
        let diags = lint(&props, "");
        assert!(
            diags.iter().all(|d| d.suggestion.is_some()),
            "all lint diagnostics should have suggestions: {diags:?}"
        );
    }

    #[test]
    fn lint_codes_are_unique() {
        let codes = [I001, I002, I003, I004, I005];
        let mut seen = std::collections::HashSet::new();
        for code in &codes {
            assert!(seen.insert(code), "duplicate lint code: {code}");
        }
    }
}
