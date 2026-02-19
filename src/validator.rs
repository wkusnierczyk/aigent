use std::collections::HashMap;
use std::path::Path;

/// Validate skill metadata against the Anthropic specification.
///
/// Returns a list of error strings (empty = valid).
#[must_use]
pub fn validate_metadata(
    _metadata: &HashMap<String, serde_yaml_ng::Value>,
    _dir: Option<&Path>,
) -> Vec<String> {
    todo!()
}

/// Validate a skill directory: find SKILL.md, parse, and check all rules.
///
/// Returns a list of error strings (empty = valid).
#[must_use]
pub fn validate(_dir: &Path) -> Vec<String> {
    todo!()
}
