//! Integration tests against the `anthropics/skills` repository.
//!
//! These tests run aigent commands against real-world skill definitions
//! from <https://github.com/anthropics/skills> (pinned snapshot in
//! `tests/fixtures/anthropics-skills/`).
//!
//! Only Apache 2.0-licensed skills are included. Source-available skills
//! (docx, pdf, pptx, xlsx) are excluded — their license prohibits redistribution.
//!
//! Gated with `#[ignore]` — run with:
//!   cargo test --test anthropics_skills -- --ignored

use assert_cmd::Command;
use predicates::prelude::*;

const FIXTURES: &str = "tests/fixtures/anthropics-skills";

fn aigent() -> Command {
    Command::cargo_bin("aigent").unwrap()
}

/// Generate per-skill integration tests.
///
/// Each invocation creates a module with tests for validate, check,
/// properties, prompt, and score.
macro_rules! skill_tests {
    ($mod_name:ident, $dir_name:literal) => {
        mod $mod_name {
            use super::*;

            fn skill_path() -> String {
                format!("{}/{}", FIXTURES, $dir_name)
            }

            #[test]
            #[ignore]
            fn validate() {
                aigent()
                    .args(["validate", &skill_path()])
                    .assert()
                    .success();
            }

            #[test]
            #[ignore]
            fn check() {
                aigent().args(["check", &skill_path()]).assert().success();
            }

            #[test]
            #[ignore]
            fn properties() {
                let output = aigent()
                    .args(["properties", &skill_path()])
                    .output()
                    .unwrap();
                assert!(output.status.success());
                let stdout = String::from_utf8_lossy(&output.stdout);
                let parsed: serde_json::Value =
                    serde_json::from_str(&stdout).expect("properties should produce valid JSON");
                assert!(parsed["name"].is_string(), "missing 'name' in properties");
                assert!(
                    parsed["description"].is_string(),
                    "missing 'description' in properties"
                );
            }

            #[test]
            #[ignore]
            fn prompt() {
                let output = aigent().args(["prompt", &skill_path()]).output().unwrap();
                assert!(output.status.success());
                let stdout = String::from_utf8_lossy(&output.stdout);
                assert!(
                    stdout.contains("<skill>"),
                    "prompt output should contain <skill> XML tag"
                );
            }

            #[test]
            #[ignore]
            fn score() {
                let output = aigent()
                    .args(["score", &skill_path(), "--format", "json"])
                    .output()
                    .unwrap();
                let stdout = String::from_utf8_lossy(&output.stdout);
                let parsed: serde_json::Value =
                    serde_json::from_str(&stdout).expect("score should produce valid JSON");
                let total = parsed["total"].as_u64().expect("total should be a number");
                assert!(total > 0, "score total should be > 0, got {total}");
            }
        }
    };
}

// ── Per-skill tests (12 Apache 2.0-licensed skills) ─────────────────

skill_tests!(algorithmic_art, "algorithmic-art");
skill_tests!(brand_guidelines, "brand-guidelines");
skill_tests!(canvas_design, "canvas-design");
skill_tests!(doc_coauthoring, "doc-coauthoring");
skill_tests!(frontend_design, "frontend-design");
skill_tests!(internal_comms, "internal-comms");
skill_tests!(mcp_builder, "mcp-builder");
skill_tests!(skill_creator, "skill-creator");
skill_tests!(slack_gif_creator, "slack-gif-creator");
skill_tests!(theme_factory, "theme-factory");
skill_tests!(web_artifacts_builder, "web-artifacts-builder");
skill_tests!(webapp_testing, "webapp-testing");

// ── Template tests ──────────────────────────────────────────────────

/// The official template SKILL.md should parse and produce valid properties,
/// but validation fails because `name: template-skill` doesn't match the
/// directory name `template`.
mod template {
    use super::*;

    fn template_path() -> String {
        format!("{}/template", FIXTURES)
    }

    #[test]
    #[ignore]
    fn properties() {
        let output = aigent()
            .args(["properties", &template_path()])
            .output()
            .unwrap();
        assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        let parsed: serde_json::Value =
            serde_json::from_str(&stdout).expect("template properties should be valid JSON");
        assert_eq!(parsed["name"], "template-skill");
    }

    #[test]
    #[ignore]
    fn validate_fails_name_mismatch() {
        aigent()
            .args(["validate", &template_path()])
            .assert()
            .failure()
            .stderr(predicate::str::contains("does not match directory name"));
    }
}

// ── Recursive discovery tests ───────────────────────────────────────

#[test]
#[ignore]
fn validate_recursive_finds_all_skills() {
    aigent()
        .args(["validate", "--recursive", FIXTURES])
        .assert()
        .failure() // template causes 1 error
        .stderr(predicate::str::contains("13 skills"))
        .stderr(predicate::str::contains("12 ok"))
        .stderr(predicate::str::contains("1 errors"));
}

/// Generate a prompt from all 12 skills at once (multi-dir mode).
#[test]
#[ignore]
fn prompt_multi_generates_all_skills() {
    let skills = [
        "algorithmic-art",
        "brand-guidelines",
        "canvas-design",
        "doc-coauthoring",
        "frontend-design",
        "internal-comms",
        "mcp-builder",
        "skill-creator",
        "slack-gif-creator",
        "theme-factory",
        "web-artifacts-builder",
        "webapp-testing",
    ];
    let paths: Vec<String> = skills.iter().map(|s| format!("{FIXTURES}/{s}")).collect();
    let mut cmd = aigent();
    cmd.arg("prompt");
    for p in &paths {
        cmd.arg(p);
    }
    let output = cmd.output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let skill_count = stdout.matches("<skill>").count();
    assert_eq!(
        skill_count, 12,
        "expected 12 <skill> tags, got {skill_count}"
    );
}
