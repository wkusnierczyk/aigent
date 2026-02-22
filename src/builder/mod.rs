/// Deterministic (zero-config) skill generation heuristics.
pub mod deterministic;
/// LLM-enhanced skill generation and provider trait.
pub mod llm;
/// LLM provider implementations (Anthropic, OpenAI, Google, Ollama).
pub mod providers;
/// Template generation for `init` command.
pub mod template;
mod util;

pub use llm::LlmProvider;
pub use template::SkillTemplate;

use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io::Write as _;
use std::path::{Path, PathBuf};

use crate::errors::{AigentError, Result};
use crate::models::SkillProperties;

/// Write content to a file atomically, failing if the file already exists.
///
/// Uses `create_new(true)` to prevent TOCTOU races. Returns a descriptive
/// error including the file path on failure.
fn write_exclusive(path: &Path, content: &[u8]) -> Result<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::AlreadyExists {
                AigentError::AlreadyExists {
                    path: path.to_path_buf(),
                }
            } else {
                AigentError::Build {
                    message: format!("cannot create {}: {e}", path.display()),
                }
            }
        })?;
    file.write_all(content).map_err(|e| AigentError::Build {
        message: format!("cannot write {}: {e}", path.display()),
    })
}
use crate::validator::validate;

use deterministic::{generate_body, generate_description};
use llm::{detect_provider, llm_derive_name, llm_generate_body, llm_generate_description};

/// User input for skill generation.
#[derive(Debug, Clone, Default)]
pub struct SkillSpec {
    /// Natural language description of what the skill should do.
    pub purpose: String,
    /// Explicit skill name override. If `None`, derived from `purpose`.
    pub name: Option<String>,
    /// Allowed tools (e.g., `"Bash, Read"`).
    pub tools: Option<String>,
    /// Compatibility string (e.g., `"Claude 3.5 and above"`).
    pub compatibility: Option<String>,
    /// License identifier (e.g., `"MIT"`).
    pub license: Option<String>,
    /// Additional files to write alongside SKILL.md, keyed by relative path.
    pub extra_files: Option<HashMap<String, String>>,
    /// Output directory override. If `None`, derived from the skill name.
    pub output_dir: Option<PathBuf>,
    /// Force deterministic mode (no LLM) regardless of environment.
    pub no_llm: bool,
    /// Template variant for generating the skill structure.
    pub template: SkillTemplate,
}

/// Result of skill generation.
#[derive(Debug)]
pub struct BuildResult {
    /// Parsed properties from the generated SKILL.md frontmatter.
    pub properties: SkillProperties,
    /// All files written, keyed by relative path (includes `SKILL.md`).
    pub files: HashMap<String, String>,
    /// Directory where the skill was created.
    pub output_dir: PathBuf,
    /// Warnings collected during generation (e.g., LLM fallback notices).
    ///
    /// These replace the previous `eprintln!` calls, giving library consumers
    /// structured access to non-fatal issues that occurred during the build.
    pub warnings: Vec<String>,
}

/// Clarity assessment result.
#[derive(Debug)]
pub struct ClarityAssessment {
    /// Whether the purpose description is clear enough for generation.
    pub clear: bool,
    /// Follow-up questions to ask if not clear (empty when `clear` is true).
    pub questions: Vec<String>,
}

/// Build a complete skill from a specification.
///
/// Generates a SKILL.md with valid frontmatter and markdown body, creates the
/// output directory, writes files, and validates the result. The output
/// directory is determined from `spec.output_dir` (if provided) or derived
/// from the skill name.
///
/// Returns `AigentError::Build` if the output directory already contains a
/// SKILL.md or if the generated output fails validation.
pub fn build_skill(spec: &SkillSpec) -> Result<BuildResult> {
    // 0. Select provider (unless no_llm).
    let provider: Option<Box<dyn LlmProvider>> = if spec.no_llm { None } else { detect_provider() };
    let mut warnings = Vec::new();

    // 1. Derive name (LLM with fallback to deterministic).
    let name = if let Some(explicit) = &spec.name {
        explicit.clone()
    } else if let Some(ref prov) = provider {
        match llm_derive_name(prov.as_ref(), &spec.purpose) {
            Ok(n) => n,
            Err(e) => {
                warnings.push(format!(
                    "LLM name derivation failed ({e}), using deterministic"
                ));
                deterministic::derive_name(&spec.purpose)
            }
        }
    } else {
        deterministic::derive_name(&spec.purpose)
    };

    // 2. Determine output directory.
    let output_dir = spec
        .output_dir
        .clone()
        .unwrap_or_else(|| PathBuf::from(&name));

    // 3. Generate description (LLM with fallback).
    let description = if let Some(ref prov) = provider {
        match llm_generate_description(prov.as_ref(), &spec.purpose, &name) {
            Ok(d) => d,
            Err(e) => {
                warnings.push(format!(
                    "LLM description generation failed ({e}), using deterministic"
                ));
                generate_description(&spec.purpose, &name)
            }
        }
    } else {
        generate_description(&spec.purpose, &name)
    };

    // 4. Construct SkillProperties directly.
    let properties = SkillProperties {
        name: name.clone(),
        description,
        license: spec.license.clone(),
        compatibility: spec.compatibility.clone(),
        allowed_tools: spec.tools.clone(),
        metadata: None,
    };

    // 5. Generate body (LLM with fallback).
    let body = if let Some(ref prov) = provider {
        match llm_generate_body(
            prov.as_ref(),
            &spec.purpose,
            &properties.name,
            &properties.description,
        ) {
            Ok(b) => b,
            Err(e) => {
                warnings.push(format!(
                    "LLM body generation failed ({e}), using deterministic"
                ));
                generate_body(&spec.purpose, &properties.name, &properties.description)
            }
        }
    } else {
        generate_body(&spec.purpose, &properties.name, &properties.description)
    };

    // 6. Serialize SkillProperties to YAML frontmatter.
    let yaml = serde_yaml_ng::to_string(&properties).map_err(|e| AigentError::Build {
        message: format!("failed to serialize frontmatter: {e}"),
    })?;

    // 7. Assemble SKILL.md content.
    let content = format!("---\n{yaml}---\n{body}");

    // 8. Create output directory if needed.
    std::fs::create_dir_all(&output_dir)?;

    // 9. Write SKILL.md atomically (fails if file already exists).
    let skill_md_path = output_dir.join("SKILL.md");
    write_exclusive(&skill_md_path, content.as_bytes())?;

    // 10. Write extra files if present.
    let mut files = HashMap::new();
    files.insert("SKILL.md".to_string(), content);

    if let Some(ref extra) = spec.extra_files {
        for (rel_path, file_content) in extra {
            // Reject absolute paths and path traversal components.
            let path = std::path::Path::new(rel_path);
            if path.is_absolute()
                || path
                    .components()
                    .any(|c| matches!(c, std::path::Component::ParentDir))
            {
                return Err(AigentError::Build {
                    message: format!("extra file path must be relative without '..': {rel_path}"),
                });
            }
            let full_path = output_dir.join(rel_path);
            if let Some(parent) = full_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&full_path, file_content)?;
            files.insert(rel_path.clone(), file_content.clone());
        }
    }

    // 11. Validate output.
    let diags = validate(&output_dir);
    let errors: Vec<_> = diags.iter().filter(|d| d.is_error()).collect();
    if !errors.is_empty() {
        // Best-effort cleanup of files we just wrote, to avoid leaving
        // invalid artifacts on disk that block subsequent runs.
        let _ = std::fs::remove_file(&skill_md_path);
        if let Some(ref extra) = spec.extra_files {
            for rel_path in extra.keys() {
                let full_path = output_dir.join(rel_path);
                let _ = std::fs::remove_file(&full_path);
            }
        }
        let error_msgs: Vec<String> = errors.iter().map(|d| d.to_string()).collect();
        return Err(AigentError::Build {
            message: format!(
                "generated skill failed validation:\n{}",
                error_msgs.join("\n")
            ),
        });
    }

    // 12. Return BuildResult.
    Ok(BuildResult {
        properties,
        files,
        output_dir,
        warnings,
    })
}

/// Derive a kebab-case skill name from a natural language description.
///
/// Uses deterministic heuristics: lowercase, remove filler words, apply
/// gerund form, kebab-case, sanitize, and truncate to 64 characters.
#[must_use]
pub fn derive_name(purpose: &str) -> String {
    deterministic::derive_name(purpose)
}

/// Evaluate if a purpose description is clear enough for autonomous generation.
///
/// Uses deterministic heuristics based on word count, question marks, and
/// purpose structure.
#[must_use]
pub fn assess_clarity(purpose: &str) -> ClarityAssessment {
    deterministic::assess_clarity(purpose)
}

/// Initialize a skill directory with a template SKILL.md.
///
/// Creates the directory if it doesn't exist. Returns an error if a SKILL.md
/// (or skill.md) already exists in the target directory. The `tmpl` parameter
/// selects the template variant; use `SkillTemplate::Minimal` for the default.
pub fn init_skill(dir: &Path, tmpl: SkillTemplate) -> Result<PathBuf> {
    // Derive directory name for the template.
    // Filter out "." and ".." which produce empty kebab-case names.
    let dir_name = dir
        .file_name()
        .and_then(|n| n.to_str())
        .filter(|name| !name.is_empty() && *name != "." && *name != "..")
        .map(|name| name.to_string())
        .or_else(|| {
            // Fall back to the current working directory's basename.
            std::env::current_dir().ok().and_then(|cwd| {
                cwd.file_name()
                    .and_then(|n| n.to_str())
                    .filter(|name| !name.is_empty() && *name != "." && *name != "..")
                    .map(|name| name.to_string())
            })
        })
        .unwrap_or_else(|| "my-skill".to_string());

    // Generate template files.
    let files = template::template_files(tmpl, &dir_name);

    // Create directory if needed.
    std::fs::create_dir_all(dir)?;

    // Write all template files.
    for (rel_path, content) in &files {
        let full_path = dir.join(rel_path);
        if let Some(parent) = full_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Use atomic exclusive creation for SKILL.md to prevent TOCTOU races.
        if rel_path == "SKILL.md" {
            write_exclusive(&full_path, content.as_bytes())?;
        } else {
            std::fs::write(&full_path, content)?;
        }

        // On Unix, set execute bit on shell scripts.
        #[cfg(unix)]
        {
            if full_path
                .extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(|ext| ext.eq_ignore_ascii_case("sh"))
            {
                use std::os::unix::fs::PermissionsExt;
                let metadata = std::fs::metadata(&full_path)?;
                let mut perms = metadata.permissions();
                perms.set_mode(perms.mode() | 0o111);
                std::fs::set_permissions(&full_path, perms)?;
            }
        }
    }

    Ok(dir.join("SKILL.md"))
}

/// Run an interactive build session, prompting for confirmation at each step.
///
/// Uses the provided `reader` for input (stdin in production, `Cursor` in
/// tests). Writes progress to stderr. Returns the same `BuildResult` as
/// [`build_skill`] on success.
///
/// **Note**: Interactive mode always uses deterministic (template-based)
/// generation regardless of the `no_llm` setting on the spec. This ensures
/// the user sees exactly what will be written before confirming.
///
/// The flow is:
/// 1. Assess clarity — if unclear, print questions and return error
/// 2. Derive name — print and confirm
/// 3. Generate description — print and confirm
/// 4. Generate body preview — print first 20 lines
/// 5. Confirm write
/// 6. Build, validate, and report
pub fn interactive_build(
    spec: &SkillSpec,
    reader: &mut dyn std::io::BufRead,
) -> Result<BuildResult> {
    // 1. Assess clarity.
    let assessment = assess_clarity(&spec.purpose);
    if !assessment.clear {
        eprintln!("Purpose needs clarification:");
        for q in &assessment.questions {
            eprintln!("  - {q}");
        }
        return Err(AigentError::Build {
            message: "purpose is not clear enough for generation".to_string(),
        });
    }

    // 2. Derive name.
    let name = spec
        .name
        .clone()
        .unwrap_or_else(|| derive_name(&spec.purpose));
    eprintln!("Name: {name}");
    if !confirm("Continue?", reader)? {
        return Err(AigentError::Build {
            message: "cancelled by user".to_string(),
        });
    }

    // 3. Generate description.
    let description = deterministic::generate_description(&spec.purpose, &name);
    eprintln!("Description: {description}");
    if !confirm("Continue?", reader)? {
        return Err(AigentError::Build {
            message: "cancelled by user".to_string(),
        });
    }

    // 4. Preview body.
    let body = generate_body(&spec.purpose, &name, &description);
    eprintln!("Body preview:");
    for line in body.lines().take(20) {
        eprintln!("  {line}");
    }
    let total_lines = body.lines().count();
    if total_lines > 20 {
        eprintln!("  ... ({} more lines)", total_lines - 20);
    }

    // 5. Confirm write.
    if !confirm("Write skill?", reader)? {
        return Err(AigentError::Build {
            message: "cancelled by user".to_string(),
        });
    }

    // 6. Build (reuse standard build with forced deterministic mode).
    let build_spec = SkillSpec {
        purpose: spec.purpose.clone(),
        name: Some(name),
        no_llm: true,
        output_dir: spec.output_dir.clone(),
        template: spec.template,
        ..Default::default()
    };
    let result = build_skill(&build_spec)?;

    // 7. Report.
    let diags = validate(&result.output_dir);
    let error_count = diags.iter().filter(|d| d.is_error()).count();
    let warning_count = diags.iter().filter(|d| d.is_warning()).count();
    if error_count == 0 && warning_count == 0 {
        eprintln!("Validation: passed");
    } else {
        eprintln!("Validation: {error_count} error(s), {warning_count} warning(s)");
        for d in &diags {
            eprintln!("  {d}");
        }
    }

    Ok(result)
}

/// Read a yes/no confirmation from `reader`. Returns `true` for "y" or "yes".
fn confirm(prompt: &str, reader: &mut dyn std::io::BufRead) -> Result<bool> {
    eprint!("{prompt} [y/N] ");
    let mut line = String::new();
    reader
        .read_line(&mut line)
        .map_err(|e| AigentError::Build {
            message: format!("failed to read input: {e}"),
        })?;
    let answer = line.trim().to_lowercase();
    Ok(answer == "y" || answer == "yes")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    // ── init_skill tests (24-29) ──────────────────────────────────────

    #[test]
    fn init_creates_skill_md_in_empty_dir() {
        let parent = tempdir().unwrap();
        let dir = parent.path().join("my-skill");
        let _ = init_skill(&dir, SkillTemplate::Minimal).unwrap();
        assert!(dir.join("SKILL.md").exists());
    }

    #[test]
    fn init_returns_path_to_created_file() {
        let parent = tempdir().unwrap();
        let dir = parent.path().join("my-skill");
        let path = init_skill(&dir, SkillTemplate::Minimal).unwrap();
        assert_eq!(path, dir.join("SKILL.md"));
    }

    #[test]
    fn init_created_file_has_valid_frontmatter() {
        let parent = tempdir().unwrap();
        let dir = parent.path().join("my-skill");
        init_skill(&dir, SkillTemplate::Minimal).unwrap();
        // The file should be parseable.
        let result = crate::read_properties(&dir);
        assert!(
            result.is_ok(),
            "init output should be parseable: {result:?}"
        );
    }

    #[test]
    fn init_name_derived_from_directory() {
        let parent = tempdir().unwrap();
        let dir = parent.path().join("cool-tool");
        init_skill(&dir, SkillTemplate::Minimal).unwrap();
        let props = crate::read_properties(&dir).unwrap();
        assert_eq!(props.name, "cool-tool");
    }

    #[test]
    fn init_fails_if_skill_md_exists() {
        let parent = tempdir().unwrap();
        let dir = parent.path().join("my-skill");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("SKILL.md"), "---\nname: x\n---\n").unwrap();
        let result = init_skill(&dir, SkillTemplate::Minimal);
        assert!(result.is_err(), "should fail if SKILL.md already exists");
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("already exists"),
            "error should mention 'already exists': {err}"
        );
    }

    #[test]
    fn init_creates_directory_if_missing() {
        let parent = tempdir().unwrap();
        let dir = parent.path().join("nonexistent-dir");
        assert!(!dir.exists());
        init_skill(&dir, SkillTemplate::Minimal).unwrap();
        assert!(dir.exists());
        assert!(dir.join("SKILL.md").exists());
    }

    #[test]
    #[cfg(unix)]
    fn init_code_skill_script_is_executable() {
        use std::os::unix::fs::PermissionsExt;
        let parent = tempdir().unwrap();
        let dir = parent.path().join("code-skill");
        init_skill(&dir, SkillTemplate::CodeSkill).unwrap();
        let script = dir.join("scripts/run.sh");
        assert!(script.exists(), "scripts/run.sh should exist");
        let perms = std::fs::metadata(&script).unwrap().permissions();
        assert!(
            perms.mode() & 0o111 != 0,
            "scripts/run.sh should be executable, mode: {:o}",
            perms.mode()
        );
    }

    // ── build_skill tests (30-38) ─────────────────────────────────────

    #[test]
    fn build_deterministic_creates_valid_skill_md() {
        let parent = tempdir().unwrap();
        let dir = parent.path().join("processing-pdf-files");
        let spec = SkillSpec {
            purpose: "Process PDF files".to_string(),
            output_dir: Some(dir.clone()),
            no_llm: true,
            ..Default::default()
        };
        let result = build_skill(&spec).unwrap();
        assert!(dir.join("SKILL.md").exists());
        assert_eq!(result.properties.name, "processing-pdf-files");
    }

    #[test]
    fn build_output_passes_validate() {
        let parent = tempdir().unwrap();
        let dir = parent.path().join("processing-pdf-files");
        let spec = SkillSpec {
            purpose: "Process PDF files".to_string(),
            output_dir: Some(dir.clone()),
            no_llm: true,
            ..Default::default()
        };
        build_skill(&spec).unwrap();
        let diags = crate::validate(&dir);
        let errors: Vec<_> = diags.iter().filter(|d| d.is_error()).collect();
        assert!(
            errors.is_empty(),
            "validate should report no errors: {errors:?}"
        );
    }

    #[test]
    fn build_uses_name_override() {
        let parent = tempdir().unwrap();
        let dir = parent.path().join("my-custom-name");
        let spec = SkillSpec {
            purpose: "Process PDF files".to_string(),
            name: Some("my-custom-name".to_string()),
            output_dir: Some(dir),
            no_llm: true,
            ..Default::default()
        };
        let result = build_skill(&spec).unwrap();
        assert_eq!(result.properties.name, "my-custom-name");
    }

    #[test]
    fn build_derives_name_from_purpose() {
        let parent = tempdir().unwrap();
        let spec = SkillSpec {
            purpose: "Process PDF files".to_string(),
            output_dir: Some(parent.path().join("processing-pdf-files")),
            no_llm: true,
            ..Default::default()
        };
        let result = build_skill(&spec).unwrap();
        assert!(
            result.properties.name.starts_with("processing"),
            "name should be derived from purpose: {}",
            result.properties.name
        );
    }

    #[test]
    fn build_fails_if_skill_md_exists() {
        let parent = tempdir().unwrap();
        let dir = parent.path().join("existing-skill");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("SKILL.md"), "---\nname: x\n---\n").unwrap();
        let spec = SkillSpec {
            purpose: "Process PDF files".to_string(),
            name: Some("existing-skill".to_string()),
            output_dir: Some(dir),
            no_llm: true,
            ..Default::default()
        };
        let result = build_skill(&spec);
        assert!(result.is_err(), "should fail if SKILL.md already exists");
    }

    #[test]
    fn build_creates_output_dir_if_missing() {
        let parent = tempdir().unwrap();
        let dir = parent.path().join("new-skill-dir");
        assert!(!dir.exists());
        let spec = SkillSpec {
            purpose: "Process PDF files".to_string(),
            name: Some("new-skill-dir".to_string()),
            output_dir: Some(dir.clone()),
            no_llm: true,
            ..Default::default()
        };
        build_skill(&spec).unwrap();
        assert!(dir.exists());
    }

    #[test]
    fn build_result_contains_skill_md_key() {
        let parent = tempdir().unwrap();
        let dir = parent.path().join("processing-pdf-files");
        let spec = SkillSpec {
            purpose: "Process PDF files".to_string(),
            output_dir: Some(dir),
            no_llm: true,
            ..Default::default()
        };
        let result = build_skill(&spec).unwrap();
        assert!(
            result.files.contains_key("SKILL.md"),
            "files should contain 'SKILL.md' key"
        );
    }

    #[test]
    fn build_extra_files_written() {
        let parent = tempdir().unwrap();
        let dir = parent.path().join("extras-skill");
        let mut extra = HashMap::new();
        extra.insert(
            "examples/example.txt".to_string(),
            "example content".to_string(),
        );
        let spec = SkillSpec {
            purpose: "Process files".to_string(),
            name: Some("extras-skill".to_string()),
            output_dir: Some(dir.clone()),
            no_llm: true,
            extra_files: Some(extra),
            ..Default::default()
        };
        let result = build_skill(&spec).unwrap();
        assert!(dir.join("examples/example.txt").exists());
        assert!(result.files.contains_key("examples/example.txt"));
    }

    #[test]
    fn build_spec_with_all_optional_fields() {
        let parent = tempdir().unwrap();
        let dir = parent.path().join("full-skill");
        let spec = SkillSpec {
            purpose: "Process PDF files".to_string(),
            name: Some("full-skill".to_string()),
            tools: Some("Bash, Read".to_string()),
            compatibility: Some("Claude 3.5 and above".to_string()),
            license: Some("MIT".to_string()),
            output_dir: Some(dir),
            no_llm: true,
            extra_files: None,
            template: SkillTemplate::Minimal,
        };
        let result = build_skill(&spec).unwrap();
        assert_eq!(result.properties.name, "full-skill");
        assert_eq!(result.properties.license.as_deref(), Some("MIT"));
        assert_eq!(
            result.properties.compatibility.as_deref(),
            Some("Claude 3.5 and above")
        );
        assert_eq!(
            result.properties.allowed_tools.as_deref(),
            Some("Bash, Read")
        );
    }

    // ── interactive_build tests ──────────────────────────────────────

    #[test]
    fn interactive_build_with_yes_answers() {
        let parent = tempdir().unwrap();
        let dir = parent.path().join("processing-pdf-files");
        let spec = SkillSpec {
            purpose: "Process PDF files and extract text content".to_string(),
            name: Some("processing-pdf-files".to_string()),
            output_dir: Some(dir.clone()),
            no_llm: true,
            ..Default::default()
        };
        // Simulate "y" for all three prompts (name, description, write).
        let mut input = std::io::Cursor::new(b"y\ny\ny\n".to_vec());
        let result = interactive_build(&spec, &mut input).unwrap();
        assert!(dir.join("SKILL.md").exists());
        assert!(!result.properties.name.is_empty());
    }

    #[test]
    fn interactive_build_cancel_at_name() {
        let parent = tempdir().unwrap();
        let dir = parent.path().join("interactive-cancel");
        let spec = SkillSpec {
            purpose: "Process PDF files and extract text content".to_string(),
            output_dir: Some(dir.clone()),
            no_llm: true,
            ..Default::default()
        };
        // Simulate "n" at the name confirmation.
        let mut input = std::io::Cursor::new(b"n\n".to_vec());
        let result = interactive_build(&spec, &mut input);
        assert!(result.is_err());
        assert!(!dir.exists(), "no files should be created on cancel");
    }

    #[test]
    fn interactive_build_unclear_purpose() {
        let parent = tempdir().unwrap();
        let dir = parent.path().join("unclear");
        let spec = SkillSpec {
            purpose: "do stuff".to_string(),
            output_dir: Some(dir),
            no_llm: true,
            ..Default::default()
        };
        let mut input = std::io::Cursor::new(b"".to_vec());
        let result = interactive_build(&spec, &mut input);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("not clear enough"));
    }

    #[test]
    fn non_interactive_build_unchanged() {
        // Verify that the standard build path is unaffected.
        let parent = tempdir().unwrap();
        let dir = parent.path().join("processing-pdf-files");
        let spec = SkillSpec {
            purpose: "Process PDF files".to_string(),
            output_dir: Some(dir.clone()),
            no_llm: true,
            ..Default::default()
        };
        let result = build_skill(&spec).unwrap();
        assert!(dir.join("SKILL.md").exists());
        assert_eq!(result.properties.name, "processing-pdf-files");
    }

    // ── TOCTOU race fix tests ──────────────────────────────────────

    #[test]
    fn build_existing_skill_md_error_contains_path() {
        let parent = tempdir().unwrap();
        let dir = parent.path().join("toctou-build");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("SKILL.md"), "---\nname: x\n---\n").unwrap();
        let spec = SkillSpec {
            purpose: "Process PDF files".to_string(),
            name: Some("toctou-build".to_string()),
            output_dir: Some(dir.clone()),
            no_llm: true,
            ..Default::default()
        };
        let result = build_skill(&spec);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(err, AigentError::AlreadyExists { .. }),
            "expected AlreadyExists variant, got: {err}"
        );
        let msg = err.to_string();
        assert!(
            msg.contains(&dir.join("SKILL.md").display().to_string()),
            "error should contain the file path: {msg}"
        );
    }

    #[test]
    fn init_existing_skill_md_error_contains_path() {
        let parent = tempdir().unwrap();
        let dir = parent.path().join("toctou-init");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("SKILL.md"), "---\nname: x\n---\n").unwrap();
        let result = init_skill(&dir, SkillTemplate::Minimal);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(err, AigentError::AlreadyExists { .. }),
            "expected AlreadyExists variant, got: {err}"
        );
        let msg = err.to_string();
        assert!(
            msg.contains(&dir.join("SKILL.md").display().to_string()),
            "error should contain the file path: {msg}"
        );
    }

    #[test]
    fn build_does_not_overwrite_existing_skill_md() {
        let parent = tempdir().unwrap();
        let dir = parent.path().join("no-overwrite-build");
        std::fs::create_dir_all(&dir).unwrap();
        let original = "---\nname: original\n---\nOriginal content\n";
        std::fs::write(dir.join("SKILL.md"), original).unwrap();
        let spec = SkillSpec {
            purpose: "Process PDF files".to_string(),
            name: Some("no-overwrite-build".to_string()),
            output_dir: Some(dir.clone()),
            no_llm: true,
            ..Default::default()
        };
        let _ = build_skill(&spec);
        let content = std::fs::read_to_string(dir.join("SKILL.md")).unwrap();
        assert_eq!(
            content, original,
            "existing SKILL.md must not be overwritten"
        );
    }

    #[test]
    fn init_does_not_overwrite_existing_skill_md() {
        let parent = tempdir().unwrap();
        let dir = parent.path().join("no-overwrite-init");
        std::fs::create_dir_all(&dir).unwrap();
        let original = "---\nname: original\n---\nOriginal content\n";
        std::fs::write(dir.join("SKILL.md"), original).unwrap();
        let _ = init_skill(&dir, SkillTemplate::Minimal);
        let content = std::fs::read_to_string(dir.join("SKILL.md")).unwrap();
        assert_eq!(
            content, original,
            "existing SKILL.md must not be overwritten"
        );
    }

    #[test]
    fn build_result_has_empty_warnings_on_deterministic() {
        let parent = tempdir().unwrap();
        let dir = parent.path().join("processing-pdf-files");
        let spec = SkillSpec {
            purpose: "Process PDF files".to_string(),
            name: Some("processing-pdf-files".to_string()),
            output_dir: Some(dir),
            no_llm: true,
            ..Default::default()
        };
        let result = build_skill(&spec).unwrap();
        assert!(
            result.warnings.is_empty(),
            "deterministic build should produce no warnings: {:?}",
            result.warnings
        );
    }
}
