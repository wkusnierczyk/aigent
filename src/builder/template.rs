/// Generate a SKILL.md template for `init`.
///
/// The `dir_name` is used as the `name` field (kebab-cased) and the heading
/// (title-cased).
#[must_use]
pub fn skill_template(dir_name: &str) -> String {
    let name = to_kebab_case(dir_name);
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

/// Convert a kebab-case name to title case.
fn to_title_case(name: &str) -> String {
    name.split('-')
        .map(capitalize_first)
        .collect::<Vec<_>>()
        .join(" ")
}

/// Capitalize the first character of a string.
fn capitalize_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => {
            let upper: String = c.to_uppercase().collect();
            format!("{upper}{}", chars.as_str())
        }
    }
}
