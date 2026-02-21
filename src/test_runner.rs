//! Fixture-based skill testing: run test suites defined in `tests.yml`.
//!
//! Each skill can include a `tests.yml` file with activation queries and expected
//! outcomes. The test runner evaluates each query using the probe infrastructure
//! and compares against expectations.

use std::path::Path;

use crate::errors::{AigentError, Result};
use crate::tester;

/// Result of running a full test suite.
#[derive(Debug, serde::Serialize)]
pub struct TestSuiteResult {
    /// Number of passing test cases.
    pub passed: usize,
    /// Number of failing test cases.
    pub failed: usize,
    /// Individual test case results.
    pub results: Vec<TestCaseResult>,
}

/// Result of a single test case.
#[derive(Debug, serde::Serialize)]
pub struct TestCaseResult {
    /// The input query.
    pub input: String,
    /// Whether a match was expected.
    pub should_match: bool,
    /// Whether the probe actually matched.
    pub actual_match: bool,
    /// The score from the probe.
    pub score: f64,
    /// Whether the test case passed.
    pub passed: bool,
    /// Optional failure reason.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// A test fixture parsed from tests.yml.
#[derive(Debug, serde::Deserialize)]
struct TestFixture {
    /// The list of test queries.
    queries: Vec<TestQuery>,
}

/// A single test query from the fixture.
#[derive(Debug, serde::Deserialize)]
struct TestQuery {
    /// The query string.
    input: String,
    /// Whether the query should activate the skill.
    should_match: bool,
    /// Optional minimum score threshold.
    #[serde(default)]
    min_score: Option<f64>,
}

/// Run a test suite for a skill directory.
///
/// Reads `tests.yml` from the skill directory and runs each query through
/// the probe infrastructure, comparing results against expectations.
///
/// # Errors
///
/// Returns an error if `tests.yml` cannot be found or parsed, or if the
/// skill directory is invalid.
pub fn run_test_suite(skill_dir: &Path) -> Result<TestSuiteResult> {
    let fixture_path = skill_dir.join("tests.yml");
    if !fixture_path.exists() {
        return Err(AigentError::Parse {
            message: format!(
                "no tests.yml found in {}. Use --generate to create one.",
                skill_dir.display()
            ),
        });
    }

    let content = std::fs::read_to_string(&fixture_path)?;
    let fixture: TestFixture =
        serde_yaml_ng::from_str(&content).map_err(|e| AigentError::Parse {
            message: format!("invalid tests.yml: {e}"),
        })?;

    let mut results = Vec::new();
    let mut passed = 0;
    let mut failed = 0;

    for query in &fixture.queries {
        let probe_result = tester::test_skill(skill_dir, &query.input)?;

        let actual_match = !matches!(probe_result.query_match, tester::QueryMatch::None);
        let score = probe_result.score;

        let mut case_passed = actual_match == query.should_match;
        let mut reason = None;

        // Check min_score constraint if specified and the match was expected.
        if case_passed && query.should_match {
            if let Some(min) = query.min_score {
                if score < min {
                    case_passed = false;
                    reason = Some(format!("score {score:.2} below minimum {min:.2}"));
                }
            }
        }

        if !case_passed && reason.is_none() {
            reason = Some(format!(
                "expected {} match, got {}",
                if query.should_match { "a" } else { "no" },
                if actual_match { "match" } else { "no match" },
            ));
        }

        if case_passed {
            passed += 1;
        } else {
            failed += 1;
        }

        results.push(TestCaseResult {
            input: query.input.clone(),
            should_match: query.should_match,
            actual_match,
            score,
            passed: case_passed,
            reason,
        });
    }

    Ok(TestSuiteResult {
        passed,
        failed,
        results,
    })
}

/// Generate a starter tests.yml from skill metadata.
///
/// Creates a basic fixture with a positive query derived from the skill
/// description and a negative query.
pub fn generate_fixture(skill_dir: &Path) -> Result<String> {
    let props = crate::read_properties(skill_dir)?;

    // Create a positive query from the description (first sentence).
    let positive = props
        .description
        .split('.')
        .next()
        .unwrap_or(&props.description)
        .trim()
        .to_lowercase();

    Ok(format!(
        r#"# Test fixture for {name}
# Run with: aigent test {name}/
queries:
  - input: "{positive}"
    should_match: true
    min_score: 0.3
  - input: "something completely unrelated to this skill"
    should_match: false
"#,
        name = props.name,
        positive = positive,
    ))
}

/// Format test suite results as human-readable text.
#[must_use]
pub fn format_text(result: &TestSuiteResult) -> String {
    let mut out = String::new();

    for case in &result.results {
        let status = if case.passed { "PASS" } else { "FAIL" };
        out.push_str(&format!(
            "[{status}] \"{input}\" (score: {score:.2})\n",
            input = case.input,
            score = case.score,
        ));
        if let Some(reason) = &case.reason {
            out.push_str(&format!("      → {reason}\n"));
        }
    }

    out.push_str(&format!(
        "\n{passed} passed, {failed} failed, {total} total\n",
        passed = result.passed,
        failed = result.failed,
        total = result.passed + result.failed,
    ));

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    fn make_skill_with_tests(
        name: &str,
        skill_content: &str,
        tests_content: &str,
    ) -> (tempfile::TempDir, std::path::PathBuf) {
        let parent = tempdir().unwrap();
        let dir = parent.path().join(name);
        fs::create_dir(&dir).unwrap();
        fs::write(dir.join("SKILL.md"), skill_content).unwrap();
        fs::write(dir.join("tests.yml"), tests_content).unwrap();
        (parent, dir)
    }

    #[test]
    fn run_suite_all_pass() {
        let (_parent, dir) = make_skill_with_tests(
            "my-skill",
            "---\nname: my-skill\ndescription: Processes PDF files and generates reports. Use when working with documents.\n---\nBody.\n",
            "queries:\n  - input: \"process PDF files\"\n    should_match: true\n  - input: \"deploy kubernetes\"\n    should_match: false\n",
        );
        let result = run_test_suite(&dir).unwrap();
        assert_eq!(result.passed, 2);
        assert_eq!(result.failed, 0);
    }

    #[test]
    fn run_suite_with_failure() {
        let (_parent, dir) = make_skill_with_tests(
            "my-skill",
            "---\nname: my-skill\ndescription: Processes PDF files and generates reports. Use when working with documents.\n---\nBody.\n",
            "queries:\n  - input: \"process PDF files\"\n    should_match: false\n",
        );
        let result = run_test_suite(&dir).unwrap();
        assert_eq!(result.failed, 1);
        assert!(result.results[0].reason.is_some());
    }

    #[test]
    fn run_suite_min_score_check() {
        let (_parent, dir) = make_skill_with_tests(
            "my-skill",
            "---\nname: my-skill\ndescription: Processes PDF files and generates reports. Use when working with documents.\n---\nBody.\n",
            "queries:\n  - input: \"process PDF files\"\n    should_match: true\n    min_score: 0.99\n",
        );
        let result = run_test_suite(&dir).unwrap();
        // Score likely below 0.99, so should fail.
        assert_eq!(result.failed, 1);
        assert!(result.results[0]
            .reason
            .as_ref()
            .unwrap()
            .contains("below minimum"));
    }

    #[test]
    fn missing_tests_yml_returns_error() {
        let parent = tempdir().unwrap();
        let dir = parent.path().join("no-tests");
        fs::create_dir(&dir).unwrap();
        fs::write(
            dir.join("SKILL.md"),
            "---\nname: no-tests\ndescription: Does things\n---\nBody.\n",
        )
        .unwrap();
        let result = run_test_suite(&dir);
        assert!(result.is_err());
    }

    #[test]
    fn generate_fixture_produces_valid_yaml() {
        let parent = tempdir().unwrap();
        let dir = parent.path().join("gen-test");
        fs::create_dir(&dir).unwrap();
        fs::write(
            dir.join("SKILL.md"),
            "---\nname: gen-test\ndescription: Processes documents. Use when handling files.\n---\nBody.\n",
        )
        .unwrap();
        let yaml = generate_fixture(&dir).unwrap();
        assert!(yaml.contains("queries:"));
        assert!(yaml.contains("should_match: true"));
        assert!(yaml.contains("should_match: false"));
        // Verify it parses as valid YAML.
        let fixture: TestFixture = serde_yaml_ng::from_str(&yaml).unwrap();
        assert_eq!(fixture.queries.len(), 2);
    }

    #[test]
    fn format_text_shows_pass_fail() {
        let result = TestSuiteResult {
            passed: 1,
            failed: 1,
            results: vec![
                TestCaseResult {
                    input: "query one".into(),
                    should_match: true,
                    actual_match: true,
                    score: 0.75,
                    passed: true,
                    reason: None,
                },
                TestCaseResult {
                    input: "query two".into(),
                    should_match: true,
                    actual_match: false,
                    score: 0.1,
                    passed: false,
                    reason: Some("expected a match, got no match".into()),
                },
            ],
        };
        let text = format_text(&result);
        assert!(text.contains("[PASS]"));
        assert!(text.contains("[FAIL]"));
        assert!(text.contains("1 passed, 1 failed"));
    }
}
