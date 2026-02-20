use super::util::to_title_case;

/// Generate a SKILL.md template for `init`.
///
/// The `dir_name` is used as the `name` field (kebab-cased) and the heading
/// (title-cased).
#[must_use]
pub fn skill_template(dir_name: &str) -> String {
    let raw_name = to_kebab_case(dir_name);
    let name = if raw_name.is_empty() {
        "my-skill".to_string()
    } else {
        raw_name
    };
    let title = to_title_case(&name);

    format!(
        "---\n\
         name: {name}\n\
         description: Describe what this skill does and when to use it\n\
         ---\n\
         \n\
         # {title}\n\
         \n\
         ## Quick start\n\
         \n\
         [Add quick start instructions here]\n\
         \n\
         ## Usage\n\
         \n\
         [Add detailed usage instructions here]\n"
    )
}

/// Convert a string to kebab-case: lowercase, replace non-alphanumeric with
/// hyphens, collapse consecutive hyphens, trim leading/trailing hyphens.
fn to_kebab_case(s: &str) -> String {
    let lower = s.to_lowercase();
    let mut result = String::with_capacity(lower.len());
    let mut prev_hyphen = false;

    for c in lower.chars() {
        if c.is_ascii_alphanumeric() {
            result.push(c);
            prev_hyphen = false;
        } else if !prev_hyphen && !result.is_empty() {
            result.push('-');
            prev_hyphen = true;
        }
    }

    result.trim_matches('-').to_string()
}
