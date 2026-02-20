pub mod deterministic;
pub mod llm;
pub mod providers;
pub mod template;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::errors::{AigentError, Result};
use crate::models::SkillProperties;
use crate::parser::find_skill_md;
use crate::validator::validate;

use deterministic::{generate_body, generate_description};
use llm::{
    detect_provider, llm_derive_name, llm_generate_body, llm_generate_description, LlmProvider,
};

/// User input for skill generation.
#[derive(Debug, Clone, Default)]
pub struct SkillSpec {
    pub purpose: String,
    pub name: Option<String>,
    pub tools: Option<String>,
    pub compatibility: Option<String>,
    pub license: Option<String>,
    pub extra_files: Option<HashMap<String, String>>,
    /// Output directory override. If `None`, derived from the skill name.
    pub output_dir: Option<PathBuf>,
    /// Force deterministic mode (no LLM) regardless of environment.
    pub no_llm: bool,
}

/// Result of skill generation.
#[derive(Debug)]
pub struct BuildResult {
    pub properties: SkillProperties,
    pub files: HashMap<String, String>,
    pub output_dir: PathBuf,
}

/// Clarity assessment result.
#[derive(Debug)]
pub struct ClarityAssessment {
    pub clear: bool,
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

    // 1. Derive name (LLM with fallback to deterministic).
    let name = if let Some(explicit) = &spec.name {
        explicit.clone()
    } else if let Some(ref prov) = provider {
        match llm_derive_name(prov.as_ref(), &spec.purpose) {
            Ok(n) => n,
            Err(e) => {
                eprintln!("warning: LLM name derivation failed ({e}), using deterministic");
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
                eprintln!("warning: LLM description generation failed ({e}), using deterministic");
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
                eprintln!("warning: LLM body generation failed ({e}), using deterministic");
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

    // 8. Check for existing SKILL.md.
    if output_dir.exists() {
        if let Some(existing) = find_skill_md(&output_dir) {
            return Err(AigentError::Build {
                message: format!("already exists: {}", existing.display()),
            });
        }
    }

    // 9. Create output directory if needed.
    std::fs::create_dir_all(&output_dir)?;

    // 10. Write SKILL.md.
    let skill_md_path = output_dir.join("SKILL.md");
    std::fs::write(&skill_md_path, &content)?;

    // 11. Write extra files if present.
    let mut files = HashMap::new();
    files.insert("SKILL.md".to_string(), content);

    if let Some(ref extra) = spec.extra_files {
        for (rel_path, file_content) in extra {
            let full_path = output_dir.join(rel_path);
            if let Some(parent) = full_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&full_path, file_content)?;
            files.insert(rel_path.clone(), file_content.clone());
        }
    }

    // 12. Validate output.
    let messages = validate(&output_dir);
    let errors: Vec<&str> = messages
        .iter()
        .filter(|m| !m.starts_with("warning: "))
        .map(|m| m.as_str())
        .collect();
    if !errors.is_empty() {
        return Err(AigentError::Build {
            message: format!("generated skill failed validation:\n{}", errors.join("\n")),
        });
    }

    // 13. Return BuildResult.
    Ok(BuildResult {
        properties,
        files,
        output_dir,
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
/// (or skill.md) already exists in the target directory.
pub fn init_skill(dir: &Path) -> Result<PathBuf> {
    // Check for existing SKILL.md.
    if dir.exists() {
        if let Some(existing) = find_skill_md(dir) {
            return Err(AigentError::Build {
                message: format!("already exists: {}", existing.display()),
            });
        }
    }

    // Derive directory name for the template.
    let dir_name = dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("my-skill");

    // Generate template content.
    let content = template::skill_template(dir_name);

    // Create directory if needed.
    std::fs::create_dir_all(dir)?;

    // Write SKILL.md.
    let path = dir.join("SKILL.md");
    std::fs::write(&path, content)?;

    Ok(path)
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
        let _ = init_skill(&dir).unwrap();
        assert!(dir.join("SKILL.md").exists());
    }

    #[test]
    fn init_returns_path_to_created_file() {
        let parent = tempdir().unwrap();
        let dir = parent.path().join("my-skill");
        let path = init_skill(&dir).unwrap();
        assert_eq!(path, dir.join("SKILL.md"));
    }

    #[test]
    fn init_created_file_has_valid_frontmatter() {
        let parent = tempdir().unwrap();
        let dir = parent.path().join("my-skill");
        init_skill(&dir).unwrap();
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
        init_skill(&dir).unwrap();
        let props = crate::read_properties(&dir).unwrap();
        assert_eq!(props.name, "cool-tool");
    }

    #[test]
    fn init_fails_if_skill_md_exists() {
        let parent = tempdir().unwrap();
        let dir = parent.path().join("my-skill");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("SKILL.md"), "---\nname: x\n---\n").unwrap();
        let result = init_skill(&dir);
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
        init_skill(&dir).unwrap();
        assert!(dir.exists());
        assert!(dir.join("SKILL.md").exists());
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
        let messages = crate::validate(&dir);
        let errors: Vec<&str> = messages
            .iter()
            .filter(|m| !m.starts_with("warning: "))
            .map(|s| s.as_str())
            .collect();
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
}
