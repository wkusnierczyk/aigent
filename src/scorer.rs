//! Quality scoring for SKILL.md files.
//!
//! Combines structural validation and semantic lint checks into a
//! single 0–100 quality score with detailed breakdown.
//!
//! Scoring weights:
//! - Structural checks (validation): 60 points base
//! - Quality checks (lint): 8 points per check (5 checks × 8 = 40 max)
//!
//! A perfect skill with no validation errors and no lint issues scores 100.

use std::path::Path;

use serde::Serialize;

use crate::diagnostics::Diagnostic;
use crate::linter;
use crate::validator;

/// Maximum points for structural (validation) checks.
const STRUCTURAL_MAX: u32 = 60;

/// Points per passing lint check.
const LINT_POINTS_PER_CHECK: u32 = 8;

/// Total number of lint checks.
const LINT_CHECK_COUNT: u32 = 5;

/// Maximum points for quality (lint) checks.
const QUALITY_MAX: u32 = LINT_POINTS_PER_CHECK * LINT_CHECK_COUNT;

/// Result of scoring a skill directory.
#[derive(Debug, Clone, Serialize)]
pub struct ScoreResult {
    /// Overall quality score (0–100).
    pub total: u32,
    /// Maximum possible score (always 100).
    pub max: u32,
    /// Structural check breakdown.
    pub structural: CategoryResult,
    /// Quality (lint) check breakdown.
    pub quality: CategoryResult,
}

/// Breakdown for a scoring category (structural or quality).
#[derive(Debug, Clone, Serialize)]
pub struct CategoryResult {
    /// Points earned in this category.
    pub score: u32,
    /// Maximum possible points for this category.
    pub max: u32,
    /// Individual check results within this category.
    pub checks: Vec<CheckResult>,
}

/// Result of a single check within a category.
#[derive(Debug, Clone, Serialize)]
pub struct CheckResult {
    /// Human-readable label for this check (shown when the check passes).
    pub label: String,
    /// Label shown when the check fails (if different from the pass label).
    ///
    /// When `None`, the pass label is used for both states. This avoids
    /// confusing output like `[FAIL] No unknown fields` — instead, the
    /// fail label reads `Unknown fields found`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fail_label: Option<String>,
    /// Whether this check passed.
    pub passed: bool,
    /// Diagnostic message if the check failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl CheckResult {
    /// Returns the appropriate label for the current state.
    ///
    /// Uses `fail_label` when the check has failed and a fail-specific label
    /// is defined; otherwise falls back to the standard `label`.
    #[must_use]
    pub fn display_label(&self) -> &str {
        if self.passed {
            &self.label
        } else {
            self.fail_label.as_deref().unwrap_or(&self.label)
        }
    }
}

/// Score a skill directory against Anthropic best-practices.
///
/// Runs both structural validation and semantic lint checks, then
/// combines the results into a weighted 0–100 score.
///
/// # Arguments
///
/// * `dir` - Path to the skill directory containing SKILL.md
///
/// # Returns
///
/// A `ScoreResult` with total score and detailed breakdown.
#[must_use]
pub fn score(dir: &Path) -> ScoreResult {
    let validation_diags = validator::validate(dir);
    let structural = score_structural(&validation_diags);

    // Only run lint checks if the skill is parseable (no infrastructure errors).
    let quality = match crate::parser::read_properties(dir) {
        Ok(props) => {
            let body = read_body(dir);
            let lint_diags = linter::lint(&props, &body);
            score_quality(&lint_diags)
        }
        Err(_) => {
            // If we can't parse the skill, all lint checks fail.
            all_quality_checks_failed()
        }
    };

    let total = structural.score + quality.score;
    ScoreResult {
        total,
        max: STRUCTURAL_MAX + QUALITY_MAX,
        structural,
        quality,
    }
}

/// Score the structural (validation) category.
///
/// Full 60 points if no errors; 0 if any error or warning present.
/// Individual checks are derived from validation diagnostic codes.
fn score_structural(diags: &[Diagnostic]) -> CategoryResult {
    let has_errors = diags.iter().any(|d| d.is_error());
    let has_warnings = diags.iter().any(|d| d.is_warning());

    let checks = vec![
        CheckResult {
            label: "SKILL.md exists and is parseable".to_string(),
            fail_label: Some("SKILL.md missing or unparseable".to_string()),
            passed: !diags.iter().any(|d| d.code == "E000"),
            message: diags
                .iter()
                .find(|d| d.code == "E000")
                .map(|d| d.message.clone()),
        },
        CheckResult {
            label: "Name format valid".to_string(),
            fail_label: Some("Name format invalid".to_string()),
            passed: !diags.iter().any(|d| {
                matches!(
                    d.code,
                    "E001" | "E002" | "E003" | "E004" | "E005" | "E006" | "E007" | "E008" | "E009"
                )
            }),
            message: diags
                .iter()
                .find(|d| {
                    matches!(
                        d.code,
                        "E001"
                            | "E002"
                            | "E003"
                            | "E004"
                            | "E005"
                            | "E006"
                            | "E007"
                            | "E008"
                            | "E009"
                    )
                })
                .map(|d| d.message.clone()),
        },
        CheckResult {
            label: "Description valid".to_string(),
            fail_label: Some("Description invalid".to_string()),
            passed: !diags
                .iter()
                .any(|d| matches!(d.code, "E010" | "E011" | "E012")),
            message: diags
                .iter()
                .find(|d| matches!(d.code, "E010" | "E011" | "E012"))
                .map(|d| d.message.clone()),
        },
        CheckResult {
            label: "Required fields present".to_string(),
            fail_label: Some("Required fields missing".to_string()),
            passed: !diags
                .iter()
                .any(|d| matches!(d.code, "E014" | "E015" | "E016" | "E017" | "E018")),
            message: diags
                .iter()
                .find(|d| matches!(d.code, "E014" | "E015" | "E016" | "E017" | "E018"))
                .map(|d| d.message.clone()),
        },
        CheckResult {
            label: "No unknown fields".to_string(),
            fail_label: Some("Unknown fields found".to_string()),
            passed: !has_warnings,
            message: diags
                .iter()
                .find(|d| d.is_warning())
                .map(|d| d.message.clone()),
        },
        CheckResult {
            label: "Body within size limits".to_string(),
            fail_label: Some("Body exceeds size limits".to_string()),
            passed: !diags.iter().any(|d| d.code == "W002"),
            message: diags
                .iter()
                .find(|d| d.code == "W002")
                .map(|d| d.message.clone()),
        },
    ];

    let points = if has_errors || has_warnings {
        0
    } else {
        STRUCTURAL_MAX
    };

    CategoryResult {
        score: points,
        max: STRUCTURAL_MAX,
        checks,
    }
}

/// Score the quality (lint) category.
///
/// 8 points per passing lint check (5 checks × 8 = 40 max).
fn score_quality(lint_diags: &[Diagnostic]) -> CategoryResult {
    let checks = vec![
        CheckResult {
            label: "Third-person description".to_string(),
            fail_label: Some("Not third-person description".to_string()),
            passed: !lint_diags.iter().any(|d| d.code == linter::I001),
            message: lint_diags
                .iter()
                .find(|d| d.code == linter::I001)
                .map(|d| d.message.clone()),
        },
        CheckResult {
            label: "Trigger phrase present".to_string(),
            fail_label: Some("Trigger phrase missing".to_string()),
            passed: !lint_diags.iter().any(|d| d.code == linter::I002),
            message: lint_diags
                .iter()
                .find(|d| d.code == linter::I002)
                .map(|d| d.message.clone()),
        },
        CheckResult {
            label: "Gerund name form".to_string(),
            fail_label: Some("Non-gerund name form".to_string()),
            passed: !lint_diags.iter().any(|d| d.code == linter::I003),
            message: lint_diags
                .iter()
                .find(|d| d.code == linter::I003)
                .map(|d| d.message.clone()),
        },
        CheckResult {
            label: "Specific name".to_string(),
            fail_label: Some("Generic name".to_string()),
            passed: !lint_diags.iter().any(|d| d.code == linter::I004),
            message: lint_diags
                .iter()
                .find(|d| d.code == linter::I004)
                .map(|d| d.message.clone()),
        },
        CheckResult {
            label: "Detailed description".to_string(),
            fail_label: Some("Description too short".to_string()),
            passed: !lint_diags.iter().any(|d| d.code == linter::I005),
            message: lint_diags
                .iter()
                .find(|d| d.code == linter::I005)
                .map(|d| d.message.clone()),
        },
    ];

    let passing = checks.iter().filter(|c| c.passed).count() as u32;
    let points = passing * LINT_POINTS_PER_CHECK;

    CategoryResult {
        score: points,
        max: QUALITY_MAX,
        checks,
    }
}

/// All quality checks failed (used when skill can't be parsed).
fn all_quality_checks_failed() -> CategoryResult {
    CategoryResult {
        score: 0,
        max: QUALITY_MAX,
        checks: vec![
            CheckResult {
                label: "Third-person description".to_string(),
                fail_label: Some("Not third-person description".to_string()),
                passed: false,
                message: Some("Skill could not be parsed".to_string()),
            },
            CheckResult {
                label: "Trigger phrase present".to_string(),
                fail_label: Some("Trigger phrase missing".to_string()),
                passed: false,
                message: Some("Skill could not be parsed".to_string()),
            },
            CheckResult {
                label: "Gerund name form".to_string(),
                fail_label: Some("Non-gerund name form".to_string()),
                passed: false,
                message: Some("Skill could not be parsed".to_string()),
            },
            CheckResult {
                label: "Specific name".to_string(),
                fail_label: Some("Generic name".to_string()),
                passed: false,
                message: Some("Skill could not be parsed".to_string()),
            },
            CheckResult {
                label: "Detailed description".to_string(),
                fail_label: Some("Description too short".to_string()),
                passed: false,
                message: Some("Skill could not be parsed".to_string()),
            },
        ],
    }
}

/// Read the SKILL.md body (post-frontmatter) for lint scoring.
fn read_body(dir: &Path) -> String {
    let path = match crate::parser::find_skill_md(dir) {
        Some(p) => p,
        None => return String::new(),
    };
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return String::new(),
    };
    match crate::parser::parse_frontmatter(&content) {
        Ok((_, body)) => body,
        Err(_) => String::new(),
    }
}

/// Format a `ScoreResult` as human-readable text.
#[must_use]
pub fn format_text(result: &ScoreResult) -> String {
    let mut out = String::new();

    out.push_str(&format!("Score: {}/{}\n", result.total, result.max));

    out.push_str(&format!(
        "\nStructural ({}/{}):\n",
        result.structural.score, result.structural.max
    ));
    for check in &result.structural.checks {
        let status = if check.passed { "PASS" } else { "FAIL" };
        let display_label = check.display_label();
        out.push_str(&format!("  [{status}] {display_label}\n"));
        if let Some(msg) = &check.message {
            out.push_str(&format!("         {msg}\n"));
        }
    }

    out.push_str(&format!(
        "\nQuality ({}/{}):\n",
        result.quality.score, result.quality.max
    ));
    for check in &result.quality.checks {
        let status = if check.passed { "PASS" } else { "FAIL" };
        let display_label = check.display_label();
        out.push_str(&format!("  [{status}] {display_label}\n"));
        if let Some(msg) = &check.message {
            out.push_str(&format!("         {msg}\n"));
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    /// Create a skill directory with given frontmatter content.
    fn make_skill(name: &str, frontmatter: &str) -> (tempfile::TempDir, std::path::PathBuf) {
        let parent = tempdir().unwrap();
        let dir = parent.path().join(name);
        fs::create_dir(&dir).unwrap();
        fs::write(dir.join("SKILL.md"), frontmatter).unwrap();
        (parent, dir)
    }

    // ── Score constants ──────────────────────────────────────────────

    #[test]
    fn max_score_is_100() {
        assert_eq!(STRUCTURAL_MAX + QUALITY_MAX, 100);
    }

    #[test]
    fn quality_max_is_5_times_8() {
        assert_eq!(QUALITY_MAX, 40);
    }

    // ── Perfect skill ────────────────────────────────────────────────

    #[test]
    fn perfect_skill_scores_100() {
        let (_parent, dir) = make_skill(
            "processing-pdfs",
            "---\nname: processing-pdfs\ndescription: >-\n  Processes PDF files and generates detailed reports for analysis.\n  Use when working with document conversion tasks.\n---\n# Body\n",
        );
        let result = score(&dir);
        assert_eq!(
            result.total,
            100,
            "perfect skill should score 100, got {}. structural={}/{}, quality={}/{}",
            result.total,
            result.structural.score,
            result.structural.max,
            result.quality.score,
            result.quality.max,
        );
    }

    // ── Lint issues reduce quality score ──────────────────────────────

    #[test]
    fn lint_issues_reduce_quality_score() {
        // "pdf-processor" is not gerund, no trigger phrase → loses I002 + I003 = -16
        let (_parent, dir) = make_skill(
            "pdf-processor",
            "---\nname: pdf-processor\ndescription: Processes PDF files and generates detailed reports for analysis\n---\n",
        );
        let result = score(&dir);
        assert!(
            result.total < 100,
            "skill with lint issues should score < 100, got {}",
            result.total,
        );
        // Structural should be perfect
        assert_eq!(result.structural.score, 60);
        // Quality should be < 40
        assert!(
            result.quality.score < 40,
            "quality should be < 40, got {}",
            result.quality.score,
        );
    }

    // ── Structural errors ────────────────────────────────────────────

    #[test]
    fn structural_errors_score_at_most_quality_max() {
        // Missing SKILL.md entirely → structural = 0, quality = 0
        let parent = tempdir().unwrap();
        let result = score(parent.path());
        assert_eq!(
            result.structural.score, 0,
            "missing SKILL.md should score structural 0"
        );
        assert!(
            result.total <= QUALITY_MAX,
            "structural errors should cap total to at most quality max ({}), got {}",
            QUALITY_MAX,
            result.total,
        );
    }

    #[test]
    fn structural_error_sets_structural_to_zero() {
        // Empty name causes E001
        let (_parent, dir) = make_skill(
            "bad-skill",
            "---\nname: \"\"\ndescription: A valid description for scoring tests here\n---\n",
        );
        let result = score(&dir);
        assert_eq!(
            result.structural.score, 0,
            "validation errors should zero structural score"
        );
    }

    // ── Unparseable skill ────────────────────────────────────────────

    #[test]
    fn unparseable_skill_scores_zero() {
        let (_parent, dir) = make_skill("broken", "this is not valid frontmatter");
        let result = score(&dir);
        assert_eq!(result.total, 0, "unparseable skill should score 0");
        assert_eq!(result.structural.score, 0);
        assert_eq!(result.quality.score, 0);
    }

    // ── Check details ────────────────────────────────────────────────

    #[test]
    fn perfect_skill_all_checks_pass() {
        let (_parent, dir) = make_skill(
            "processing-pdfs",
            "---\nname: processing-pdfs\ndescription: >-\n  Processes PDF files and generates detailed reports.\n  Use when working with documents.\n---\n",
        );
        let result = score(&dir);
        assert!(
            result.structural.checks.iter().all(|c| c.passed),
            "all structural checks should pass: {:?}",
            result.structural.checks,
        );
        assert!(
            result.quality.checks.iter().all(|c| c.passed),
            "all quality checks should pass: {:?}",
            result.quality.checks,
        );
    }

    #[test]
    fn failed_checks_have_messages() {
        let (_parent, dir) = make_skill("helper", "---\nname: helper\ndescription: Helps\n---\n");
        let result = score(&dir);
        let failed: Vec<_> = result.quality.checks.iter().filter(|c| !c.passed).collect();
        assert!(
            failed.iter().all(|c| c.message.is_some()),
            "failed checks should have messages: {failed:?}",
        );
    }

    // ── Format text ──────────────────────────────────────────────────

    #[test]
    fn format_text_includes_score_line() {
        let (_parent, dir) = make_skill(
            "processing-pdfs",
            "---\nname: processing-pdfs\ndescription: >-\n  Processes PDF files and generates detailed reports.\n  Use when working with documents.\n---\n",
        );
        let result = score(&dir);
        let text = format_text(&result);
        assert!(
            text.contains("Score: 100/100"),
            "text should contain score line: {text}",
        );
    }

    #[test]
    fn format_text_shows_pass_fail() {
        let (_parent, dir) = make_skill("helper", "---\nname: helper\ndescription: Helps\n---\n");
        let result = score(&dir);
        let text = format_text(&result);
        assert!(text.contains("[PASS]"), "text should contain [PASS]");
        assert!(text.contains("[FAIL]"), "text should contain [FAIL]");
    }

    // ── JSON serialization ───────────────────────────────────────────

    #[test]
    fn score_result_serializes_to_json() {
        let (_parent, dir) = make_skill(
            "processing-pdfs",
            "---\nname: processing-pdfs\ndescription: >-\n  Processes PDF files and generates detailed reports.\n  Use when working with documents.\n---\n",
        );
        let result = score(&dir);
        let json = serde_json::to_value(&result).unwrap();
        assert_eq!(json["total"], 100);
        assert_eq!(json["max"], 100);
        assert!(json["structural"]["checks"].is_array());
        assert!(json["quality"]["checks"].is_array());
    }

    #[test]
    fn json_omits_message_when_passed() {
        let (_parent, dir) = make_skill(
            "processing-pdfs",
            "---\nname: processing-pdfs\ndescription: >-\n  Processes PDF files and generates detailed reports.\n  Use when working with documents.\n---\n",
        );
        let result = score(&dir);
        let json = serde_json::to_value(&result).unwrap();
        let checks = json["quality"]["checks"].as_array().unwrap();
        for check in checks {
            assert!(
                check.get("message").is_none(),
                "passing check should not have message: {check}",
            );
        }
    }

    // ── Scoring granularity ──────────────────────────────────────────

    #[test]
    fn each_lint_failure_costs_8_points() {
        // "processing-pdfs" with only I002 missing (no trigger phrase)
        let (_parent, dir) = make_skill(
            "processing-pdfs",
            "---\nname: processing-pdfs\ndescription: Processes PDF files and generates detailed reports for analysis\n---\n",
        );
        let result = score(&dir);
        // Should lose exactly I002 (no trigger) → quality = 32
        let expected_quality = QUALITY_MAX - LINT_POINTS_PER_CHECK; // 40 - 8 = 32
        assert_eq!(
            result.quality.score, expected_quality,
            "missing one lint check should cost {} points, got quality={}",
            LINT_POINTS_PER_CHECK, result.quality.score,
        );
    }
}
