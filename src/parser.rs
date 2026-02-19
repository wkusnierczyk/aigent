use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::errors::Result;
use crate::models::SkillProperties;

/// Locate SKILL.md in a directory (prefer uppercase over lowercase).
#[must_use]
pub fn find_skill_md(dir: &Path) -> Option<PathBuf> {
    let uppercase = dir.join("SKILL.md");
    if uppercase.is_file() {
        return Some(uppercase);
    }
    let lowercase = dir.join("skill.md");
    if lowercase.is_file() {
        return Some(lowercase);
    }
    None
}

/// Extract YAML frontmatter between `---` delimiters.
///
/// Returns `(metadata_map, body_text)`.
pub fn parse_frontmatter(
    _content: &str,
) -> Result<(HashMap<String, serde_yaml_ng::Value>, String)> {
    todo!()
}

/// Full pipeline: find file → read → parse → validate required fields → return properties.
pub fn read_properties(_dir: &Path) -> Result<SkillProperties> {
    todo!()
}
