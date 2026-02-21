//! Directory structure validation for skill packages.
//!
//! Extends validation beyond frontmatter to check the skill's directory
//! structure: file references in the markdown body, script permissions,
//! reference depth, and nesting depth.
//!
//! Structure diagnostics use codes S001–S005. Most are `Severity::Warning`
//! unless the issue would cause a broken skill at runtime. S005 (symlink
//! detected) uses `Severity::Info`.

use std::path::Path;
use std::sync::LazyLock;

use regex::Regex;

use crate::diagnostics::{Diagnostic, Severity, S001, S003, S004, S005, S006};
use crate::fs_util::{is_regular_dir, is_symlink};

#[cfg(unix)]
use crate::diagnostics::S002;
#[cfg(unix)]
use crate::fs_util::is_regular_file;

/// Maximum allowed nesting depth for files referenced from SKILL.md.
const MAX_REFERENCE_DEPTH: usize = 1;

/// Maximum allowed subdirectory nesting depth within a skill directory.
const MAX_NESTING_DEPTH: usize = 2;

/// Regex for markdown links and images: `[text](path)` and `![alt](path)`.
///
/// Captures the path in group 1. Excludes URLs (http:// or https://) and
/// anchors (#fragment).
static LINK_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"!?\[(?:[^\]]*)\]\((?P<path>[^)]+)\)").expect("link regex must compile")
});

/// Validate the directory structure of a skill package.
///
/// Checks:
/// - S001: Referenced files in the markdown body actually exist
/// - S002: Scripts (.sh) have execute permission (Unix only)
/// - S003: File references exceed 1 level of depth
/// - S004: Excessive directory nesting depth
/// - S005: Symlink detected in skill directory (Info)
/// - S006: Path traversal in reference link
///
/// # Arguments
///
/// * `dir` - Path to the skill directory
///
/// # Returns
///
/// A list of diagnostics. Empty means the structure is valid.
#[must_use]
pub fn validate_structure(dir: &Path) -> Vec<Diagnostic> {
    let mut diags = Vec::new();

    // Read the SKILL.md body for reference checking.
    let body = crate::parser::read_body(dir).unwrap_or_default();

    // S001 + S003 + S006: Check file references in the body.
    diags.extend(check_references(dir, &body));

    // S002: Check script permissions.
    diags.extend(check_script_permissions(dir));

    // S004: Check directory nesting depth.
    diags.extend(check_nesting_depth(dir));

    // S005: Check for symlinks.
    diags.extend(check_symlinks(dir));

    diags
}

/// Returns `true` if the given path string contains `..` (parent directory) components.
fn contains_path_traversal(path: &str) -> bool {
    Path::new(path)
        .components()
        .any(|c| c == std::path::Component::ParentDir)
}

/// S001 + S003 + S006: Check file references in the markdown body.
///
/// Extracts `[text](path)` and `![alt](path)` patterns, skipping URLs
/// and anchors. Reports S006 if the path contains `..` traversal components,
/// S003 if the reference path exceeds `MAX_REFERENCE_DEPTH` levels, and
/// S001 if the referenced file doesn't exist.
fn check_references(dir: &Path, body: &str) -> Vec<Diagnostic> {
    let mut diags = Vec::new();

    for cap in LINK_RE.captures_iter(body) {
        let path_str = &cap["path"];

        // Skip URLs and anchors.
        if path_str.starts_with("http://")
            || path_str.starts_with("https://")
            || path_str.starts_with('#')
        {
            continue;
        }

        // Strip fragment from path (e.g., "file.md#section").
        let clean_path = path_str.split('#').next().unwrap_or(path_str);

        // Check for path traversal (S006).
        if contains_path_traversal(clean_path) {
            diags.push(
                Diagnostic::new(
                    Severity::Warning,
                    S006,
                    format!("path traversal in reference link: '{clean_path}'"),
                )
                .with_field("body")
                .with_suggestion(
                    "Remove '..' components — references must stay within the skill directory",
                ),
            );
            continue;
        }

        // Check reference depth (S003).
        let depth = clean_path.matches('/').count();
        if depth > MAX_REFERENCE_DEPTH {
            diags.push(
                Diagnostic::new(
                    Severity::Warning,
                    S003,
                    format!(
                        "reference depth exceeds {} level(s): '{clean_path}'",
                        MAX_REFERENCE_DEPTH
                    ),
                )
                .with_field("body")
                .with_suggestion("Keep referenced files at most one directory level deep"),
            );
        }

        // Check file existence (S001).
        let full_path = dir.join(clean_path);
        if !full_path.exists() {
            diags.push(
                Diagnostic::new(
                    Severity::Warning,
                    S001,
                    format!("referenced file does not exist: '{clean_path}'"),
                )
                .with_field("body")
                .with_suggestion(format!(
                    "Create the file or fix the reference path: '{clean_path}'"
                )),
            );
        }
    }

    diags
}

/// S002: Check that scripts (.sh) have execute permission.
///
/// Only checked on Unix systems. On non-Unix platforms, this check is
/// skipped entirely.
fn check_script_permissions(dir: &Path) -> Vec<Diagnostic> {
    check_script_permissions_impl(dir)
}

/// Platform-specific script permission check.
#[cfg(unix)]
fn check_script_permissions_impl(dir: &Path) -> Vec<Diagnostic> {
    use std::os::unix::fs::PermissionsExt;

    let mut diags = Vec::new();
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return diags,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if is_regular_file(&path) {
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                if ext.eq_ignore_ascii_case("sh") {
                    if let Ok(metadata) = std::fs::metadata(&path) {
                        let mode = metadata.permissions().mode();
                        if mode & 0o111 == 0 {
                            let name = path
                                .file_name()
                                .and_then(|n| n.to_str())
                                .unwrap_or("unknown");
                            diags.push(
                                Diagnostic::new(
                                    Severity::Warning,
                                    S002,
                                    format!("script missing execute permission: '{name}'"),
                                )
                                .with_field("structure")
                                .with_suggestion(format!("Run: chmod +x {name}")),
                            );
                        }
                    }
                }
            }
        }
    }
    diags
}

/// Non-Unix stub: script permission checks are not applicable.
#[cfg(not(unix))]
fn check_script_permissions_impl(_dir: &Path) -> Vec<Diagnostic> {
    Vec::new()
}

/// S004: Check for excessive directory nesting depth.
///
/// Walks the directory tree and reports if any subdirectory exceeds
/// `MAX_NESTING_DEPTH` levels relative to the skill directory root.
fn check_nesting_depth(dir: &Path) -> Vec<Diagnostic> {
    let mut diags = Vec::new();
    check_nesting_recursive(dir, dir, 0, &mut diags);
    diags
}

/// Recursive helper for nesting depth check.
fn check_nesting_recursive(root: &Path, current: &Path, depth: usize, diags: &mut Vec<Diagnostic>) {
    if depth > MAX_NESTING_DEPTH {
        let relative = current
            .strip_prefix(root)
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| current.display().to_string());
        diags.push(
            Diagnostic::new(
                Severity::Warning,
                S004,
                format!("excessive nesting depth ({depth} levels): '{relative}'"),
            )
            .with_field("structure")
            .with_suggestion(format!(
                "Keep directory depth to at most {MAX_NESTING_DEPTH} levels"
            )),
        );
        return; // Don't recurse deeper once flagged.
    }

    let entries = match std::fs::read_dir(current) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if is_regular_dir(&path) {
            // Skip hidden directories.
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name.starts_with('.') {
                    continue;
                }
            }
            check_nesting_recursive(root, &path, depth + 1, diags);
        }
    }
}

/// S005: Check for symlinks in the skill directory.
///
/// Walks the directory entries and reports any symlinks found. Symlinks are
/// an informational finding (not an error) because they can be used to escape
/// the skill directory boundary.
fn check_symlinks(dir: &Path) -> Vec<Diagnostic> {
    let mut diags = Vec::new();
    check_symlinks_recursive(dir, dir, &mut diags);
    diags
}

/// Recursive helper for symlink detection.
fn check_symlinks_recursive(root: &Path, current: &Path, diags: &mut Vec<Diagnostic>) {
    let entries = match std::fs::read_dir(current) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();

        // Skip hidden entries.
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            if name.starts_with('.') {
                continue;
            }
        }

        if is_symlink(&path) {
            let relative = path
                .strip_prefix(root)
                .map(|p| p.display().to_string())
                .unwrap_or_else(|_| path.display().to_string());
            diags.push(
                Diagnostic::new(
                    Severity::Info,
                    S005,
                    format!("symlink detected: '{relative}'"),
                )
                .with_field("structure")
                .with_suggestion(
                    "Remove symlinks from skill directories to prevent directory escape",
                ),
            );
        } else if is_regular_dir(&path) {
            check_symlinks_recursive(root, &path, diags);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    /// Create a skill directory with SKILL.md content.
    fn make_skill(name: &str, content: &str) -> (tempfile::TempDir, std::path::PathBuf) {
        let parent = tempdir().unwrap();
        let dir = parent.path().join(name);
        fs::create_dir(&dir).unwrap();
        fs::write(dir.join("SKILL.md"), content).unwrap();
        (parent, dir)
    }

    // ── S001: Referenced file does not exist ─────────────────────────

    #[test]
    fn s001_missing_file_reference() {
        let (_parent, dir) = make_skill(
            "my-skill",
            "---\nname: my-skill\ndescription: desc\n---\n\nSee [guide](guide.md) for details.\n",
        );
        let diags = validate_structure(&dir);
        assert!(
            diags.iter().any(|d| d.code == S001),
            "expected S001 for missing guide.md, got: {diags:?}",
        );
    }

    #[test]
    fn s001_existing_file_no_diagnostic() {
        let (_parent, dir) = make_skill(
            "my-skill",
            "---\nname: my-skill\ndescription: desc\n---\n\nSee [guide](guide.md) for details.\n",
        );
        fs::write(dir.join("guide.md"), "# Guide").unwrap();
        let diags = validate_structure(&dir);
        assert!(
            !diags.iter().any(|d| d.code == S001),
            "expected no S001 when file exists, got: {diags:?}",
        );
    }

    #[test]
    fn s001_url_references_skipped() {
        let (_parent, dir) = make_skill(
            "my-skill",
            "---\nname: my-skill\ndescription: desc\n---\n\nSee [docs](https://example.com) for details.\n",
        );
        let diags = validate_structure(&dir);
        assert!(
            !diags.iter().any(|d| d.code == S001),
            "URL references should be skipped, got: {diags:?}",
        );
    }

    #[test]
    fn s001_anchor_references_skipped() {
        let (_parent, dir) = make_skill(
            "my-skill",
            "---\nname: my-skill\ndescription: desc\n---\n\nSee [section](#usage) for details.\n",
        );
        let diags = validate_structure(&dir);
        assert!(
            !diags.iter().any(|d| d.code == S001),
            "anchor references should be skipped, got: {diags:?}",
        );
    }

    #[test]
    fn s001_image_reference_checked() {
        let (_parent, dir) = make_skill(
            "my-skill",
            "---\nname: my-skill\ndescription: desc\n---\n\n![diagram](arch.png)\n",
        );
        let diags = validate_structure(&dir);
        assert!(
            diags.iter().any(|d| d.code == S001),
            "expected S001 for missing arch.png, got: {diags:?}",
        );
    }

    // ── S002: Script missing execute permission ──────────────────────

    #[cfg(unix)]
    #[test]
    fn s002_script_without_execute() {
        let (_parent, dir) =
            make_skill("my-skill", "---\nname: my-skill\ndescription: desc\n---\n");
        let script = dir.join("setup.sh");
        fs::write(&script, "#!/bin/bash\necho hello").unwrap();
        // Remove execute permission.
        let mut perms = fs::metadata(&script).unwrap().permissions();
        use std::os::unix::fs::PermissionsExt;
        perms.set_mode(0o644);
        fs::set_permissions(&script, perms).unwrap();

        let diags = validate_structure(&dir);
        assert!(
            diags.iter().any(|d| d.code == S002),
            "expected S002 for non-executable script, got: {diags:?}",
        );
    }

    #[cfg(unix)]
    #[test]
    fn s002_script_with_execute_no_diagnostic() {
        let (_parent, dir) =
            make_skill("my-skill", "---\nname: my-skill\ndescription: desc\n---\n");
        let script = dir.join("setup.sh");
        fs::write(&script, "#!/bin/bash\necho hello").unwrap();
        let mut perms = fs::metadata(&script).unwrap().permissions();
        use std::os::unix::fs::PermissionsExt;
        perms.set_mode(0o755);
        fs::set_permissions(&script, perms).unwrap();

        let diags = validate_structure(&dir);
        assert!(
            !diags.iter().any(|d| d.code == S002),
            "expected no S002 for executable script, got: {diags:?}",
        );
    }

    // ── S003: Reference depth exceeds 1 level ────────────────────────

    #[test]
    fn s003_deep_reference() {
        let (_parent, dir) = make_skill(
            "my-skill",
            "---\nname: my-skill\ndescription: desc\n---\n\nSee [doc](sub/deep/guide.md)\n",
        );
        // Create the file so we only get S003, not S001.
        fs::create_dir_all(dir.join("sub/deep")).unwrap();
        fs::write(dir.join("sub/deep/guide.md"), "# Guide").unwrap();

        let diags = validate_structure(&dir);
        assert!(
            diags.iter().any(|d| d.code == S003),
            "expected S003 for deep reference, got: {diags:?}",
        );
    }

    #[test]
    fn s003_one_level_reference_ok() {
        let (_parent, dir) = make_skill(
            "my-skill",
            "---\nname: my-skill\ndescription: desc\n---\n\nSee [doc](sub/guide.md)\n",
        );
        fs::create_dir(dir.join("sub")).unwrap();
        fs::write(dir.join("sub/guide.md"), "# Guide").unwrap();

        let diags = validate_structure(&dir);
        assert!(
            !diags.iter().any(|d| d.code == S003),
            "one-level reference should be ok, got: {diags:?}",
        );
    }

    #[test]
    fn s003_same_level_reference_ok() {
        let (_parent, dir) = make_skill(
            "my-skill",
            "---\nname: my-skill\ndescription: desc\n---\n\nSee [doc](guide.md)\n",
        );
        fs::write(dir.join("guide.md"), "# Guide").unwrap();

        let diags = validate_structure(&dir);
        assert!(
            !diags.iter().any(|d| d.code == S003),
            "same-level reference should be ok, got: {diags:?}",
        );
    }

    // ── S004: Excessive nesting depth ────────────────────────────────

    #[test]
    fn s004_excessive_nesting() {
        let (_parent, dir) =
            make_skill("my-skill", "---\nname: my-skill\ndescription: desc\n---\n");
        // Create depth 3: my-skill/a/b/c
        fs::create_dir_all(dir.join("a/b/c")).unwrap();

        let diags = validate_structure(&dir);
        assert!(
            diags.iter().any(|d| d.code == S004),
            "expected S004 for depth 3, got: {diags:?}",
        );
    }

    #[test]
    fn s004_allowed_nesting_no_diagnostic() {
        let (_parent, dir) =
            make_skill("my-skill", "---\nname: my-skill\ndescription: desc\n---\n");
        // Create depth 2: my-skill/a/b (allowed)
        fs::create_dir_all(dir.join("a/b")).unwrap();

        let diags = validate_structure(&dir);
        assert!(
            !diags.iter().any(|d| d.code == S004),
            "depth 2 should be ok, got: {diags:?}",
        );
    }

    #[test]
    fn s004_hidden_dirs_skipped() {
        let (_parent, dir) =
            make_skill("my-skill", "---\nname: my-skill\ndescription: desc\n---\n");
        fs::create_dir_all(dir.join(".hidden/a/b/c")).unwrap();

        let diags = validate_structure(&dir);
        assert!(
            !diags.iter().any(|d| d.code == S004),
            "hidden directories should be skipped, got: {diags:?}",
        );
    }

    // ── S005: Symlink detected ───────────────────────────────────────

    #[cfg(unix)]
    #[test]
    fn s005_symlink_detected() {
        let (_parent, dir) =
            make_skill("my-skill", "---\nname: my-skill\ndescription: desc\n---\n");
        // Create a regular file and a symlink to it inside the skill dir.
        let target = dir.join("real.txt");
        fs::write(&target, "content").unwrap();
        let link = dir.join("link.txt");
        std::os::unix::fs::symlink(&target, &link).unwrap();

        let diags = validate_structure(&dir);
        assert!(
            diags.iter().any(|d| d.code == S005),
            "expected S005 for symlink, got: {diags:?}",
        );
    }

    #[cfg(unix)]
    #[test]
    fn s005_symlink_is_info_severity() {
        let (_parent, dir) =
            make_skill("my-skill", "---\nname: my-skill\ndescription: desc\n---\n");
        let target = dir.join("real.txt");
        fs::write(&target, "content").unwrap();
        let link = dir.join("link.txt");
        std::os::unix::fs::symlink(&target, &link).unwrap();

        let diags = validate_structure(&dir);
        let s005_diags: Vec<_> = diags.iter().filter(|d| d.code == S005).collect();
        assert!(!s005_diags.is_empty(), "expected S005 diagnostic",);
        assert!(
            s005_diags.iter().all(|d| d.is_info()),
            "S005 should be Info severity, got: {s005_diags:?}",
        );
    }

    #[test]
    fn s005_no_symlinks_no_diagnostic() {
        let (_parent, dir) =
            make_skill("my-skill", "---\nname: my-skill\ndescription: desc\n---\n");
        fs::write(dir.join("regular.txt"), "content").unwrap();

        let diags = validate_structure(&dir);
        assert!(
            !diags.iter().any(|d| d.code == S005),
            "expected no S005 for regular files, got: {diags:?}",
        );
    }

    // ── No SKILL.md ──────────────────────────────────────────────────

    #[test]
    fn no_skill_md_returns_empty() {
        let parent = tempdir().unwrap();
        let diags = validate_structure(parent.path());
        assert!(
            diags.is_empty(),
            "no SKILL.md should return empty diagnostics, got: {diags:?}",
        );
    }

    // ── Clean skill ──────────────────────────────────────────────────

    #[test]
    fn clean_skill_no_diagnostics() {
        let (_parent, dir) = make_skill(
            "my-skill",
            "---\nname: my-skill\ndescription: A good skill description\n---\n\n# Usage\n\nSee [guide](guide.md) for more info.\n",
        );
        fs::write(dir.join("guide.md"), "# Guide\n\nDetails here.\n").unwrap();

        let diags = validate_structure(&dir);
        assert!(
            diags.is_empty(),
            "clean skill should have no structure diagnostics, got: {diags:?}",
        );
    }

    // ── Diagnostics metadata ─────────────────────────────────────────

    #[test]
    fn diagnostics_have_fields_and_suggestions() {
        let (_parent, dir) = make_skill(
            "my-skill",
            "---\nname: my-skill\ndescription: desc\n---\n\nSee [doc](missing.md)\n",
        );
        let diags = validate_structure(&dir);
        assert!(
            diags.iter().all(|d| d.field.is_some()),
            "all structure diagnostics should have field set: {diags:?}",
        );
        assert!(
            diags.iter().all(|d| d.suggestion.is_some()),
            "all structure diagnostics should have suggestions: {diags:?}",
        );
    }

    #[test]
    fn all_structure_diagnostics_are_warnings() {
        let (_parent, dir) = make_skill(
            "my-skill",
            "---\nname: my-skill\ndescription: desc\n---\n\nSee [doc](a/b/missing.md)\n",
        );
        let diags = validate_structure(&dir);
        assert!(
            diags.iter().all(|d| d.is_warning()),
            "all structure diagnostics should be warnings: {diags:?}",
        );
    }

    // ── S006: Path traversal in reference link ───────────────────────

    #[test]
    fn s006_parent_traversal_detected() {
        let (_parent, dir) = make_skill(
            "my-skill",
            "---\nname: my-skill\ndescription: desc\n---\n\nSee [secret](../../../etc/passwd)\n",
        );
        let diags = validate_structure(&dir);
        assert!(
            diags.iter().any(|d| d.code == S006),
            "expected S006 for ../../../etc/passwd, got: {diags:?}",
        );
    }

    #[test]
    fn s006_dot_slash_no_traversal() {
        let (_parent, dir) = make_skill(
            "my-skill",
            "---\nname: my-skill\ndescription: desc\n---\n\nSee [script](./scripts/run.sh)\n",
        );
        fs::create_dir(dir.join("scripts")).unwrap();
        fs::write(dir.join("scripts/run.sh"), "#!/bin/bash").unwrap();
        let diags = validate_structure(&dir);
        assert!(
            !diags.iter().any(|d| d.code == S006),
            "expected no S006 for ./scripts/run.sh, got: {diags:?}",
        );
    }

    #[test]
    fn s006_embedded_dotdot_detected() {
        let (_parent, dir) = make_skill(
            "my-skill",
            "---\nname: my-skill\ndescription: desc\n---\n\nSee [file](sub/../file.md)\n",
        );
        let diags = validate_structure(&dir);
        assert!(
            diags.iter().any(|d| d.code == S006),
            "expected S006 for sub/../file.md, got: {diags:?}",
        );
    }

    #[test]
    fn s006_normal_relative_no_traversal() {
        let (_parent, dir) = make_skill(
            "my-skill",
            "---\nname: my-skill\ndescription: desc\n---\n\nSee [setup](scripts/setup.sh)\n",
        );
        fs::create_dir(dir.join("scripts")).unwrap();
        fs::write(dir.join("scripts/setup.sh"), "#!/bin/bash").unwrap();
        let diags = validate_structure(&dir);
        assert!(
            !diags.iter().any(|d| d.code == S006),
            "expected no S006 for scripts/setup.sh, got: {diags:?}",
        );
    }

    #[test]
    fn s006_multiple_traversals_produce_multiple_diagnostics() {
        let (_parent, dir) = make_skill(
            "my-skill",
            "---\nname: my-skill\ndescription: desc\n---\n\nSee [a](../secret.txt) and [b](../../other.txt) and [c](sub/../../../leak.md)\n",
        );
        let diags = validate_structure(&dir);
        let s006_count = diags.iter().filter(|d| d.code == S006).count();
        assert_eq!(
            s006_count, 3,
            "expected 3 S006 diagnostics, got {s006_count}: {diags:?}",
        );
    }

    #[test]
    fn s006_traversal_skips_existence_check() {
        // Path traversal should NOT also produce S001 (missing file).
        let (_parent, dir) = make_skill(
            "my-skill",
            "---\nname: my-skill\ndescription: desc\n---\n\nSee [file](../escape.md)\n",
        );
        let diags = validate_structure(&dir);
        assert!(
            diags.iter().any(|d| d.code == S006),
            "expected S006 for traversal, got: {diags:?}",
        );
        assert!(
            !diags.iter().any(|d| d.code == S001),
            "traversal path should not also trigger S001, got: {diags:?}",
        );
    }
}
