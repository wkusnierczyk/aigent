use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::errors::Result;
use crate::models::SkillProperties;

/// User input for skill generation.
#[derive(Debug, Clone)]
pub struct SkillSpec {
    pub purpose: String,
    pub name: Option<String>,
    pub tools: Option<String>,
    pub compatibility: Option<String>,
    pub license: Option<String>,
    pub extra_files: Option<HashMap<String, String>>,
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
pub fn build_skill(_spec: &SkillSpec, _output_dir: &Path) -> Result<BuildResult> {
    todo!()
}

/// Derive a kebab-case skill name from a natural language description.
#[must_use]
pub fn derive_name(_purpose: &str) -> String {
    todo!()
}

/// Evaluate if a purpose description is clear enough for autonomous generation.
#[must_use]
pub fn assess_clarity(_purpose: &str) -> ClarityAssessment {
    todo!()
}
