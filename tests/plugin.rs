use std::fs;
use std::path::Path;

use assert_cmd::cargo::cargo_bin_cmd;
use assert_cmd::Command;

/// Return a `Command` for the `aigent` binary built by Cargo.
fn aigent() -> Command {
    cargo_bin_cmd!("aigent")
}

// ── Self-validation tests ───────────────────────────────────────────

#[test]
fn validate_builder_skill() {
    aigent()
        .args(["validate", "skills/aigent-builder/"])
        .assert()
        .success();
}

#[test]
fn validate_validator_skill() {
    aigent()
        .args(["validate", "skills/aigent-validator/"])
        .assert()
        .success();
}

// ── Plugin manifest tests ───────────────────────────────────────────

fn read_plugin_json() -> serde_json::Value {
    let content =
        fs::read_to_string(".claude-plugin/plugin.json").expect("plugin.json should exist");
    serde_json::from_str(&content).expect("plugin.json should be valid JSON")
}

#[test]
fn plugin_json_is_valid() {
    read_plugin_json();
}

#[test]
fn plugin_json_has_name() {
    let json = read_plugin_json();
    assert_eq!(json["name"], "aigent");
}

#[test]
fn plugin_json_has_version() {
    let json = read_plugin_json();
    let version = json["version"]
        .as_str()
        .expect("version should be a string");
    assert!(!version.is_empty(), "version should be non-empty");
}

#[test]
fn plugin_json_has_description() {
    let json = read_plugin_json();
    let desc = json["description"]
        .as_str()
        .expect("description should be a string");
    assert!(!desc.is_empty(), "description should be non-empty");
}

// ── Version sync test ───────────────────────────────────────────────

#[test]
fn plugin_version_matches_cargo_version() {
    let json = read_plugin_json();
    let plugin_version = json["version"].as_str().unwrap();
    let cargo_version = env!("CARGO_PKG_VERSION");
    assert_eq!(
        plugin_version, cargo_version,
        "plugin.json version ({plugin_version}) must match Cargo.toml version ({cargo_version})"
    );
}

// ── Skill content tests ─────────────────────────────────────────────

#[test]
fn builder_skill_has_allowed_tools() {
    let props = aigent::read_properties(Path::new("skills/aigent-builder")).unwrap();
    assert!(
        props.allowed_tools.is_some(),
        "builder skill should have allowed-tools"
    );
}

#[test]
fn validator_skill_has_allowed_tools() {
    let props = aigent::read_properties(Path::new("skills/aigent-validator")).unwrap();
    assert!(
        props.allowed_tools.is_some(),
        "validator skill should have allowed-tools"
    );
}

#[test]
fn builder_skill_name_matches_directory() {
    let props = aigent::read_properties(Path::new("skills/aigent-builder")).unwrap();
    assert_eq!(props.name, "aigent-builder");
}

#[test]
fn validator_skill_name_matches_directory() {
    let props = aigent::read_properties(Path::new("skills/aigent-validator")).unwrap();
    assert_eq!(props.name, "aigent-validator");
}

// ── Install script tests ────────────────────────────────────────────

#[test]
fn install_script_exists_and_executable() {
    let path = Path::new("install.sh");
    assert!(path.exists(), "install.sh should exist");

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = fs::metadata(path).unwrap().permissions();
        assert!(perms.mode() & 0o111 != 0, "install.sh should be executable");
    }
}

#[test]
fn install_script_has_posix_shebang() {
    let content = fs::read_to_string("install.sh").unwrap();
    assert!(
        content.starts_with("#!/bin/sh"),
        "install.sh should start with #!/bin/sh"
    );
}
