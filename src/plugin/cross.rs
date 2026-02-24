//! Cross-component consistency checks for plugin directories.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use crate::diagnostics::{Diagnostic, Severity, X001, X002, X003, X004, X005, X006};

/// Default token budget threshold for all skills combined.
const TOKEN_BUDGET_THRESHOLD: usize = 50_000;

/// Files that are never considered orphaned in component directories.
const IGNORED_FILES: &[&str] = &[".gitkeep", "README.md", "readme.md", ".DS_Store"];

/// A discovered component with its name and type.
#[derive(Debug)]
struct Component {
    name: String,
    kind: &'static str,
}

/// Run cross-component consistency checks on a plugin directory.
///
/// Takes the plugin root directory and returns diagnostics. Requires that
/// `plugin.json` exists and the directory structure follows conventions.
#[must_use]
pub fn validate_cross_component(root: &Path) -> Vec<Diagnostic> {
    let mut diags = Vec::new();
    let mut all_components: Vec<Component> = Vec::new();

    // Check flat component directories (agents, commands contain .md files directly)
    let flat_dirs: &[(&str, &str)] = &[("agents", "agent"), ("commands", "command")];

    for &(dir_name, kind) in flat_dirs {
        let dir = root.join(dir_name);
        if !dir.is_dir() {
            continue;
        }

        let entries: Vec<_> = match std::fs::read_dir(&dir) {
            Ok(rd) => rd.flatten().collect(),
            Err(_) => continue,
        };

        // X001: Component directory exists but contains no valid files
        let valid_files: Vec<_> = entries
            .iter()
            .filter(|e| {
                let p = e.path();
                p.is_file() && p.extension().is_some_and(|e| e == "md")
            })
            .collect();

        if valid_files.is_empty() {
            diags.push(Diagnostic::new(
                Severity::Info,
                X001,
                format!("`{dir_name}/` directory exists but contains no .md files"),
            ));
        }

        // Collect component names for X004/X006
        for f in &valid_files {
            let path = f.path();
            let stem = path
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();
            all_components.push(Component { name: stem, kind });
        }

        // X003: Orphaned files (not .md and not in ignore list)
        for entry in &entries {
            let path = entry.path();
            let file_name = path
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();

            if IGNORED_FILES.contains(&file_name.as_str()) {
                continue;
            }

            if path.is_file() && path.extension().is_none_or(|e| e != "md") {
                diags.push(
                    Diagnostic::new(
                        Severity::Warning,
                        X003,
                        format!("orphaned file in `{dir_name}/`: \"{file_name}\""),
                    )
                    .with_suggestion(format!("Remove it or convert to .md if it's a {kind} file")),
                );
            }
        }
    }

    // Check skills directory (skills are subdirectories containing SKILL.md)
    let skills_dir = root.join("skills");
    if skills_dir.is_dir() {
        if let Ok(entries) = std::fs::read_dir(&skills_dir) {
            let skill_subdirs: Vec<_> = entries.flatten().filter(|e| e.path().is_dir()).collect();

            let valid_skills: Vec<_> = skill_subdirs
                .iter()
                .filter(|e| e.path().join("SKILL.md").exists())
                .collect();

            if valid_skills.is_empty() && !skill_subdirs.is_empty() {
                diags.push(Diagnostic::new(
                    Severity::Info,
                    X001,
                    "`skills/` directory has subdirectories but none contain SKILL.md".to_string(),
                ));
            } else if skill_subdirs.is_empty() {
                diags.push(Diagnostic::new(
                    Severity::Info,
                    X001,
                    "`skills/` directory exists but contains no skill subdirectories".to_string(),
                ));
            }

            // Collect skill names from directory names
            for entry in &valid_skills {
                let name = entry
                    .path()
                    .file_name()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_default();
                all_components.push(Component {
                    name,
                    kind: "skill",
                });
            }
        }
    }

    // X002: Hook command references script that doesn't exist
    let hooks_path = root.join("hooks.json");
    if hooks_path.is_file() {
        if let Ok(content) = crate::parser::read_file_checked(&hooks_path) {
            if let Ok(raw) = serde_json::from_str::<serde_json::Value>(&content) {
                check_hook_script_paths(&raw, root, &mut diags);
            }
        }
    }

    // X004: Naming inconsistency — check if some components use kebab-case
    // and others don't
    check_naming_consistency(&all_components, &mut diags);

    // X005: Total token budget across all skills
    let skills_dir = root.join("skills");
    if skills_dir.is_dir() {
        check_token_budget(&skills_dir, &mut diags);
    }

    // X006: Duplicate component names across types
    check_duplicate_names(&all_components, &mut diags);

    diags
}

/// Check if hook commands reference scripts that don't exist on disk.
fn check_hook_script_paths(raw: &serde_json::Value, root: &Path, diags: &mut Vec<Diagnostic>) {
    let obj = match raw.as_object() {
        Some(o) => o,
        None => return,
    };

    for (_event, entries) in obj {
        let arr = match entries.as_array() {
            Some(a) => a,
            None => continue,
        };
        for entry in arr {
            let hooks = match entry.get("hooks").and_then(|h| h.as_array()) {
                Some(h) => h,
                None => continue,
            };
            for hook in hooks {
                let hook_type = hook.get("type").and_then(|t| t.as_str()).unwrap_or("");
                if hook_type != "command" {
                    continue;
                }
                let command = match hook.get("command").and_then(|c| c.as_str()) {
                    Some(c) => c,
                    None => continue,
                };

                // Expand ${CLAUDE_PLUGIN_ROOT} and check path
                let expanded =
                    command.replace("${CLAUDE_PLUGIN_ROOT}", &root.display().to_string());

                // Only check path-like commands (start with ./ or ${CLAUDE_PLUGIN_ROOT})
                if expanded.starts_with("./") || command.contains("${CLAUDE_PLUGIN_ROOT}") {
                    // Find the first token that looks like a script path, not a
                    // command prefix like "bash" or "node".
                    let script_token = expanded
                        .split_whitespace()
                        .find(|tok| tok.starts_with("./") || tok.starts_with('/'))
                        .or_else(|| expanded.split_whitespace().next())
                        .unwrap_or("");

                    let resolved = root.join(script_token);
                    if !resolved.exists() {
                        diags.push(
                            Diagnostic::new(
                                Severity::Error,
                                X002,
                                format!("hook command references missing script: \"{command}\""),
                            )
                            .with_suggestion("Ensure the script exists at the referenced path"),
                        );
                    }
                }
            }
        }
    }
}

/// Simple kebab-case check for file stems.
fn is_kebab_case(s: &str) -> bool {
    !s.is_empty()
        && s.chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        && !s.starts_with('-')
        && !s.ends_with('-')
}

/// Check naming consistency across components.
fn check_naming_consistency(components: &[Component], diags: &mut Vec<Diagnostic>) {
    if components.len() < 2 {
        return;
    }

    let kebab: Vec<_> = components
        .iter()
        .filter(|c| is_kebab_case(&c.name))
        .collect();
    let non_kebab: Vec<_> = components
        .iter()
        .filter(|c| !is_kebab_case(&c.name))
        .collect();

    // Only warn if there's a mix (some kebab, some not)
    if !kebab.is_empty() && !non_kebab.is_empty() {
        let examples: Vec<String> = non_kebab
            .iter()
            .take(3)
            .map(|c| format!("{}/{}", c.kind, c.name))
            .collect();
        diags.push(
            Diagnostic::new(
                Severity::Warning,
                X004,
                format!(
                    "naming inconsistency: {} of {} components are not kebab-case ({})",
                    non_kebab.len(),
                    components.len(),
                    examples.join(", ")
                ),
            )
            .with_suggestion("Use consistent kebab-case naming across all components"),
        );
    }
}

/// Check total token budget across all skills.
fn check_token_budget(skills_dir: &Path, diags: &mut Vec<Diagnostic>) {
    let entries = match std::fs::read_dir(skills_dir) {
        Ok(rd) => rd,
        Err(_) => return,
    };

    let mut total_tokens = 0usize;
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        if let Ok(props) = crate::parser::read_properties(&path) {
            total_tokens += crate::prompt::estimate_tokens(&props.name)
                + crate::prompt::estimate_tokens(&props.description);
        }
    }

    if total_tokens > TOKEN_BUDGET_THRESHOLD {
        diags.push(
            Diagnostic::new(
                Severity::Info,
                X005,
                format!(
                    "total skill token budget is ~{total_tokens} tokens (threshold: {TOKEN_BUDGET_THRESHOLD})"
                ),
            )
            .with_suggestion("Consider splitting skills into separate plugins or reducing descriptions"),
        );
    }
}

/// Check for duplicate component names across types.
fn check_duplicate_names(components: &[Component], diags: &mut Vec<Diagnostic>) {
    let mut seen: HashMap<&str, Vec<&str>> = HashMap::new();
    for c in components {
        seen.entry(c.name.as_str()).or_default().push(c.kind);
    }

    for (name, kinds) in &seen {
        if kinds.len() > 1 {
            let unique_kinds: HashSet<&&str> = kinds.iter().collect();
            if unique_kinds.len() > 1 {
                diags.push(
                    Diagnostic::new(
                        Severity::Error,
                        X006,
                        format!(
                            "duplicate name \"{name}\" used across component types: {}",
                            kinds.join(", ")
                        ),
                    )
                    .with_suggestion("Use unique names for each component"),
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    fn make_plugin(name: &str) -> (tempfile::TempDir, std::path::PathBuf) {
        let dir = tempdir().unwrap();
        let root = dir.path().to_path_buf();
        fs::write(
            root.join("plugin.json"),
            format!(r#"{{ "name": "{name}", "description": "test" }}"#),
        )
        .unwrap();
        (dir, root)
    }

    #[test]
    fn clean_plugin_no_errors() {
        let (_dir, root) = make_plugin("clean");
        let agents = root.join("agents");
        fs::create_dir(&agents).unwrap();
        fs::write(
            agents.join("reviewer.md"),
            "---\nname: reviewer\n---\nBody.\n",
        )
        .unwrap();
        let diags = validate_cross_component(&root);
        let errors: Vec<_> = diags.iter().filter(|d| d.is_error()).collect();
        assert!(errors.is_empty(), "unexpected errors: {errors:?}");
    }

    #[test]
    fn empty_component_dir_x001() {
        let (_dir, root) = make_plugin("test");
        fs::create_dir(root.join("agents")).unwrap();
        let diags = validate_cross_component(&root);
        assert!(diags.iter().any(|d| d.code == X001));
        // X001 is Info severity
        assert!(diags.iter().filter(|d| d.code == X001).all(|d| d.is_info()));
    }

    #[test]
    fn hook_missing_script_x002() {
        let (_dir, root) = make_plugin("test");
        fs::write(
            root.join("hooks.json"),
            r#"{ "PreToolUse": [{ "hooks": [{ "type": "command", "command": "./scripts/missing.sh" }] }] }"#,
        )
        .unwrap();
        let diags = validate_cross_component(&root);
        assert!(diags.iter().any(|d| d.code == X002));
    }

    #[test]
    fn hook_existing_script_no_x002() {
        let (_dir, root) = make_plugin("test");
        let scripts = root.join("scripts");
        fs::create_dir(&scripts).unwrap();
        fs::write(scripts.join("check.sh"), "#!/bin/bash\necho ok").unwrap();
        fs::write(
            root.join("hooks.json"),
            r#"{ "PreToolUse": [{ "hooks": [{ "type": "command", "command": "./scripts/check.sh" }] }] }"#,
        )
        .unwrap();
        let diags = validate_cross_component(&root);
        assert!(!diags.iter().any(|d| d.code == X002));
    }

    #[test]
    fn orphaned_file_x003() {
        let (_dir, root) = make_plugin("test");
        let agents = root.join("agents");
        fs::create_dir(&agents).unwrap();
        fs::write(agents.join("reviewer.md"), "---\nname: r\n---\nBody.\n").unwrap();
        fs::write(agents.join("notes.txt"), "some notes").unwrap();
        let diags = validate_cross_component(&root);
        assert!(diags.iter().any(|d| d.code == X003));
    }

    #[test]
    fn gitkeep_not_orphaned() {
        let (_dir, root) = make_plugin("test");
        let agents = root.join("agents");
        fs::create_dir(&agents).unwrap();
        fs::write(agents.join(".gitkeep"), "").unwrap();
        let diags = validate_cross_component(&root);
        assert!(!diags.iter().any(|d| d.code == X003));
    }

    #[test]
    fn naming_inconsistency_x004() {
        let (_dir, root) = make_plugin("test");
        let agents = root.join("agents");
        let commands = root.join("commands");
        fs::create_dir(&agents).unwrap();
        fs::create_dir(&commands).unwrap();
        fs::write(agents.join("code-reviewer.md"), "---\n---\nBody.\n").unwrap();
        fs::write(commands.join("MyCommand.md"), "Body.\n").unwrap();
        let diags = validate_cross_component(&root);
        assert!(diags.iter().any(|d| d.code == X004));
    }

    #[test]
    fn consistent_naming_no_x004() {
        let (_dir, root) = make_plugin("test");
        let agents = root.join("agents");
        let commands = root.join("commands");
        fs::create_dir(&agents).unwrap();
        fs::create_dir(&commands).unwrap();
        fs::write(agents.join("code-reviewer.md"), "---\n---\nBody.\n").unwrap();
        fs::write(commands.join("run-tests.md"), "Body.\n").unwrap();
        let diags = validate_cross_component(&root);
        assert!(!diags.iter().any(|d| d.code == X004));
    }

    #[test]
    fn duplicate_names_x006() {
        let (_dir, root) = make_plugin("test");
        let agents = root.join("agents");
        let commands = root.join("commands");
        fs::create_dir(&agents).unwrap();
        fs::create_dir(&commands).unwrap();
        fs::write(agents.join("deploy.md"), "---\n---\nBody.\n").unwrap();
        fs::write(commands.join("deploy.md"), "Body.\n").unwrap();
        let diags = validate_cross_component(&root);
        assert!(diags.iter().any(|d| d.code == X006));
    }

    #[test]
    fn unique_names_no_x006() {
        let (_dir, root) = make_plugin("test");
        let agents = root.join("agents");
        let commands = root.join("commands");
        fs::create_dir(&agents).unwrap();
        fs::create_dir(&commands).unwrap();
        fs::write(agents.join("reviewer.md"), "---\n---\nBody.\n").unwrap();
        fs::write(commands.join("deploy.md"), "Body.\n").unwrap();
        let diags = validate_cross_component(&root);
        assert!(!diags.iter().any(|d| d.code == X006));
    }

    #[test]
    fn no_component_dirs_no_errors() {
        let (_dir, root) = make_plugin("test");
        let diags = validate_cross_component(&root);
        let errors: Vec<_> = diags.iter().filter(|d| d.is_error()).collect();
        assert!(errors.is_empty());
    }

    #[test]
    fn hook_with_plugin_root_var_x002() {
        let (_dir, root) = make_plugin("test");
        fs::write(
            root.join("hooks.json"),
            r#"{ "PreToolUse": [{ "hooks": [{ "type": "command", "command": "${CLAUDE_PLUGIN_ROOT}/scripts/missing.sh" }] }] }"#,
        )
        .unwrap();
        let diags = validate_cross_component(&root);
        assert!(diags.iter().any(|d| d.code == X002));
    }

    #[test]
    fn prompt_hooks_not_checked_for_x002() {
        let (_dir, root) = make_plugin("test");
        fs::write(
            root.join("hooks.json"),
            r#"{ "Stop": [{ "hooks": [{ "type": "prompt", "prompt": "Review output" }] }] }"#,
        )
        .unwrap();
        let diags = validate_cross_component(&root);
        assert!(!diags.iter().any(|d| d.code == X002));
    }

    #[test]
    fn skills_as_subdirs_discovered() {
        let (_dir, root) = make_plugin("test");
        let skills = root.join("skills");
        let skill_dir = skills.join("my-skill");
        fs::create_dir_all(&skill_dir).unwrap();
        fs::write(
            skill_dir.join("SKILL.md"),
            "---\nname: my-skill\ndescription: Does things.\n---\nBody.\n",
        )
        .unwrap();
        let diags = validate_cross_component(&root);
        // Should NOT emit X001 for a valid skill subdirectory
        assert!(
            !diags.iter().any(|d| d.code == X001),
            "unexpected X001: {diags:?}"
        );
    }

    #[test]
    fn empty_skills_dir_x001() {
        let (_dir, root) = make_plugin("test");
        fs::create_dir(root.join("skills")).unwrap();
        let diags = validate_cross_component(&root);
        assert!(diags.iter().any(|d| d.code == X001));
    }

    #[test]
    fn skill_and_agent_duplicate_x006() {
        let (_dir, root) = make_plugin("test");
        // Create agent "deploy"
        let agents = root.join("agents");
        fs::create_dir(&agents).unwrap();
        fs::write(agents.join("deploy.md"), "---\n---\nBody.\n").unwrap();
        // Create skill "deploy"
        let skill_dir = root.join("skills").join("deploy");
        fs::create_dir_all(&skill_dir).unwrap();
        fs::write(
            skill_dir.join("SKILL.md"),
            "---\nname: deploy\ndescription: Deploys things.\n---\nBody.\n",
        )
        .unwrap();
        let diags = validate_cross_component(&root);
        assert!(
            diags.iter().any(|d| d.code == X006),
            "expected X006 for skill/agent name collision: {diags:?}"
        );
    }
}
