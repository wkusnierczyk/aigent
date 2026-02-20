/// Capitalize the first character of a string.
pub(crate) fn capitalize_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => {
            let upper: String = c.to_uppercase().collect();
            format!("{upper}{}", chars.as_str())
        }
    }
}

/// Convert a kebab-case name to title case.
pub(crate) fn to_title_case(name: &str) -> String {
    name.split('-')
        .map(capitalize_first)
        .collect::<Vec<_>>()
        .join(" ")
}
