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

// ── Unimplemented subcommands ───────────────────────────────────────

#[test]
fn build_not_yet_implemented() {
    aigent()
        .args(["build", "a test purpose"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("not yet implemented"));
}

#[test]
fn init_not_yet_implemented() {
    aigent()
        .args(["init"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("not yet implemented"));
}
