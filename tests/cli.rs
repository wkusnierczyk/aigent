use std::fs;
use std::path::PathBuf;

use assert_cmd::cargo::cargo_bin_cmd;
use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::tempdir;

/// Return a `Command` for the `aigent` binary built by Cargo.
fn aigent() -> Command {
    cargo_bin_cmd!("aigent")
}

/// Create a temp dir with a named subdirectory containing a SKILL.md.
/// Returns the parent TempDir (for lifetime) and the path to the subdirectory.
fn make_skill_dir(name: &str, content: &str) -> (tempfile::TempDir, PathBuf) {
    let parent = tempdir().unwrap();
    let dir = parent.path().join(name);
    fs::create_dir(&dir).unwrap();
    fs::write(dir.join("SKILL.md"), content).unwrap();
    (parent, dir)
}

// ── Global flags ────────────────────────────────────────────────────

#[test]
fn help_flag() {
    aigent()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("AI agent skill builder"));
}

#[test]
fn version_flag() {
    aigent()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains(env!("CARGO_PKG_VERSION")));
}

#[test]
fn about_flag() {
    aigent()
        .arg("--about")
        .assert()
        .success()
        .stdout(predicate::str::contains("aigent:"))
        .stdout(predicate::str::contains(env!("CARGO_PKG_VERSION")))
        .stdout(predicate::str::contains("author:"))
        .stdout(predicate::str::contains("developer:"))
        .stdout(predicate::str::contains("licence:"))
        .stdout(predicate::str::contains("https://opensource.org/licenses/"));
}

#[test]
fn no_args_shows_usage() {
    aigent()
        .assert()
        .failure()
        .stderr(predicate::str::contains("Usage"));
}

// ── validate ────────────────────────────────────────────────────────

#[test]
fn validate_valid_skill() {
    let (_parent, dir) = make_skill_dir(
        "my-skill",
        "---\nname: my-skill\ndescription: A test skill\n---\nShort body.\n",
    );
    aigent()
        .args(["validate", dir.to_str().unwrap()])
        .assert()
        .success()
        .stderr(predicate::str::contains("ok"));
}

#[test]
fn validate_missing_name() {
    let (_parent, dir) =
        make_skill_dir("bad-skill", "---\ndescription: A test skill\n---\nBody.\n");
    aigent()
        .args(["validate", dir.to_str().unwrap()])
        .assert()
        .failure()
        .stderr(predicate::str::contains("name"));
}

#[test]
fn validate_missing_skill_md() {
    let parent = tempdir().unwrap();
    let dir = parent.path().join("no-skill");
    fs::create_dir(&dir).unwrap();
    aigent()
        .args(["validate", dir.to_str().unwrap()])
        .assert()
        .failure()
        .stderr(predicate::str::contains("SKILL.md"));
}

#[test]
fn validate_warnings_only_exit_zero() {
    // Body with > 500 lines triggers a warning but no error.
    let long_body = "line\n".repeat(501);
    let content = format!("---\nname: warn-skill\ndescription: A test skill\n---\n{long_body}");
    let (_parent, dir) = make_skill_dir("warn-skill", &content);
    aigent()
        .args(["validate", dir.to_str().unwrap()])
        .assert()
        .success()
        .stderr(predicate::str::contains("warning:"));
}

#[test]
fn validate_skill_md_file_path() {
    let (_parent, dir) = make_skill_dir(
        "my-skill",
        "---\nname: my-skill\ndescription: A test skill\n---\nShort body.\n",
    );
    let skill_md = dir.join("SKILL.md");
    aigent()
        .args(["validate", skill_md.to_str().unwrap()])
        .assert()
        .success()
        .stderr(predicate::str::contains("ok"));
}

// ── properties ──────────────────────────────────────────────────────

#[test]
fn properties_valid() {
    let (_parent, dir) = make_skill_dir(
        "my-skill",
        "---\nname: my-skill\ndescription: A test skill\n---\nBody.\n",
    );
    aigent()
        .args(["properties", dir.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"name\""))
        .stdout(predicate::str::contains("my-skill"));
}

#[test]
fn properties_invalid() {
    let parent = tempdir().unwrap();
    let dir = parent.path().join("no-skill");
    fs::create_dir(&dir).unwrap();
    aigent()
        .args(["properties", dir.to_str().unwrap()])
        .assert()
        .failure()
        .stderr(predicate::str::contains("aigent properties:"));
}

#[test]
fn read_properties_alias_works() {
    let (_parent, dir) = make_skill_dir(
        "my-skill",
        "---\nname: my-skill\ndescription: A test skill\n---\nBody.\n",
    );
    aigent()
        .args(["read-properties", dir.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"name\""));
}

#[test]
fn read_properties_skill_md_file_path() {
    let (_parent, dir) = make_skill_dir(
        "my-skill",
        "---\nname: my-skill\ndescription: A test skill\n---\nBody.\n",
    );
    let skill_md = dir.join("SKILL.md");
    aigent()
        .args(["read-properties", skill_md.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"name\""));
}

// ── to-prompt ───────────────────────────────────────────────────────

#[test]
fn to_prompt_single_skill() {
    let (_parent, dir) = make_skill_dir(
        "my-skill",
        "---\nname: my-skill\ndescription: A test skill\n---\nBody.\n",
    );
    aigent()
        .args(["to-prompt", dir.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("<available_skills>"))
        .stdout(predicate::str::contains("<name>my-skill</name>"));
}

#[test]
fn to_prompt_multiple_skills() {
    let (_p1, d1) = make_skill_dir(
        "skill-one",
        "---\nname: skill-one\ndescription: First\n---\nBody.\n",
    );
    let (_p2, d2) = make_skill_dir(
        "skill-two",
        "---\nname: skill-two\ndescription: Second\n---\nBody.\n",
    );
    let out = aigent()
        .args(["to-prompt", d1.to_str().unwrap(), d2.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(out.status.success(), "expected exit code 0");
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert_eq!(stdout.matches("<skill>").count(), 2);
    assert!(stdout.contains("<name>skill-one</name>"));
    assert!(stdout.contains("<name>skill-two</name>"));
}

#[test]
fn to_prompt_no_directories() {
    aigent()
        .args(["to-prompt"])
        .assert()
        .success()
        .stdout(predicate::str::diff("<available_skills>\n</available_skills>").trim());
}

#[test]
fn to_prompt_mixed_valid_and_invalid() {
    let (_p1, good) = make_skill_dir(
        "good-skill",
        "---\nname: good-skill\ndescription: Works\n---\nBody.\n",
    );
    let parent = tempdir().unwrap();
    let bad = parent.path().join("bad-skill");
    fs::create_dir(&bad).unwrap();
    // bad has no SKILL.md
    let out = aigent()
        .args(["to-prompt", good.to_str().unwrap(), bad.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(out.status.success(), "expected exit code 0");
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert_eq!(stdout.matches("<skill>").count(), 1);
    assert!(stdout.contains("<name>good-skill</name>"));
}

// ── new (skill creation) ──────────────────────────────────────────

#[test]
fn new_deterministic_creates_dir() {
    let parent = tempdir().unwrap();
    let dir = parent.path().join("processing-pdf-files");
    aigent()
        .args([
            "new",
            "Process PDF files",
            "--no-llm",
            "--dir",
            dir.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Created skill"));
    assert!(dir.join("SKILL.md").exists());
}

#[test]
fn new_with_name_override() {
    let parent = tempdir().unwrap();
    let dir = parent.path().join("my-pdf-tool");
    aigent()
        .args([
            "new",
            "Process PDF files",
            "--no-llm",
            "--name",
            "my-pdf-tool",
            "--dir",
            dir.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("my-pdf-tool"));
}

#[test]
fn new_with_dir_override() {
    let parent = tempdir().unwrap();
    let dir = parent.path().join("custom-output");
    // Use --name matching the dir name so validation passes.
    aigent()
        .args([
            "new",
            "Process PDF files",
            "--no-llm",
            "--name",
            "custom-output",
            "--dir",
            dir.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("custom-output"));
    assert!(dir.join("SKILL.md").exists());
}

// ── init ───────────────────────────────────────────────────────────

#[test]
fn init_in_empty_dir() {
    let parent = tempdir().unwrap();
    let dir = parent.path().join("new-skill");
    aigent()
        .args(["init", dir.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("Created"));
    assert!(dir.join("SKILL.md").exists());
}

#[test]
fn init_where_skill_md_exists() {
    let (_parent, dir) = make_skill_dir(
        "existing",
        "---\nname: existing\ndescription: Test\n---\nBody.\n",
    );
    aigent()
        .args(["init", dir.to_str().unwrap()])
        .assert()
        .failure()
        .stderr(predicate::str::contains("already exists"));
}

#[test]
fn init_with_dir_arg() {
    let parent = tempdir().unwrap();
    let dir = parent.path().join("specified-dir");
    aigent()
        .args(["init", dir.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("Created"));
    assert!(dir.join("SKILL.md").exists());
}

#[test]
fn new_skill_passes_validate() {
    let parent = tempdir().unwrap();
    let dir = parent.path().join("roundtrip-skill");
    // Create the skill.
    aigent()
        .args([
            "new",
            "Process PDF files",
            "--no-llm",
            "--name",
            "roundtrip-skill",
            "--dir",
            dir.to_str().unwrap(),
        ])
        .assert()
        .success();
    // Validate the built skill.
    aigent()
        .args(["validate", dir.to_str().unwrap()])
        .assert()
        .success();
}

// ── --format flag ──────────────────────────────────────────────────

#[test]
fn validate_format_text_default() {
    let (_parent, dir) = make_skill_dir(
        "my-skill",
        "---\nname: my-skill\ndescription: A test skill\n---\nBody.\n",
    );
    aigent()
        .args(["validate", dir.to_str().unwrap(), "--format", "text"])
        .assert()
        .success()
        .stderr(predicate::str::contains("ok"));
}

#[test]
fn validate_format_json_valid() {
    let (_parent, dir) = make_skill_dir(
        "my-skill",
        "---\nname: my-skill\ndescription: A test skill\n---\nBody.\n",
    );
    let output = aigent()
        .args(["validate", dir.to_str().unwrap(), "--format", "json"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let arr = json.as_array().unwrap();
    assert_eq!(arr.len(), 1, "expected single entry in array");
    assert!(arr[0].get("path").is_some(), "entry should have 'path'");
    let diags = arr[0]["diagnostics"].as_array().unwrap();
    assert!(diags.is_empty(), "expected no diagnostics");
}

#[test]
fn validate_format_json_with_errors() {
    let (_parent, dir) =
        make_skill_dir("bad-skill", "---\ndescription: A test skill\n---\nBody.\n");
    let output = aigent()
        .args(["validate", dir.to_str().unwrap(), "--format", "json"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let arr = json.as_array().unwrap();
    assert_eq!(arr.len(), 1, "expected single entry in array");
    assert!(arr[0].get("path").is_some(), "entry should have 'path'");
    let diags = arr[0]["diagnostics"].as_array().unwrap();
    assert!(!diags.is_empty());
    // Check that diagnostics have expected structure.
    let first = &diags[0];
    assert!(first.get("severity").is_some());
    assert!(first.get("code").is_some());
    assert!(first.get("message").is_some());
}

#[test]
fn validate_format_json_with_warnings() {
    let long_body = "line\n".repeat(501);
    let content = format!("---\nname: warn-skill\ndescription: A test skill\n---\n{long_body}");
    let (_parent, dir) = make_skill_dir("warn-skill", &content);
    let output = aigent()
        .args(["validate", dir.to_str().unwrap(), "--format", "json"])
        .output()
        .unwrap();
    assert!(output.status.success()); // warnings don't cause failure
    let stdout = String::from_utf8(output.stdout).unwrap();
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let arr = json.as_array().unwrap();
    assert_eq!(arr.len(), 1, "expected single entry in array");
    let diags = arr[0]["diagnostics"].as_array().unwrap();
    assert!(
        diags.iter().any(|d| d["severity"] == "warning"),
        "expected warning diagnostic in JSON output"
    );
}

// ── --target flag ──────────────────────────────────────────────────

#[test]
fn validate_target_standard_warns_on_claude_code_fields() {
    let (_parent, dir) = make_skill_dir(
        "ext-skill",
        "---\nname: ext-skill\ndescription: desc\nargument-hint: \"[file]\"\n---\nBody.\n",
    );
    aigent()
        .args(["validate", dir.to_str().unwrap(), "--target", "standard"])
        .assert()
        .success() // warnings only, not errors
        .stderr(
            predicate::str::contains("warning:").and(predicate::str::contains("argument-hint")),
        );
}

#[test]
fn validate_target_claude_code_accepts_extension_fields() {
    let (_parent, dir) = make_skill_dir(
        "ext-skill",
        "---\nname: ext-skill\ndescription: desc\nargument-hint: \"[file]\"\n---\nBody.\n",
    );
    aigent()
        .args(["validate", dir.to_str().unwrap(), "--target", "claude-code"])
        .assert()
        .success()
        .stderr(predicate::str::contains("ok"));
}

#[test]
fn validate_target_permissive_no_unknown_field_warnings() {
    let (_parent, dir) = make_skill_dir(
        "custom-skill",
        "---\nname: custom-skill\ndescription: desc\ncustom-field: value\n---\nBody.\n",
    );
    aigent()
        .args(["validate", dir.to_str().unwrap(), "--target", "permissive"])
        .assert()
        .success()
        .stderr(predicate::str::contains("ok"));
}

// ── check command (validate + semantic) ───────────────────────────

#[test]
fn check_shows_info_diagnostics() {
    // Name is not gerund, description has no trigger phrase and is vague.
    let (_parent, dir) = make_skill_dir(
        "helper",
        "---\nname: helper\ndescription: Helps\n---\nBody.\n",
    );
    aigent()
        .args(["check", dir.to_str().unwrap()])
        .assert()
        .success() // lint info never causes failure
        .stderr(predicate::str::contains("info:"));
}

#[test]
fn validate_without_check_no_info_diagnostics() {
    let (_parent, dir) = make_skill_dir(
        "helper",
        "---\nname: helper\ndescription: Helps\n---\nBody.\n",
    );
    aigent()
        .args(["validate", dir.to_str().unwrap()])
        .assert()
        .success()
        .stderr(predicate::str::contains("info:").not());
}

#[test]
fn check_no_validate_skips_conformance() {
    // A skill with invalid name format — should fail validate, but
    // --no-validate skips conformance so only semantic checks run.
    let (_parent, dir) = make_skill_dir(
        "UPPERCASE",
        "---\nname: UPPERCASE\ndescription: Helps with things\n---\nBody.\n",
    );
    // Without --no-validate: fails because name has uppercase.
    aigent()
        .args(["check", dir.to_str().unwrap()])
        .assert()
        .failure();
    // With --no-validate: only semantic checks, which are info-level.
    aigent()
        .args(["check", dir.to_str().unwrap(), "--no-validate"])
        .assert()
        .success()
        .stderr(predicate::str::contains("info:"));
}

// ── lint alias (maps to check) ────────────────────────────────────

#[test]
fn lint_alias_shows_info() {
    let (_parent, dir) = make_skill_dir(
        "helper",
        "---\nname: helper\ndescription: Helps\n---\nBody.\n",
    );
    // `lint` is an alias for `check` — runs validate + semantic checks.
    aigent()
        .args(["lint", dir.to_str().unwrap()])
        .assert()
        .success()
        .stderr(predicate::str::contains("info:"));
}

#[test]
fn check_perfect_skill_no_output() {
    let (_parent, dir) = make_skill_dir(
        "processing-pdfs",
        "---\nname: processing-pdfs\ndescription: Processes PDF files and generates reports. Use when working with documents.\n---\nBody.\n",
    );
    aigent()
        .args(["check", dir.to_str().unwrap()])
        .assert()
        .success()
        .stderr(predicate::str::contains("ok"));
}

#[test]
fn check_json_format() {
    let (_parent, dir) = make_skill_dir(
        "helper",
        "---\nname: helper\ndescription: Helps\n---\nBody.\n",
    );
    let output = aigent()
        .args(["check", dir.to_str().unwrap(), "--format", "json"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    // Check outputs array-of-objects: [{"path": ..., "diagnostics": [...]}].
    let arr = json.as_array().unwrap();
    assert!(!arr.is_empty());
    let diags = arr[0]["diagnostics"].as_array().unwrap();
    assert!(!diags.is_empty());
    assert!(diags.iter().all(|d| d["severity"] == "info"));
}

// ── multi-dir validation ───────────────────────────────────────────

#[test]
fn validate_multiple_dirs() {
    let (_p1, d1) = make_skill_dir(
        "skill-one",
        "---\nname: skill-one\ndescription: First skill\n---\nBody.\n",
    );
    let (_p2, d2) = make_skill_dir(
        "skill-two",
        "---\nname: skill-two\ndescription: Second skill\n---\nBody.\n",
    );
    aigent()
        .args(["validate", d1.to_str().unwrap(), d2.to_str().unwrap()])
        .assert()
        .success();
}

#[test]
fn validate_multiple_dirs_with_errors() {
    let (_p1, d1) = make_skill_dir(
        "good-skill",
        "---\nname: good-skill\ndescription: A test skill\n---\nBody.\n",
    );
    let (_p2, d2) = make_skill_dir("bad-skill", "---\ndescription: No name\n---\nBody.\n");
    aigent()
        .args(["validate", d1.to_str().unwrap(), d2.to_str().unwrap()])
        .assert()
        .failure() // any error → failure
        .stderr(predicate::str::contains("skills:"));
}

// ── --recursive flag ───────────────────────────────────────────────

#[test]
fn validate_recursive_discovers_skills() {
    let parent = tempdir().unwrap();
    let skill_a = parent.path().join("skill-a");
    let skill_b = parent.path().join("skill-b");
    fs::create_dir(&skill_a).unwrap();
    fs::create_dir(&skill_b).unwrap();
    fs::write(
        skill_a.join("SKILL.md"),
        "---\nname: skill-a\ndescription: First\n---\nBody.\n",
    )
    .unwrap();
    fs::write(
        skill_b.join("SKILL.md"),
        "---\nname: skill-b\ndescription: Second\n---\nBody.\n",
    )
    .unwrap();
    aigent()
        .args(["validate", parent.path().to_str().unwrap(), "--recursive"])
        .assert()
        .success()
        .stderr(predicate::str::contains("skills:"));
}

#[test]
fn validate_recursive_no_skills_found() {
    let parent = tempdir().unwrap();
    // Empty dir, no SKILL.md files.
    aigent()
        .args(["validate", parent.path().to_str().unwrap(), "--recursive"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("No SKILL.md files found"));
}

// ── --apply-fixes flag ─────────────────────────────────────────────

#[test]
fn validate_apply_fixes_uppercase_name() {
    let (_parent, dir) = make_skill_dir(
        "myskill",
        "---\nname: MySkill\ndescription: A valid skill for testing\n---\nBody.\n",
    );
    aigent()
        .args(["validate", dir.to_str().unwrap(), "--apply-fixes"])
        .assert()
        .stderr(predicate::str::contains("Applied"));
    // After fix, re-validate should pass (name lowercased).
    let content = fs::read_to_string(dir.join("SKILL.md")).unwrap();
    assert!(
        content.contains("name: myskill"),
        "name should be lowercased: {content}"
    );
}

#[test]
fn validate_apply_fixes_xml_tags_in_description() {
    let (_parent, dir) = make_skill_dir(
        "test",
        "---\nname: test\ndescription: A <b>bold</b> skill for testing\n---\nBody.\n",
    );
    aigent()
        .args(["validate", dir.to_str().unwrap(), "--apply-fixes"])
        .assert()
        .stderr(predicate::str::contains("Applied"));
    let content = fs::read_to_string(dir.join("SKILL.md")).unwrap();
    assert!(
        !content.contains("<b>"),
        "XML tags should be removed: {content}"
    );
}

// ── recursive mode with file path ───────────────────────────────────

#[test]
fn validate_recursive_with_file_path_input() {
    // Passing a SKILL.md file path with --recursive should resolve to
    // the parent and discover skills from there.
    let parent = tempdir().unwrap();
    let skill_a = parent.path().join("skill-a");
    let skill_b = parent.path().join("skill-b");
    fs::create_dir(&skill_a).unwrap();
    fs::create_dir(&skill_b).unwrap();
    fs::write(
        skill_a.join("SKILL.md"),
        "---\nname: skill-a\ndescription: First\n---\nBody.\n",
    )
    .unwrap();
    fs::write(
        skill_b.join("SKILL.md"),
        "---\nname: skill-b\ndescription: Second\n---\nBody.\n",
    )
    .unwrap();
    // Pass a file path (SKILL.md) instead of a directory with --recursive.
    let skill_a_md = skill_a.join("SKILL.md");
    aigent()
        .args(["validate", skill_a_md.to_str().unwrap(), "--recursive"])
        .assert()
        .success();
}

// ── JSON output shape consistency ───────────────────────────────────

#[test]
fn validate_json_shape_consistent_single_and_multi_dir() {
    // Both single-dir and multi-dir should produce the same JSON shape:
    // an array of objects with "path" and "diagnostics" keys.
    let (_p1, d1) = make_skill_dir(
        "json-skill-one",
        "---\nname: json-skill-one\ndescription: First\n---\nBody.\n",
    );
    let (_p2, d2) = make_skill_dir(
        "json-skill-two",
        "---\nname: json-skill-two\ndescription: Second\n---\nBody.\n",
    );

    // Single dir.
    let single = aigent()
        .args(["validate", d1.to_str().unwrap(), "--format", "json"])
        .output()
        .unwrap();
    let single_json: serde_json::Value =
        serde_json::from_str(&String::from_utf8(single.stdout).unwrap()).unwrap();

    // Multi dir.
    let multi = aigent()
        .args([
            "validate",
            d1.to_str().unwrap(),
            d2.to_str().unwrap(),
            "--format",
            "json",
        ])
        .output()
        .unwrap();
    let multi_json: serde_json::Value =
        serde_json::from_str(&String::from_utf8(multi.stdout).unwrap()).unwrap();

    // Both should be arrays.
    assert!(single_json.is_array(), "single-dir JSON should be an array");
    assert!(multi_json.is_array(), "multi-dir JSON should be an array");

    // Both entries should have the same shape: {path, diagnostics}.
    let single_entry = &single_json.as_array().unwrap()[0];
    let multi_entry = &multi_json.as_array().unwrap()[0];
    assert!(
        single_entry.get("path").is_some(),
        "single-dir entry should have 'path'"
    );
    assert!(
        single_entry.get("diagnostics").is_some(),
        "single-dir entry should have 'diagnostics'"
    );
    assert!(
        multi_entry.get("path").is_some(),
        "multi-dir entry should have 'path'"
    );
    assert!(
        multi_entry.get("diagnostics").is_some(),
        "multi-dir entry should have 'diagnostics'"
    );
}

// ── M11: to-prompt --format flag ──────────────────────────────────

#[test]
fn to_prompt_format_json() {
    let (_parent, dir) = make_skill_dir(
        "my-skill",
        "---\nname: my-skill\ndescription: A test skill\n---\nBody.\n",
    );
    let output = aigent()
        .args(["to-prompt", dir.to_str().unwrap(), "--format", "json"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert!(json.is_array());
    let arr = json.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["name"], "my-skill");
}

#[test]
fn to_prompt_format_yaml() {
    let (_parent, dir) = make_skill_dir(
        "my-skill",
        "---\nname: my-skill\ndescription: A test skill\n---\nBody.\n",
    );
    aigent()
        .args(["to-prompt", dir.to_str().unwrap(), "--format", "yaml"])
        .assert()
        .success()
        .stdout(predicate::str::contains("skills:"))
        .stdout(predicate::str::contains("- name: my-skill"));
}

#[test]
fn to_prompt_format_markdown() {
    let (_parent, dir) = make_skill_dir(
        "my-skill",
        "---\nname: my-skill\ndescription: A test skill\n---\nBody.\n",
    );
    aigent()
        .args(["to-prompt", dir.to_str().unwrap(), "--format", "markdown"])
        .assert()
        .success()
        .stdout(predicate::str::contains("# Available Skills"))
        .stdout(predicate::str::contains("## my-skill"));
}

#[test]
fn to_prompt_budget_flag() {
    let (_parent, dir) = make_skill_dir(
        "my-skill",
        "---\nname: my-skill\ndescription: A test skill\n---\nBody.\n",
    );
    aigent()
        .args(["to-prompt", dir.to_str().unwrap(), "--budget"])
        .assert()
        .success()
        .stderr(predicate::str::contains("Token budget"))
        .stderr(predicate::str::contains("Total:"));
}

// ── M11: to-prompt --output flag ──────────────────────────────────

#[test]
fn to_prompt_output_creates_file() {
    let (_parent, dir) = make_skill_dir(
        "my-skill",
        "---\nname: my-skill\ndescription: A test skill\n---\nBody.\n",
    );
    let out_dir = tempdir().unwrap();
    let out_file = out_dir.path().join("prompt.xml");
    aigent()
        .args([
            "to-prompt",
            dir.to_str().unwrap(),
            "--output",
            out_file.to_str().unwrap(),
        ])
        .assert()
        .failure() // exit 1 = changed (file didn't exist)
        .stderr(predicate::str::contains("Updated"));
    assert!(out_file.exists());
    let content = fs::read_to_string(&out_file).unwrap();
    assert!(content.contains("<available_skills>"));
}

#[test]
fn to_prompt_output_unchanged_exit_zero() {
    let (_parent, dir) = make_skill_dir(
        "my-skill",
        "---\nname: my-skill\ndescription: A test skill\n---\nBody.\n",
    );
    let out_dir = tempdir().unwrap();
    let out_file = out_dir.path().join("prompt.xml");
    // First run: creates file (exit 1).
    aigent()
        .args([
            "to-prompt",
            dir.to_str().unwrap(),
            "--output",
            out_file.to_str().unwrap(),
        ])
        .assert()
        .failure();
    // Second run: same input, should be unchanged (exit 0).
    aigent()
        .args([
            "to-prompt",
            dir.to_str().unwrap(),
            "--output",
            out_file.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stderr(predicate::str::contains("Unchanged"));
}

#[test]
fn to_prompt_output_with_format() {
    let (_parent, dir) = make_skill_dir(
        "my-skill",
        "---\nname: my-skill\ndescription: A test skill\n---\nBody.\n",
    );
    let out_dir = tempdir().unwrap();
    let out_file = out_dir.path().join("prompt.json");
    aigent()
        .args([
            "to-prompt",
            dir.to_str().unwrap(),
            "--format",
            "json",
            "--output",
            out_file.to_str().unwrap(),
        ])
        .assert()
        .failure(); // exit 1 = changed
    let content = fs::read_to_string(&out_file).unwrap();
    let json: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert!(json.is_array());
}

// ── M11: init --template flag ─────────────────────────────────────

#[test]
fn init_with_template_reference_guide() {
    let parent = tempdir().unwrap();
    let dir = parent.path().join("ref-skill");
    aigent()
        .args([
            "init",
            dir.to_str().unwrap(),
            "--template",
            "reference-guide",
        ])
        .assert()
        .success();
    assert!(dir.join("SKILL.md").exists());
    assert!(dir.join("REFERENCE.md").exists());
    assert!(dir.join("EXAMPLES.md").exists());
}

#[test]
fn init_with_template_code_skill() {
    let parent = tempdir().unwrap();
    let dir = parent.path().join("code-skill");
    aigent()
        .args(["init", dir.to_str().unwrap(), "--template", "code-skill"])
        .assert()
        .success();
    assert!(dir.join("SKILL.md").exists());
    assert!(dir.join("scripts/run.sh").exists());
}

#[test]
fn init_with_template_claude_code() {
    let parent = tempdir().unwrap();
    let dir = parent.path().join("cc-skill");
    aigent()
        .args(["init", dir.to_str().unwrap(), "--template", "claude-code"])
        .assert()
        .success();
    let content = fs::read_to_string(dir.join("SKILL.md")).unwrap();
    assert!(content.contains("user-invocable: true"));
}

// ── M12: score subcommand ──────────────────────────────────────────

#[test]
fn score_perfect_skill_exits_zero() {
    let (_parent, dir) = make_skill_dir(
        "processing-pdfs",
        "---\nname: processing-pdfs\ndescription: >-\n  Processes PDF files and generates detailed reports.\n  Use when working with documents.\n---\nBody.\n",
    );
    aigent()
        .args(["score", dir.to_str().unwrap()])
        .assert()
        .success()
        .stderr(predicate::str::contains("Score: 100/100"));
}

#[test]
fn score_imperfect_skill_exits_nonzero() {
    let (_parent, dir) = make_skill_dir(
        "helper",
        "---\nname: helper\ndescription: Helps\n---\nBody.\n",
    );
    aigent()
        .args(["score", dir.to_str().unwrap()])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Score:"));
}

#[test]
fn score_json_format() {
    let (_parent, dir) = make_skill_dir(
        "processing-pdfs",
        "---\nname: processing-pdfs\ndescription: >-\n  Processes PDF files and generates detailed reports.\n  Use when working with documents.\n---\nBody.\n",
    );
    let output = aigent()
        .args(["score", dir.to_str().unwrap(), "--format", "json"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(json["total"], 100);
    assert_eq!(json["max"], 100);
}

#[test]
fn score_missing_skill_exits_nonzero() {
    let parent = tempdir().unwrap();
    aigent()
        .args(["score", parent.path().to_str().unwrap()])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Score:"));
}

// ── M12: --structure flag ──────────────────────────────────────────

#[test]
fn validate_structure_flag_accepted() {
    let (_parent, dir) = make_skill_dir(
        "my-skill",
        "---\nname: my-skill\ndescription: A test skill\n---\nBody.\n",
    );
    aigent()
        .args(["validate", dir.to_str().unwrap(), "--structure"])
        .assert()
        .success();
}

#[test]
fn validate_structure_detects_missing_reference() {
    let (_parent, dir) = make_skill_dir(
        "my-skill",
        "---\nname: my-skill\ndescription: A test skill\n---\n\nSee [guide](guide.md) for details.\n",
    );
    aigent()
        .args(["validate", dir.to_str().unwrap(), "--structure"])
        .assert()
        .success() // structure checks are warnings, not errors
        .stderr(predicate::str::contains("warning:").and(predicate::str::contains("guide.md")));
}

#[test]
fn validate_structure_clean_skill_no_warnings() {
    let (_parent, dir) = make_skill_dir(
        "my-skill",
        "---\nname: my-skill\ndescription: A test skill\n---\n\nSee [guide](guide.md) for details.\n",
    );
    fs::write(dir.join("guide.md"), "# Guide").unwrap();
    aigent()
        .args(["validate", dir.to_str().unwrap(), "--structure"])
        .assert()
        .success()
        .stderr(predicate::str::contains("ok"));
}

// ── M12: doc subcommand ──────────────────────────────────────────

#[test]
fn doc_generates_markdown_catalog() {
    let (_parent, dir) = make_skill_dir(
        "my-doc-skill",
        "---\nname: my-doc-skill\ndescription: A documented skill\n---\nBody.\n",
    );
    aigent()
        .args(["doc", dir.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("# Skill Catalog"))
        .stdout(predicate::str::contains("## my-doc-skill"))
        .stdout(predicate::str::contains("A documented skill"));
}

#[test]
fn doc_no_args_defaults_to_current_dir() {
    // With default_value = ".", `doc` without args uses the current directory.
    // From a non-skill directory, it produces an empty catalog with a warning.
    aigent()
        .arg("doc")
        .assert()
        .success()
        .stderr(predicate::str::contains("cannot read skill properties"));
}

#[test]
fn doc_output_writes_file() {
    let (_parent, dir) = make_skill_dir(
        "doc-out-skill",
        "---\nname: doc-out-skill\ndescription: Outputs to file\n---\nBody.\n",
    );
    let outdir = tempdir().unwrap();
    let outfile = outdir.path().join("catalog.md");
    aigent()
        .args([
            "doc",
            dir.to_str().unwrap(),
            "--output",
            outfile.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stderr(predicate::str::contains("Updated"));
    let content = fs::read_to_string(&outfile).unwrap();
    assert!(content.contains("# Skill Catalog"));
    assert!(content.contains("doc-out-skill"));
}

#[test]
fn doc_output_unchanged_on_rerun() {
    let (_parent, dir) = make_skill_dir(
        "doc-stable",
        "---\nname: doc-stable\ndescription: Stable\n---\nBody.\n",
    );
    let outdir = tempdir().unwrap();
    let outfile = outdir.path().join("catalog.md");
    // First run: creates file.
    aigent()
        .args([
            "doc",
            dir.to_str().unwrap(),
            "--output",
            outfile.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stderr(predicate::str::contains("Updated"));
    // Second run: content unchanged → "Unchanged".
    aigent()
        .args([
            "doc",
            dir.to_str().unwrap(),
            "--output",
            outfile.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stderr(predicate::str::contains("Unchanged"));
}

#[test]
fn doc_recursive_discovers_nested_skills() {
    let parent = tempdir().unwrap();
    let nested = parent.path().join("skills").join("nested-skill");
    fs::create_dir_all(&nested).unwrap();
    fs::write(
        nested.join("SKILL.md"),
        "---\nname: nested-skill\ndescription: Found recursively\n---\nBody.\n",
    )
    .unwrap();
    aigent()
        .args([
            "doc",
            parent.path().join("skills").to_str().unwrap(),
            "--recursive",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("nested-skill"));
}

// ── M12: test subcommand ─────────────────────────────────────────

#[test]
fn probe_skill_shows_activation_status() {
    let (_parent, dir) = make_skill_dir(
        "test-skill-activate",
        "---\nname: test-skill-activate\ndescription: Processes PDF files and extracts text\n---\nBody.\n",
    );
    aigent()
        .args([
            "probe",
            dir.to_str().unwrap(),
            "--query",
            "process PDF files",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Activation:"))
        .stdout(predicate::str::contains("STRONG"));
}

#[test]
fn probe_skill_no_match_query() {
    let (_parent, dir) = make_skill_dir(
        "test-no-match",
        "---\nname: test-no-match\ndescription: Manages database connections\n---\nBody.\n",
    );
    aigent()
        .args([
            "probe",
            dir.to_str().unwrap(),
            "--query",
            "deploy kubernetes cluster",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("NONE"));
}

#[test]
fn probe_skill_json_format() {
    let (_parent, dir) = make_skill_dir(
        "test-json",
        "---\nname: test-json\ndescription: Processes PDF files\n---\nBody.\n",
    );
    let output = aigent()
        .args([
            "probe",
            dir.to_str().unwrap(),
            "--query",
            "process PDF",
            "--format",
            "json",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(json["name"], "test-json");
    assert!(json["estimated_tokens"].as_u64().unwrap() > 0);
}

#[test]
fn probe_skill_missing_dir_exits_nonzero() {
    aigent()
        .args(["probe", "/nonexistent/skill", "--query", "some query"])
        .assert()
        .failure();
}

// ── M12: upgrade subcommand ──────────────────────────────────────

#[test]
fn upgrade_detects_missing_compatibility() {
    let (_parent, dir) = make_skill_dir(
        "upgrade-test",
        "---\nname: upgrade-test\ndescription: A basic skill\n---\nBody.\n",
    );
    aigent()
        .args(["upgrade", dir.to_str().unwrap()])
        .assert()
        .failure()
        .stderr(predicate::str::contains("compatibility"));
}

#[test]
fn upgrade_detects_missing_trigger_phrase() {
    let (_parent, dir) = make_skill_dir(
        "upgrade-trigger",
        "---\nname: upgrade-trigger\ndescription: Does something\n---\nBody.\n",
    );
    aigent()
        .args(["upgrade", dir.to_str().unwrap()])
        .assert()
        .failure()
        .stderr(predicate::str::contains("trigger phrase"));
}

#[test]
fn upgrade_clean_skill_no_suggestions() {
    let (_parent, dir) = make_skill_dir(
        "upgrade-clean",
        "---\nname: upgrade-clean\ndescription: >-\n  Manages user sessions. Use when handling authentication.\ncompatibility: claude-code\nmetadata:\n  version: '1.0.0'\n  author: test\n---\nBody.\n",
    );
    aigent()
        .args(["upgrade", dir.to_str().unwrap()])
        .assert()
        .success()
        .stderr(predicate::str::contains("No upgrade suggestions"));
}

#[test]
fn upgrade_apply_modifies_skill() {
    let (_parent, dir) = make_skill_dir(
        "upgrade-apply",
        "---\nname: upgrade-apply\ndescription: A basic skill\n---\nBody.\n",
    );
    aigent()
        .args(["upgrade", dir.to_str().unwrap(), "--apply"])
        .assert()
        .success()
        .stderr(predicate::str::contains("Applied"));
    // Verify the file was updated.
    let content = fs::read_to_string(dir.join("SKILL.md")).unwrap();
    assert!(content.contains("compatibility"));
}

#[test]
fn upgrade_full_reports_suggestions() {
    let (_parent, dir) = make_skill_dir(
        "upgrade-full-test",
        "---\nname: upgrade-full-test\ndescription: A basic skill\n---\nBody.\n",
    );
    // Without --apply, exits 1 when suggestions exist (same as upgrade without --full).
    aigent()
        .args(["upgrade", dir.to_str().unwrap(), "--full"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("compatibility"))
        .stderr(predicate::str::contains("metadata"));
}

#[test]
fn upgrade_full_apply_fixes_then_upgrades() {
    let (_parent, dir) = make_skill_dir(
        "upgrade-full-apply",
        "---\nname: upgrade-full-apply\ndescription: A basic skill\n---\nBody.\n",
    );
    aigent()
        .args(["upgrade", dir.to_str().unwrap(), "--full", "--apply"])
        .assert()
        .success()
        .stderr(predicate::str::contains("Applied"));
    // Verify the file was updated.
    let content = fs::read_to_string(dir.join("SKILL.md")).unwrap();
    assert!(content.contains("compatibility"));
}

// ── M13: YAML parser edge cases (#81) ────────────────────────────

#[test]
fn upgrade_apply_preserves_existing_metadata_keys() {
    // Skill has metadata.version but NOT metadata.author — apply should add only author.
    let (_parent, dir) = make_skill_dir(
        "upgrade-partial-meta",
        "---\nname: upgrade-partial-meta\ndescription: A basic skill\nmetadata:\n  version: '1.0.0'\n---\nBody.\n",
    );
    aigent()
        .args(["upgrade", dir.to_str().unwrap(), "--apply"])
        .assert()
        .success()
        .stderr(predicate::str::contains("Applied"));
    let content = fs::read_to_string(dir.join("SKILL.md")).unwrap();
    // version should still be there (preserved).
    assert!(content.contains("version: '1.0.0'"));
    // author should have been added.
    assert!(content.contains("author: unknown"));
    // Should still be valid YAML after modifications.
    aigent()
        .args(["validate", dir.to_str().unwrap()])
        .assert()
        .success();
}

#[test]
fn upgrade_apply_respects_four_space_indentation() {
    // Skill uses 4-space indentation — upgrade should detect and use 4 spaces.
    let (_parent, dir) = make_skill_dir(
        "upgrade-4sp-indent",
        "---\nname: upgrade-4sp-indent\ndescription: A basic skill\nmetadata:\n    version: '1.0.0'\n---\nBody.\n",
    );
    aigent()
        .args(["upgrade", dir.to_str().unwrap(), "--apply"])
        .assert()
        .success();
    let content = fs::read_to_string(dir.join("SKILL.md")).unwrap();
    // The added author should use 4-space indentation to match existing style.
    assert!(
        content.contains("    author: unknown"),
        "Expected 4-space indented author line, got:\n{content}"
    );
}

#[test]
fn upgrade_apply_with_comments_in_frontmatter() {
    // Frontmatter has comment lines — the parser should skip them when locating metadata.
    let (_parent, dir) = make_skill_dir(
        "upgrade-comments",
        "---\n# This is a comment\nname: upgrade-comments\ndescription: A basic skill\n# Another comment\nmetadata:\n  version: '1.0.0'\n---\nBody.\n",
    );
    aigent()
        .args(["upgrade", dir.to_str().unwrap(), "--apply"])
        .assert()
        .success();
    let content = fs::read_to_string(dir.join("SKILL.md")).unwrap();
    // Comments should be preserved.
    assert!(content.contains("# This is a comment"));
    assert!(content.contains("# Another comment"));
    // author should have been inserted.
    assert!(content.contains("author: unknown"));
}

#[test]
fn upgrade_apply_no_metadata_block_creates_one() {
    // Skill has no metadata block at all — apply should add the whole block.
    let (_parent, dir) = make_skill_dir(
        "upgrade-no-meta",
        "---\nname: upgrade-no-meta\ndescription: A basic skill\n---\nBody.\n",
    );
    aigent()
        .args(["upgrade", dir.to_str().unwrap(), "--apply"])
        .assert()
        .success();
    let content = fs::read_to_string(dir.join("SKILL.md")).unwrap();
    // Should create a metadata block with both version and author.
    assert!(content.contains("metadata:"));
    assert!(content.contains("version: '0.1.0'"));
    assert!(content.contains("author: unknown"));
    // Should also add compatibility.
    assert!(content.contains("compatibility: claude-code"));
    // The resulting file should still be valid.
    aigent()
        .args(["validate", dir.to_str().unwrap()])
        .assert()
        .success();
}

// ── M13: CLI renames and aliases (#76) ───────────────────────────

#[test]
fn new_command_creates_skill() {
    let parent = tempdir().unwrap();
    aigent()
        .args([
            "new",
            "Process PDF files",
            "--no-llm",
            "--dir",
            parent.path().join("processing-pdf-files").to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Created skill"));
}

#[test]
fn create_alias_produces_same_as_new() {
    let parent = tempdir().unwrap();
    // `create` is an alias for `new` — should produce same result.
    aigent()
        .args([
            "create",
            "Analyze logs",
            "--no-llm",
            "--dir",
            parent.path().join("analyzing-logs").to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Created skill"));
}

#[test]
fn prompt_command_works() {
    let (_parent, dir) = make_skill_dir(
        "my-skill",
        "---\nname: my-skill\ndescription: A test skill\n---\nBody.\n",
    );
    aigent()
        .args(["prompt", dir.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("<available_skills>"));
}

#[test]
fn to_prompt_alias_works() {
    let (_parent, dir) = make_skill_dir(
        "my-skill",
        "---\nname: my-skill\ndescription: A test skill\n---\nBody.\n",
    );
    aigent()
        .args(["to-prompt", dir.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("<available_skills>"));
}

#[test]
fn probe_command_shows_activation() {
    let (_parent, dir) = make_skill_dir(
        "processing-pdf-files",
        "---\nname: processing-pdf-files\ndescription: Processes PDF files and generates reports. Use when working with documents.\n---\nBody.\n",
    );
    aigent()
        .args([
            "probe",
            dir.to_str().unwrap(),
            "--query",
            "process PDF files",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Activation:"));
}

#[test]
fn probe_defaults_to_current_dir() {
    let (_parent, dir) = make_skill_dir(
        "processing-pdf-files",
        "---\nname: processing-pdf-files\ndescription: Processes PDF files and generates reports. Use when working with documents.\n---\nBody.\n",
    );
    aigent()
        .args(["probe", "--query", "process PDF files"])
        .current_dir(&dir)
        .assert()
        .success()
        .stdout(predicate::str::contains("Activation:"));
}

#[test]
fn probe_multiple_dirs_ranked() {
    let parent = tempdir().unwrap();
    let dir_a = parent.path().join("skill-a");
    let dir_b = parent.path().join("skill-b");
    fs::create_dir(&dir_a).unwrap();
    fs::create_dir(&dir_b).unwrap();
    fs::write(
        dir_a.join("SKILL.md"),
        "---\nname: skill-a\ndescription: Processes PDF files and extracts text\n---\nBody.\n",
    )
    .unwrap();
    fs::write(
        dir_b.join("SKILL.md"),
        "---\nname: skill-b\ndescription: Manages database connections\n---\nBody.\n",
    )
    .unwrap();
    aigent()
        .args([
            "probe",
            dir_a.to_str().unwrap(),
            dir_b.to_str().unwrap(),
            "--query",
            "process PDF files",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("skill-a"))
        .stdout(predicate::str::contains("skill-b"));
}

#[test]
fn check_runs_validate_and_lint() {
    // A skill with a lint issue (no trigger phrase) but valid spec.
    let (_parent, dir) = make_skill_dir(
        "helper",
        "---\nname: helper\ndescription: Helps with things\n---\nBody.\n",
    );
    // `check` should run both validate (passes) and semantic lint (finds issues).
    aigent()
        .args(["check", dir.to_str().unwrap()])
        .assert()
        .success()
        .stderr(predicate::str::contains("info:"));
}

#[test]
fn lint_alias_maps_to_check() {
    let (_parent, dir) = make_skill_dir(
        "helper",
        "---\nname: helper\ndescription: Helps with things\n---\nBody.\n",
    );
    // `lint` is an alias for `check`.
    aigent()
        .args(["lint", dir.to_str().unwrap()])
        .assert()
        .success()
        .stderr(predicate::str::contains("info:"));
}

// ── M13: build (plugin assembly) (#83) ───────────────────────────

#[test]
fn build_assembles_single_skill() {
    let (_parent, dir) = make_skill_dir(
        "my-skill",
        "---\nname: my-skill\ndescription: Does things\n---\nBody.\n",
    );
    let output = tempdir().unwrap();
    let out_dir = output.path().join("plugin");
    aigent()
        .args([
            "build",
            dir.to_str().unwrap(),
            "--output",
            out_dir.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Assembled 1 skill"));
    assert!(out_dir.join("plugin.json").exists());
    assert!(out_dir.join("skills/my-skill/SKILL.md").exists());
}

#[test]
fn build_assembles_multiple_skills() {
    let (_p1, d1) = make_skill_dir(
        "skill-one",
        "---\nname: skill-one\ndescription: First\n---\nBody.\n",
    );
    let (_p2, d2) = make_skill_dir(
        "skill-two",
        "---\nname: skill-two\ndescription: Second\n---\nBody.\n",
    );
    let output = tempdir().unwrap();
    let out_dir = output.path().join("plugin");
    aigent()
        .args([
            "build",
            d1.to_str().unwrap(),
            d2.to_str().unwrap(),
            "--output",
            out_dir.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Assembled 2 skill"));
}

#[test]
fn build_with_validate_rejects_invalid() {
    let (_parent, dir) = make_skill_dir(
        "bad-skill",
        "---\ndescription: Missing name field\n---\nBody.\n",
    );
    let output = tempdir().unwrap();
    let out_dir = output.path().join("plugin");
    aigent()
        .args([
            "build",
            dir.to_str().unwrap(),
            "--output",
            out_dir.to_str().unwrap(),
            "--validate",
        ])
        .assert()
        .failure();
}

#[test]
fn build_plugin_json_valid() {
    let (_parent, dir) = make_skill_dir(
        "test-skill",
        "---\nname: test-skill\ndescription: Does things\n---\nBody.\n",
    );
    let output = tempdir().unwrap();
    let out_dir = output.path().join("plugin");
    aigent()
        .args([
            "build",
            dir.to_str().unwrap(),
            "--output",
            out_dir.to_str().unwrap(),
            "--name",
            "my-plugin",
        ])
        .assert()
        .success();
    let json_str = fs::read_to_string(out_dir.join("plugin.json")).unwrap();
    let json: serde_json::Value = serde_json::from_str(&json_str).unwrap();
    assert_eq!(json["name"], "my-plugin");
    assert_eq!(json["version"], "0.1.0");
}

// ── M13: fmt subcommand (#76) ────────────────────────────────────

#[test]
fn fmt_already_formatted_no_change() {
    // Keys are already in canonical order.
    let (_parent, dir) = make_skill_dir(
        "formatted-skill",
        "---\nname: formatted-skill\ndescription: Does things\ncompatibility: claude-code\nmetadata:\n  version: '1.0'\n---\nBody.\n",
    );
    aigent()
        .args(["fmt", dir.to_str().unwrap()])
        .assert()
        .success()
        .stderr(predicate::str::contains("Formatted").not());
}

#[test]
fn fmt_reorders_keys() {
    // metadata before name — should be reordered.
    let (_parent, dir) = make_skill_dir(
        "unformatted-skill",
        "---\nmetadata:\n  version: '1.0'\nname: unformatted-skill\ndescription: Does things\n---\nBody.\n",
    );
    aigent()
        .args(["fmt", dir.to_str().unwrap()])
        .assert()
        .success()
        .stderr(predicate::str::contains("Formatted"));
    // Verify the file was reordered.
    let content = fs::read_to_string(dir.join("SKILL.md")).unwrap();
    let name_pos = content.find("name:").unwrap();
    let meta_pos = content.find("metadata:").unwrap();
    assert!(
        name_pos < meta_pos,
        "name should come before metadata after fmt"
    );
}

#[test]
fn fmt_check_unformatted_exits_nonzero() {
    let (_parent, dir) = make_skill_dir(
        "check-skill",
        "---\nmetadata:\n  version: '1.0'\nname: check-skill\ndescription: Does things\n---\nBody.\n",
    );
    aigent()
        .args(["fmt", dir.to_str().unwrap(), "--check"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Would reformat"))
        .stderr(predicate::str::contains("---"))
        .stderr(predicate::str::contains("+++"))
        .stderr(predicate::str::contains("@@"));
    // File should NOT have been modified.
    let content = fs::read_to_string(dir.join("SKILL.md")).unwrap();
    assert!(
        content.starts_with("---\nmetadata:"),
        "file should be unchanged in --check mode"
    );
}

#[test]
fn fmt_check_shows_diff_content() {
    let (_parent, dir) = make_skill_dir(
        "diff-skill",
        "---\nallowed-tools: Bash\nname: diff-skill\ndescription: Does things\n---\nBody.\n",
    );
    aigent()
        .args(["fmt", dir.to_str().unwrap(), "--check"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("-allowed-tools"))
        .stderr(predicate::str::contains("+allowed-tools"));
}

#[test]
fn fmt_check_formatted_no_diff() {
    let (_parent, dir) = make_skill_dir(
        "nodiff-skill",
        "---\nname: nodiff-skill\ndescription: Does things\n---\nBody.\n",
    );
    aigent()
        .args(["fmt", dir.to_str().unwrap(), "--check"])
        .assert()
        .success()
        .stderr(predicate::str::contains("Would reformat").not())
        .stderr(predicate::str::contains("@@").not());
}

#[test]
fn fmt_check_formatted_exits_zero() {
    let (_parent, dir) = make_skill_dir(
        "clean-skill",
        "---\nname: clean-skill\ndescription: Does things\n---\nBody.\n",
    );
    aigent()
        .args(["fmt", dir.to_str().unwrap(), "--check"])
        .assert()
        .success();
}

#[test]
fn fmt_preserves_values_after_reorder() {
    let (_parent, dir) = make_skill_dir(
        "preserve-test",
        "---\ncompatibility: claude-code\nname: preserve-test\ndescription: >-\n  A multiline description\n  that spans two lines\nmetadata:\n  version: '1.0'\n  author: test\n---\n# Body\n\nParagraph.\n",
    );
    aigent()
        .args(["fmt", dir.to_str().unwrap()])
        .assert()
        .success();
    let content = fs::read_to_string(dir.join("SKILL.md")).unwrap();
    // All values should be preserved.
    assert!(content.contains("name: preserve-test"));
    assert!(content.contains("A multiline description"));
    assert!(content.contains("compatibility: claude-code"));
    assert!(content.contains("version: '1.0'"));
    assert!(content.contains("# Body"));
    // Should still be valid.
    aigent()
        .args(["validate", dir.to_str().unwrap()])
        .assert()
        .success();
}

#[test]
fn format_alias_works() {
    let (_parent, dir) = make_skill_dir(
        "alias-test",
        "---\nname: alias-test\ndescription: Does things\n---\nBody.\n",
    );
    // `format` is an alias for `fmt`.
    aigent()
        .args(["format", dir.to_str().unwrap(), "--check"])
        .assert()
        .success();
}

// ── M12: watch mode (no-feature build) ───────────────────────────

#[test]
fn watch_flag_without_feature_exits_with_message() {
    let (_parent, dir) = make_skill_dir(
        "watch-test",
        "---\nname: watch-test\ndescription: Testing watch\n---\nBody.\n",
    );
    aigent()
        .args(["validate", dir.to_str().unwrap(), "--watch"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("watch"));
}

// ── M11: build --interactive flag ─────────────────────────────────

#[test]
fn new_interactive_flag_accepted() {
    // Just verify the flag is accepted (with piped "n" to cancel).
    let parent = tempdir().unwrap();
    let dir = parent.path().join("interactive-cli");
    aigent()
        .args([
            "new",
            "Process PDF files and extract text from documents",
            "--no-llm",
            "--name",
            "interactive-cli",
            "--dir",
            dir.to_str().unwrap(),
            "--interactive",
        ])
        .write_stdin("n\n")
        .assert()
        .failure()
        .stderr(predicate::str::contains("cancelled"));
}

// ── M13: test subcommand (fixture-based) ─────────────────────────

#[test]
fn test_run_suite_all_pass() {
    let (_parent, dir) = make_skill_dir(
        "test-suite-pass",
        "---\nname: test-suite-pass\ndescription: Processes PDF files and generates reports. Use when working with documents.\n---\nBody.\n",
    );
    fs::write(
        dir.join("tests.yml"),
        "queries:\n  - input: \"process PDF files\"\n    should_match: true\n  - input: \"deploy kubernetes\"\n    should_match: false\n",
    )
    .unwrap();
    aigent()
        .args(["test", dir.to_str().unwrap()])
        .assert()
        .success()
        .stderr(predicate::str::contains("2 passed, 0 failed"));
}

#[test]
fn test_run_suite_with_failure() {
    let (_parent, dir) = make_skill_dir(
        "test-suite-fail",
        "---\nname: test-suite-fail\ndescription: Processes PDF files and generates reports. Use when working with documents.\n---\nBody.\n",
    );
    fs::write(
        dir.join("tests.yml"),
        "queries:\n  - input: \"process PDF files\"\n    should_match: false\n",
    )
    .unwrap();
    aigent()
        .args(["test", dir.to_str().unwrap()])
        .assert()
        .failure()
        .stderr(predicate::str::contains("0 passed, 1 failed"));
}

#[test]
fn test_missing_tests_yml_exits_nonzero() {
    let (_parent, dir) = make_skill_dir(
        "test-no-fixture",
        "---\nname: test-no-fixture\ndescription: Does things\n---\nBody.\n",
    );
    aigent()
        .args(["test", dir.to_str().unwrap()])
        .assert()
        .failure()
        .stderr(predicate::str::contains("no tests.yml"));
}

#[test]
fn test_generate_creates_tests_yml() {
    let (_parent, dir) = make_skill_dir(
        "test-gen",
        "---\nname: test-gen\ndescription: Processes documents. Use when handling files.\n---\nBody.\n",
    );
    aigent()
        .args(["test", dir.to_str().unwrap(), "--generate"])
        .assert()
        .success()
        .stderr(predicate::str::contains("Generated"));
    let fixture_path = dir.join("tests.yml");
    assert!(fixture_path.exists());
    let content = fs::read_to_string(&fixture_path).unwrap();
    assert!(content.contains("queries:"));
    assert!(content.contains("should_match: true"));
    assert!(content.contains("should_match: false"));
}

#[test]
fn test_generate_skips_existing_tests_yml() {
    let (_parent, dir) = make_skill_dir(
        "test-gen-exists",
        "---\nname: test-gen-exists\ndescription: Does things\n---\nBody.\n",
    );
    fs::write(dir.join("tests.yml"), "queries: []\n").unwrap();
    aigent()
        .args(["test", dir.to_str().unwrap(), "--generate"])
        .assert()
        .success()
        .stderr(predicate::str::contains("already exists"));
    // Content should be unchanged.
    let content = fs::read_to_string(dir.join("tests.yml")).unwrap();
    assert_eq!(content, "queries: []\n");
}

#[test]
fn test_json_format_outputs_suite_result() {
    let (_parent, dir) = make_skill_dir(
        "test-json-suite",
        "---\nname: test-json-suite\ndescription: Processes PDF files and generates reports. Use when working with documents.\n---\nBody.\n",
    );
    fs::write(
        dir.join("tests.yml"),
        "queries:\n  - input: \"process PDF files\"\n    should_match: true\n",
    )
    .unwrap();
    let output = aigent()
        .args(["test", dir.to_str().unwrap(), "--format", "json"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(json["passed"], 1);
    assert_eq!(json["failed"], 0);
}

// ── Default directory (#116) ────────────────────────────────────────

#[test]
fn validate_defaults_to_current_dir() {
    let (_parent, dir) = make_skill_dir(
        "my-skill",
        "---\nname: my-skill\ndescription: Processes PDF files and generates reports. Use when working with documents.\n---\nBody.\n",
    );
    aigent()
        .arg("validate")
        .current_dir(&dir)
        .assert()
        .success();
}

#[test]
fn validate_explicit_path_still_works() {
    let (_parent, dir) = make_skill_dir(
        "my-skill",
        "---\nname: my-skill\ndescription: Processes PDF files and generates reports. Use when working with documents.\n---\nBody.\n",
    );
    aigent()
        .args(["validate", dir.to_str().unwrap()])
        .assert()
        .success();
}

#[test]
fn properties_defaults_to_current_dir() {
    let (_parent, dir) = make_skill_dir(
        "my-skill",
        "---\nname: my-skill\ndescription: Processes PDF files and generates reports. Use when working with documents.\n---\nBody.\n",
    );
    aigent()
        .arg("properties")
        .current_dir(&dir)
        .assert()
        .success()
        .stdout(predicate::str::contains("\"name\": \"my-skill\""));
}

#[test]
fn score_defaults_to_current_dir() {
    let (_parent, dir) = make_skill_dir(
        "my-skill",
        "---\nname: my-skill\ndescription: Processes PDF files and generates reports. Use when working with documents.\n---\nBody.\n",
    );
    aigent()
        .arg("score")
        .current_dir(&dir)
        .assert()
        .stderr(predicate::str::contains("Score:"));
}

#[test]
fn fmt_check_defaults_to_current_dir() {
    let (_parent, dir) = make_skill_dir(
        "my-skill",
        "---\nname: my-skill\ndescription: Processes PDF files and generates reports. Use when working with documents.\n---\nBody.\n",
    );
    aigent()
        .args(["fmt", "--check"])
        .current_dir(&dir)
        .assert()
        .success();
}

#[test]
fn check_defaults_to_current_dir() {
    let (_parent, dir) = make_skill_dir(
        "my-skill",
        "---\nname: my-skill\ndescription: Processes PDF files and generates reports. Use when working with documents.\n---\nBody.\n",
    );
    aigent().arg("check").current_dir(&dir).assert().success();
}

#[test]
fn validate_multiple_explicit_paths_no_default_prepended() {
    let parent = tempdir().unwrap();
    let dir_a = parent.path().join("skill-a");
    let dir_b = parent.path().join("skill-b");
    fs::create_dir(&dir_a).unwrap();
    fs::create_dir(&dir_b).unwrap();
    fs::write(
        dir_a.join("SKILL.md"),
        "---\nname: skill-a\ndescription: Processes PDF files and generates reports. Use when working with documents.\n---\nBody.\n",
    )
    .unwrap();
    fs::write(
        dir_b.join("SKILL.md"),
        "---\nname: skill-b\ndescription: Manages database connections and queries. Use when working with databases.\n---\nBody.\n",
    )
    .unwrap();
    // When explicit paths are given, the default "." must NOT be prepended
    aigent()
        .args(["validate", dir_a.to_str().unwrap(), dir_b.to_str().unwrap()])
        .assert()
        .success();
}

#[test]
fn probe_no_skill_in_cwd_errors_gracefully() {
    let empty_dir = tempdir().unwrap();
    aigent()
        .args(["probe", "--query", "some query"])
        .current_dir(empty_dir.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("aigent probe:"));
}

#[test]
fn probe_multiple_dirs_json_returns_array() {
    let parent = tempdir().unwrap();
    let dir_a = parent.path().join("skill-a");
    let dir_b = parent.path().join("skill-b");
    fs::create_dir(&dir_a).unwrap();
    fs::create_dir(&dir_b).unwrap();
    fs::write(
        dir_a.join("SKILL.md"),
        "---\nname: skill-a\ndescription: Processes PDF files and extracts text\n---\nBody.\n",
    )
    .unwrap();
    fs::write(
        dir_b.join("SKILL.md"),
        "---\nname: skill-b\ndescription: Manages database connections\n---\nBody.\n",
    )
    .unwrap();
    let output = aigent()
        .args([
            "probe",
            dir_a.to_str().unwrap(),
            dir_b.to_str().unwrap(),
            "--query",
            "process PDF files",
            "--format",
            "json",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert!(json.is_array(), "multi-dir JSON should be an array");
    assert_eq!(json.as_array().unwrap().len(), 2);
}

#[test]
fn probe_multiple_dirs_ranked_by_score() {
    let parent = tempdir().unwrap();
    let dir_a = parent.path().join("skill-a");
    let dir_b = parent.path().join("skill-b");
    fs::create_dir(&dir_a).unwrap();
    fs::create_dir(&dir_b).unwrap();
    fs::write(
        dir_a.join("SKILL.md"),
        "---\nname: skill-a\ndescription: Manages database connections\n---\nBody.\n",
    )
    .unwrap();
    fs::write(
        dir_b.join("SKILL.md"),
        "---\nname: skill-b\ndescription: Processes PDF files and extracts text\n---\nBody.\n",
    )
    .unwrap();
    let output = aigent()
        .args([
            "probe",
            dir_a.to_str().unwrap(),
            dir_b.to_str().unwrap(),
            "--query",
            "process PDF files",
            "--format",
            "json",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let arr = json.as_array().unwrap();
    // skill-b (PDF) should rank higher than skill-a (database)
    assert_eq!(arr[0]["name"], "skill-b");
    assert_eq!(arr[1]["name"], "skill-a");
}

// ── validate-plugin ──────────────────────────────────────────────

/// Helper: write a plugin.json in a temp dir and return (dir, path).
fn make_plugin_dir(content: &str) -> (tempfile::TempDir, PathBuf) {
    let dir = tempdir().unwrap();
    let path = dir.path().to_path_buf();
    fs::write(path.join("plugin.json"), content).unwrap();
    (dir, path)
}

#[test]
fn validate_plugin_valid_manifest() {
    let (_dir, path) = make_plugin_dir(
        r#"{ "name": "my-plugin", "description": "A test plugin", "version": "1.0.0", "author": "Test", "homepage": "https://example.com", "license": "MIT" }"#,
    );
    aigent()
        .args(["validate-plugin", path.to_str().unwrap()])
        .assert()
        .success()
        .stderr(predicate::str::contains("Plugin validation passed"));
}

#[test]
fn validate_plugin_missing_plugin_json() {
    let dir = tempdir().unwrap();
    aigent()
        .args(["validate-plugin", dir.path().to_str().unwrap()])
        .assert()
        .failure()
        .stderr(predicate::str::contains("cannot read plugin.json"));
}

#[test]
fn validate_plugin_invalid_json() {
    let (_dir, path) = make_plugin_dir("{ not json }");
    aigent()
        .args(["validate-plugin", path.to_str().unwrap()])
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid JSON syntax"));
}

#[test]
fn validate_plugin_missing_name() {
    let (_dir, path) = make_plugin_dir(r#"{ "description": "test" }"#);
    aigent()
        .args(["validate-plugin", path.to_str().unwrap()])
        .assert()
        .failure()
        .stderr(predicate::str::contains("missing required field `name`"));
}

#[test]
fn validate_plugin_invalid_name() {
    let (_dir, path) = make_plugin_dir(r#"{ "name": "My Plugin" }"#);
    aigent()
        .args(["validate-plugin", path.to_str().unwrap()])
        .assert()
        .failure()
        .stderr(predicate::str::contains("not valid kebab-case"));
}

#[test]
fn validate_plugin_bad_version() {
    let (_dir, path) = make_plugin_dir(r#"{ "name": "test", "version": "1.0" }"#);
    aigent()
        .args(["validate-plugin", path.to_str().unwrap()])
        .assert()
        .success() // P004 is a warning, not an error
        .stderr(predicate::str::contains("not valid semver"));
}

#[test]
fn validate_plugin_json_format() {
    let (_dir, path) = make_plugin_dir(r#"{ "name": "test" }"#);
    let output = aigent()
        .args([
            "validate-plugin",
            path.to_str().unwrap(),
            "--format",
            "json",
        ])
        .output()
        .unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let arr = json.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    let diags = arr[0]["diagnostics"].as_array().unwrap();
    // Should have at least P005 (missing description) and P010s (missing recommended fields)
    assert!(diags.iter().any(|d| d["code"] == "P005"));
}

#[test]
fn validate_plugin_defaults_to_current_dir() {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("plugin.json"),
        r#"{ "name": "my-plugin", "description": "test", "author": "x", "homepage": "x", "license": "MIT" }"#,
    )
    .unwrap();
    aigent()
        .arg("validate-plugin")
        .current_dir(dir.path())
        .assert()
        .success()
        .stderr(predicate::str::contains("Plugin validation passed"));
}

#[test]
fn validate_plugin_credential_detection() {
    let (_dir, path) =
        make_plugin_dir(r#"{ "name": "test", "config": { "value": "api_key: 'sk-1234abcd'" } }"#);
    aigent()
        .args(["validate-plugin", path.to_str().unwrap()])
        .assert()
        .failure()
        .stderr(predicate::str::contains("credential"));
}

#[test]
fn validate_plugin_discovers_hooks() {
    let dir = tempdir().unwrap();
    let path = dir.path();
    fs::write(
        path.join("plugin.json"),
        r#"{ "name": "test", "description": "t", "author": "x", "homepage": "x", "license": "MIT" }"#,
    )
    .unwrap();
    fs::write(
        path.join("hooks.json"),
        r#"{ "BadEvent": [{ "hooks": [{ "type": "command", "command": "echo" }] }] }"#,
    )
    .unwrap();
    aigent()
        .args(["validate-plugin", path.to_str().unwrap()])
        .assert()
        .failure()
        .stderr(predicate::str::contains("unknown event name"));
}

#[test]
fn validate_plugin_discovers_agents() {
    let dir = tempdir().unwrap();
    let path = dir.path();
    fs::write(
        path.join("plugin.json"),
        r#"{ "name": "test", "description": "t", "author": "x", "homepage": "x", "license": "MIT" }"#,
    )
    .unwrap();
    let agents_dir = path.join("agents");
    fs::create_dir(&agents_dir).unwrap();
    fs::write(
        agents_dir.join("bad.md"),
        "---\nname: helper\nmodel: gpt-4\ncolor: orange\n---\nShort.\n",
    )
    .unwrap();
    aigent()
        .args(["validate-plugin", path.to_str().unwrap()])
        .assert()
        .failure()
        .stderr(predicate::str::contains("agents/bad.md"))
        .stderr(predicate::str::contains("`model` is not valid"));
}

#[test]
fn validate_plugin_discovers_commands() {
    let dir = tempdir().unwrap();
    let path = dir.path();
    fs::write(
        path.join("plugin.json"),
        r#"{ "name": "test", "description": "t", "author": "x", "homepage": "x", "license": "MIT" }"#,
    )
    .unwrap();
    let cmds_dir = path.join("commands");
    fs::create_dir(&cmds_dir).unwrap();
    fs::write(cmds_dir.join("empty.md"), "---\nmodel: haiku\n---\n").unwrap();
    aigent()
        .args(["validate-plugin", path.to_str().unwrap()])
        .assert()
        .failure()
        .stderr(predicate::str::contains("commands/empty.md"))
        .stderr(predicate::str::contains("command body is empty"));
}

#[test]
fn validate_plugin_json_includes_all_components() {
    let dir = tempdir().unwrap();
    let path = dir.path();
    fs::write(
        path.join("plugin.json"),
        r#"{ "name": "test", "description": "t", "author": "x", "homepage": "x", "license": "MIT" }"#,
    )
    .unwrap();
    fs::write(
        path.join("hooks.json"),
        r#"{ "PreToolUse": [{ "hooks": [{ "type": "command", "command": "echo" }] }] }"#,
    )
    .unwrap();
    let agents_dir = path.join("agents");
    fs::create_dir(&agents_dir).unwrap();
    fs::write(
        agents_dir.join("reviewer.md"),
        "---\nname: code-reviewer\ndescription: Reviews code for quality\nmodel: sonnet\ncolor: blue\n---\nYou review code carefully and provide helpful feedback to improve quality.\n",
    )
    .unwrap();
    let output = aigent()
        .args([
            "validate-plugin",
            path.to_str().unwrap(),
            "--format",
            "json",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let arr = json.as_array().unwrap();
    // Should have plugin.json, hooks.json, and agents/reviewer.md
    assert!(
        arr.len() >= 3,
        "expected at least 3 entries, got {}",
        arr.len()
    );
    let paths: Vec<&str> = arr.iter().map(|e| e["path"].as_str().unwrap()).collect();
    assert!(paths.contains(&"plugin.json"));
    assert!(paths.contains(&"hooks.json"));
    assert!(paths.iter().any(|p| p.starts_with("agents/")));
}
