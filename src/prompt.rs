use std::path::Path;

/// Escape XML special characters: `& < > "`.
pub fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Generate `<available_skills>` XML block from skill directories.
pub fn to_prompt(_dirs: &[&Path]) -> String {
    todo!()
}
