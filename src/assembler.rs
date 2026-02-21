//! Skill-to-plugin assembly: packages skill directories into a Claude Code plugin.
//!
//! Takes one or more skill directories and generates a complete plugin directory
//! structure with a `plugin.json` manifest, `skills/` subdirectory containing
//! the skill files, and scaffolded `agents/` and `hooks/` directories.

use std::path::{Path, PathBuf};

use crate::errors::{AigentError, Result};
use crate::parser::{find_skill_md, read_properties};

/// Maximum recursion depth for directory operations.
const MAX_RECURSION_DEPTH: usize = 10;

/// Assembled skill metadata collected during plugin assembly.
#[derive(Debug)]
pub struct AssembleWarning {
    /// The skill directory that caused the warning.
    pub dir: PathBuf,
    /// A human-readable warning message.
    pub message: String,
}

/// Options for plugin assembly.
#[derive(Debug)]
pub struct AssembleOptions {
    /// Output directory for the assembled plugin.
    pub output_dir: PathBuf,
    /// Override plugin name (default: derived from first skill).
    pub name: Option<String>,
    /// Run validation on assembled skills.
    pub validate: bool,
}

/// Result of a successful plugin assembly.
#[derive(Debug)]
pub struct AssembleResult {
    /// Path to the assembled plugin directory.
    pub plugin_dir: PathBuf,
    /// Number of skills included.
    pub skills_count: usize,
    /// Non-fatal warnings encountered during assembly.
    pub warnings: Vec<AssembleWarning>,
}

/// Assemble skills into a plugin directory.
///
/// Creates the output directory structure:
/// ```text
/// <output_dir>/
/// ├── plugin.json
/// ├── skills/
/// │   ├── <skill-1>/
/// │   │   └── SKILL.md
/// │   └── <skill-2>/
/// │       └── SKILL.md
/// ├── agents/
/// └── hooks/
/// ```
///
/// # Errors
///
/// Returns an error if:
/// - No valid skills are found in the input directories
/// - The output directory cannot be created
/// - Skill files cannot be read or copied
pub fn assemble_plugin(skill_dirs: &[&Path], opts: &AssembleOptions) -> Result<AssembleResult> {
    if skill_dirs.is_empty() {
        return Err(AigentError::Build {
            message: "no skill directories provided".into(),
        });
    }

    // Collect valid skills.
    let mut skills: Vec<(String, PathBuf)> = Vec::new();
    let mut warnings: Vec<AssembleWarning> = Vec::new();
    for dir in skill_dirs {
        if let Some(skill_path) = find_skill_md(dir) {
            match read_properties(dir) {
                Ok(props) => {
                    // Validate skill name to prevent path traversal.
                    if is_unsafe_name(&props.name) {
                        warnings.push(AssembleWarning {
                            dir: dir.to_path_buf(),
                            message: format!(
                                "skipping: unsafe skill name '{}' (contains path separators or '..')",
                                props.name
                            ),
                        });
                        continue;
                    }
                    skills.push((props.name.clone(), skill_path));
                }
                Err(e) => {
                    warnings.push(AssembleWarning {
                        dir: dir.to_path_buf(),
                        message: format!("skipping: {e}"),
                    });
                }
            }
        } else {
            warnings.push(AssembleWarning {
                dir: dir.to_path_buf(),
                message: "no SKILL.md found".into(),
            });
        }
    }

    if skills.is_empty() {
        return Err(AigentError::Build {
            message: "no valid skills found in provided directories".into(),
        });
    }

    // Determine plugin name.
    let plugin_name = opts.name.clone().unwrap_or_else(|| skills[0].0.clone());

    // Create output directory structure.
    let out = &opts.output_dir;
    let skills_dir = out.join("skills");
    let agents_dir = out.join("agents");
    let hooks_dir = out.join("hooks");

    std::fs::create_dir_all(&skills_dir)?;
    std::fs::create_dir_all(&agents_dir)?;
    std::fs::create_dir_all(&hooks_dir)?;

    // Copy each skill into skills/<name>/.
    for (name, skill_path) in &skills {
        let dest_dir = skills_dir.join(name);
        std::fs::create_dir_all(&dest_dir)?;

        // Copy the SKILL.md file.
        let dest_file = dest_dir.join("SKILL.md");
        std::fs::copy(skill_path, &dest_file)?;

        // Copy any sibling files in the same directory as SKILL.md.
        if let Some(src_dir) = skill_path.parent() {
            copy_skill_files(src_dir, &dest_dir)?;
        }
    }

    // Validate assembled skills if requested.
    if opts.validate {
        let mut all_valid = true;
        for (name, _) in &skills {
            let dest_dir = skills_dir.join(name);
            let diags = crate::validate(&dest_dir);
            if diags.iter().any(|d| d.is_error()) {
                all_valid = false;
                for d in &diags {
                    warnings.push(AssembleWarning {
                        dir: dest_dir.clone(),
                        message: format!("{name}: {d}"),
                    });
                }
            }
        }
        if !all_valid {
            return Err(AigentError::Build {
                message: "assembled skills have validation errors".into(),
            });
        }
    }

    // Generate plugin.json.
    let plugin_json = generate_plugin_json(&plugin_name, &skills)?;
    std::fs::write(out.join("plugin.json"), plugin_json)?;

    Ok(AssembleResult {
        plugin_dir: out.clone(),
        skills_count: skills.len(),
        warnings,
    })
}

/// Check whether a skill name is unsafe for use as a directory component.
///
/// Rejects names containing path separators (`/`, `\`), parent traversal (`..`),
/// or that are empty.
fn is_unsafe_name(name: &str) -> bool {
    name.is_empty()
        || name.contains('/')
        || name.contains('\\')
        || name.contains("..")
        || name == "."
}

/// Copy non-SKILL.md files from source dir to destination dir.
///
/// Copies reference files, scripts, etc. that the skill may depend on.
/// Skips hidden files and the target/ directory.
fn copy_skill_files(src: &Path, dest: &Path) -> Result<()> {
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let name = entry.file_name();
        let name_str = name.to_string_lossy();

        // Skip SKILL.md (already copied), hidden files, and target/.
        if name_str == "SKILL.md"
            || name_str == "skill.md"
            || name_str.starts_with('.')
            || name_str == "target"
        {
            continue;
        }

        let src_path = entry.path();
        let dest_path = dest.join(&name);

        if src_path.is_file() {
            std::fs::copy(&src_path, &dest_path)?;
        } else if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dest_path, 0)?;
        }
    }
    Ok(())
}

/// Recursively copy a directory.
///
/// # Errors
///
/// Returns an error if the recursion depth exceeds [`MAX_RECURSION_DEPTH`].
fn copy_dir_recursive(src: &Path, dest: &Path, depth: usize) -> Result<()> {
    if depth > MAX_RECURSION_DEPTH {
        return Err(AigentError::Build {
            message: format!("exceeded maximum directory depth ({MAX_RECURSION_DEPTH})"),
        });
    }
    std::fs::create_dir_all(dest)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dest_path = dest.join(entry.file_name());

        if src_path.is_file() {
            std::fs::copy(&src_path, &dest_path)?;
        } else if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dest_path, depth + 1)?;
        }
    }
    Ok(())
}

/// Generate plugin.json content from skill metadata.
///
/// Uses `serde_json` for proper escaping of all string values.
fn generate_plugin_json(name: &str, skills: &[(String, PathBuf)]) -> Result<String> {
    let skill_names: Vec<&str> = skills.iter().map(|(name, _)| name.as_str()).collect();

    let json = serde_json::json!({
        "name": name,
        "description": format!("Plugin assembled from {} skill(s)", skills.len()),
        "version": "0.1.0",
        "skills": skill_names,
    });

    serde_json::to_string_pretty(&json).map_err(|e| AigentError::Build {
        message: format!("failed to generate plugin.json: {e}"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    fn make_skill(parent: &Path, name: &str, content: &str) -> PathBuf {
        let dir = parent.join(name);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("SKILL.md"), content).unwrap();
        dir
    }

    #[test]
    fn assemble_single_skill_creates_plugin() {
        let parent = tempdir().unwrap();
        let skill = make_skill(
            parent.path(),
            "my-skill",
            "---\nname: my-skill\ndescription: Does things\n---\nBody.\n",
        );
        let out = parent.path().join("output");
        let opts = AssembleOptions {
            output_dir: out.clone(),
            name: None,
            validate: false,
        };
        let result = assemble_plugin(&[skill.as_path()], &opts).unwrap();
        assert_eq!(result.skills_count, 1);
        assert!(out.join("plugin.json").exists());
        assert!(out.join("skills/my-skill/SKILL.md").exists());
        assert!(out.join("agents").exists());
        assert!(out.join("hooks").exists());
    }

    #[test]
    fn assemble_multiple_skills() {
        let parent = tempdir().unwrap();
        let s1 = make_skill(
            parent.path(),
            "skill-one",
            "---\nname: skill-one\ndescription: First\n---\nBody.\n",
        );
        let s2 = make_skill(
            parent.path(),
            "skill-two",
            "---\nname: skill-two\ndescription: Second\n---\nBody.\n",
        );
        let out = parent.path().join("output");
        let opts = AssembleOptions {
            output_dir: out.clone(),
            name: Some("my-plugin".into()),
            validate: false,
        };
        let result = assemble_plugin(&[s1.as_path(), s2.as_path()], &opts).unwrap();
        assert_eq!(result.skills_count, 2);
        assert!(out.join("skills/skill-one/SKILL.md").exists());
        assert!(out.join("skills/skill-two/SKILL.md").exists());
    }

    #[test]
    fn assemble_plugin_json_has_correct_structure() {
        let parent = tempdir().unwrap();
        let skill = make_skill(
            parent.path(),
            "my-skill",
            "---\nname: my-skill\ndescription: Does things\n---\nBody.\n",
        );
        let out = parent.path().join("output");
        let opts = AssembleOptions {
            output_dir: out.clone(),
            name: Some("test-plugin".into()),
            validate: false,
        };
        assemble_plugin(&[skill.as_path()], &opts).unwrap();
        let json_str = fs::read_to_string(out.join("plugin.json")).unwrap();
        let json: serde_json::Value = serde_json::from_str(&json_str).unwrap();
        assert_eq!(json["name"], "test-plugin");
        assert_eq!(json["version"], "0.1.0");
        assert!(json["skills"].as_array().unwrap().len() == 1);
    }

    #[test]
    fn assemble_no_skills_returns_error() {
        let parent = tempdir().unwrap();
        let out = parent.path().join("output");
        let opts = AssembleOptions {
            output_dir: out,
            name: None,
            validate: false,
        };
        let result = assemble_plugin(&[], &opts);
        assert!(result.is_err());
    }

    #[test]
    fn assemble_copies_reference_files() {
        let parent = tempdir().unwrap();
        let skill_dir = parent.path().join("my-skill");
        fs::create_dir_all(&skill_dir).unwrap();
        fs::write(
            skill_dir.join("SKILL.md"),
            "---\nname: my-skill\ndescription: Does things\n---\nBody.\n",
        )
        .unwrap();
        fs::write(skill_dir.join("reference.md"), "Extra docs").unwrap();

        let out = parent.path().join("output");
        let opts = AssembleOptions {
            output_dir: out.clone(),
            name: None,
            validate: false,
        };
        assemble_plugin(&[skill_dir.as_path()], &opts).unwrap();
        assert!(out.join("skills/my-skill/reference.md").exists());
    }

    #[test]
    fn assemble_with_validate_rejects_invalid_skill() {
        let parent = tempdir().unwrap();
        // Missing required 'name' field.
        let skill = make_skill(
            parent.path(),
            "bad-skill",
            "---\ndescription: Missing name\n---\nBody.\n",
        );
        let out = parent.path().join("output");
        let opts = AssembleOptions {
            output_dir: out,
            name: None,
            validate: true,
        };
        let result = assemble_plugin(&[skill.as_path()], &opts);
        assert!(result.is_err());
    }

    #[test]
    fn assemble_name_defaults_to_first_skill() {
        let parent = tempdir().unwrap();
        let skill = make_skill(
            parent.path(),
            "first-skill",
            "---\nname: first-skill\ndescription: Does things\n---\nBody.\n",
        );
        let out = parent.path().join("output");
        let opts = AssembleOptions {
            output_dir: out.clone(),
            name: None,
            validate: false,
        };
        assemble_plugin(&[skill.as_path()], &opts).unwrap();
        let json_str = fs::read_to_string(out.join("plugin.json")).unwrap();
        let json: serde_json::Value = serde_json::from_str(&json_str).unwrap();
        assert_eq!(json["name"], "first-skill");
    }

    #[test]
    fn assemble_rejects_path_traversal_name() {
        let parent = tempdir().unwrap();
        let skill = make_skill(
            parent.path(),
            "evil-skill",
            "---\nname: ../../../etc/passwd\ndescription: Malicious\n---\nBody.\n",
        );
        let out = parent.path().join("output");
        let opts = AssembleOptions {
            output_dir: out,
            name: None,
            validate: false,
        };
        // Should fail because the only skill has an unsafe name.
        let result = assemble_plugin(&[skill.as_path()], &opts);
        assert!(result.is_err());
    }

    #[test]
    fn assemble_warns_on_unsafe_name_but_continues_with_others() {
        let parent = tempdir().unwrap();
        let bad = make_skill(
            parent.path(),
            "bad",
            "---\nname: ../escape\ndescription: Malicious\n---\nBody.\n",
        );
        let good = make_skill(
            parent.path(),
            "good-skill",
            "---\nname: good-skill\ndescription: Legit\n---\nBody.\n",
        );
        let out = parent.path().join("output");
        let opts = AssembleOptions {
            output_dir: out.clone(),
            name: None,
            validate: false,
        };
        let result = assemble_plugin(&[bad.as_path(), good.as_path()], &opts).unwrap();
        assert_eq!(result.skills_count, 1);
        assert!(!result.warnings.is_empty());
        assert!(result.warnings[0].message.contains("unsafe skill name"));
    }

    #[test]
    fn generate_plugin_json_escapes_special_characters() {
        let skills = vec![("skill-with-\"quotes\"".to_string(), PathBuf::from("a.md"))];
        let json_str = generate_plugin_json("name-with-\"quotes\"", &skills).unwrap();
        let json: serde_json::Value = serde_json::from_str(&json_str).unwrap();
        assert_eq!(json["name"], "name-with-\"quotes\"");
        assert_eq!(json["skills"][0], "skill-with-\"quotes\"");
    }

    #[test]
    fn is_unsafe_name_detects_traversal() {
        assert!(is_unsafe_name("../etc/passwd"));
        assert!(is_unsafe_name("foo/bar"));
        assert!(is_unsafe_name("foo\\bar"));
        assert!(is_unsafe_name(".."));
        assert!(is_unsafe_name("."));
        assert!(is_unsafe_name(""));
        assert!(!is_unsafe_name("my-skill"));
        assert!(!is_unsafe_name("skill_with_underscores"));
    }

    #[test]
    fn copy_dir_recursive_normal_depth() {
        let tmp = tempdir().unwrap();
        let src = tmp.path().join("src");
        let dest = tmp.path().join("dest");

        // Create a few levels of nesting (well under the limit).
        let mut current = src.clone();
        for i in 0..5 {
            current = current.join(format!("level-{i}"));
            fs::create_dir_all(&current).unwrap();
            fs::write(current.join("file.txt"), format!("level {i}")).unwrap();
        }

        copy_dir_recursive(&src, &dest, 0).unwrap();

        // Verify deepest file was copied.
        let mut check = dest.clone();
        for i in 0..5 {
            check = check.join(format!("level-{i}"));
        }
        assert!(check.join("file.txt").exists());
    }

    #[test]
    fn copy_dir_recursive_exceeds_depth_limit() {
        let tmp = tempdir().unwrap();
        let src = tmp.path().join("src");
        let dest = tmp.path().join("dest");

        // Create nesting deeper than MAX_RECURSION_DEPTH.
        let mut current = src.clone();
        for i in 0..15 {
            current = current.join(format!("level-{i}"));
            fs::create_dir_all(&current).unwrap();
        }

        let result = copy_dir_recursive(&src, &dest, 0);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("exceeded maximum directory depth"),
            "expected depth error, got: {err_msg}"
        );
    }

    #[test]
    fn copy_dir_recursive_error_message_contains_limit() {
        let tmp = tempdir().unwrap();
        let src = tmp.path().join("src");
        let dest = tmp.path().join("dest");

        let mut current = src.clone();
        for i in 0..15 {
            current = current.join(format!("level-{i}"));
            fs::create_dir_all(&current).unwrap();
        }

        let result = copy_dir_recursive(&src, &dest, 0);
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains(&MAX_RECURSION_DEPTH.to_string()),
            "error should contain the depth limit value, got: {err_msg}"
        );
    }
}
