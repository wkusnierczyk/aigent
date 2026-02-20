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
        .stdout(predicate::str::contains(env!("CARGO_PKG_LICENSE")));
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
        .stderr(predicate::str::is_empty());
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
        .stderr(predicate::str::is_empty());
}

// ── read-properties ─────────────────────────────────────────────────

#[test]
fn read_properties_valid() {
    let (_parent, dir) = make_skill_dir(
        "my-skill",
        "---\nname: my-skill\ndescription: A test skill\n---\nBody.\n",
    );
    aigent()
        .args(["read-properties", dir.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"name\""))
        .stdout(predicate::str::contains("my-skill"));
}

#[test]
fn read_properties_invalid() {
    let parent = tempdir().unwrap();
    let dir = parent.path().join("no-skill");
    fs::create_dir(&dir).unwrap();
    aigent()
        .args(["read-properties", dir.to_str().unwrap()])
        .assert()
        .failure()
        .stderr(predicate::str::contains("aigent read-properties:"));
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

// ── build ──────────────────────────────────────────────────────────

#[test]
fn build_deterministic_creates_dir() {
    let parent = tempdir().unwrap();
    let dir = parent.path().join("processing-pdf-files");
    aigent()
        .args([
            "build",
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
fn build_with_name_override() {
    let parent = tempdir().unwrap();
    let dir = parent.path().join("my-pdf-tool");
    aigent()
        .args([
            "build",
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
fn build_with_dir_override() {
    let parent = tempdir().unwrap();
    let dir = parent.path().join("custom-output");
    // Use --name matching the dir name so validation passes.
    aigent()
        .args([
            "build",
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
fn built_skill_passes_validate() {
    let parent = tempdir().unwrap();
    let dir = parent.path().join("roundtrip-skill");
    // Build the skill.
    aigent()
        .args([
            "build",
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
        .stderr(predicate::str::is_empty());
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
        .stderr(predicate::str::is_empty());
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
        .stderr(predicate::str::is_empty());
}

// ── --lint flag ────────────────────────────────────────────────────

#[test]
fn validate_with_lint_shows_info_diagnostics() {
    // Name is not gerund, description has no trigger phrase and is vague.
    let (_parent, dir) = make_skill_dir(
        "helper",
        "---\nname: helper\ndescription: Helps\n---\nBody.\n",
    );
    aigent()
        .args(["validate", dir.to_str().unwrap(), "--lint"])
        .assert()
        .success() // lint never causes failure
        .stderr(predicate::str::contains("info:"));
}

#[test]
fn validate_without_lint_no_info_diagnostics() {
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

// ── lint subcommand ────────────────────────────────────────────────

#[test]
fn lint_subcommand_shows_info() {
    let (_parent, dir) = make_skill_dir(
        "helper",
        "---\nname: helper\ndescription: Helps\n---\nBody.\n",
    );
    aigent()
        .args(["lint", dir.to_str().unwrap()])
        .assert()
        .success()
        .stderr(predicate::str::contains("info:"));
}

#[test]
fn lint_subcommand_perfect_skill_no_output() {
    let (_parent, dir) = make_skill_dir(
        "processing-pdfs",
        "---\nname: processing-pdfs\ndescription: Processes PDF files and generates reports. Use when working with documents.\n---\nBody.\n",
    );
    aigent()
        .args(["lint", dir.to_str().unwrap()])
        .assert()
        .success()
        .stderr(predicate::str::is_empty());
}

#[test]
fn lint_subcommand_json_format() {
    let (_parent, dir) = make_skill_dir(
        "helper",
        "---\nname: helper\ndescription: Helps\n---\nBody.\n",
    );
    let output = aigent()
        .args(["lint", dir.to_str().unwrap(), "--format", "json"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let arr = json.as_array().unwrap();
    assert!(!arr.is_empty());
    assert!(arr.iter().all(|d| d["severity"] == "info"));
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
