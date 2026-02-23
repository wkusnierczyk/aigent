use std::path::PathBuf;

pub(crate) fn run(skill_dir: PathBuf, apply: bool, full: bool, format: super::Format) {
    let dir = super::resolve_skill_dir(&skill_dir);
    match run_upgrade(&dir, apply, full) {
        Ok(suggestions) => {
            if suggestions.is_empty() {
                eprintln!("No upgrade suggestions — skill follows current best practices.");
            } else {
                match format {
                    super::Format::Text => {
                        for s in &suggestions {
                            eprintln!("{s}");
                        }
                        if !apply {
                            eprintln!(
                                "\nRun with --apply to apply {} suggestion(s).",
                                suggestions.len()
                            );
                        }
                    }
                    super::Format::Json => {
                        let json = serde_json::json!({
                            "suggestions": suggestions,
                            "applied": apply,
                        });
                        println!("{}", serde_json::to_string_pretty(&json).unwrap());
                    }
                }
                if !apply {
                    std::process::exit(1);
                }
            }
        }
        Err(e) => {
            eprintln!("aigent upgrade: {e}");
            std::process::exit(1);
        }
    }
}

/// Extract frontmatter lines from SKILL.md content (between the `---` delimiters).
///
/// Returns the lines without the delimiters.
fn extract_frontmatter_lines(content: &str) -> Vec<String> {
    content
        .lines()
        .skip(1) // skip opening ---
        .take_while(|l| l.trim_end() != "---")
        .map(|l| l.to_string())
        .collect()
}

/// Run upgrade analysis on a skill directory.
///
/// Checks for missing best-practice fields and returns a list of human-readable
/// suggestions. With `apply = true`, attempts to add missing optional fields.
/// With `full = true`, also runs validate + lint first (and applies fixes if
/// `apply` is also true).
fn run_upgrade(
    dir: &std::path::Path,
    apply: bool,
    full: bool,
) -> std::result::Result<Vec<String>, aigent::AigentError> {
    let mut suggestions = Vec::new();

    // Full mode: run validate + lint before upgrade analysis.
    if full {
        let mut diags = aigent::validate(dir);
        if let Ok(props) = aigent::read_properties(dir) {
            let body = aigent::read_body(dir).unwrap_or_default();
            diags.extend(aigent::lint(&props, &body));
        }

        if apply {
            let fixable: Vec<_> = diags
                .iter()
                .filter(|d| d.suggestion.is_some())
                .cloned()
                .collect();
            if !fixable.is_empty() {
                let fix_count = aigent::apply_fixes(dir, &fixable)?;
                if fix_count > 0 {
                    suggestions.push(format!(
                        "[full] Applied {fix_count} validation/lint fix(es)"
                    ));
                }
                // Re-run diagnostics after fixes to get fresh state.
                diags = aigent::validate(dir);
                if let Ok(props) = aigent::read_properties(dir) {
                    let body = aigent::read_body(dir).unwrap_or_default();
                    diags.extend(aigent::lint(&props, &body));
                }
            }
        }

        let errors: Vec<_> = diags.iter().filter(|d| d.is_error()).collect();
        let warnings: Vec<_> = diags.iter().filter(|d| d.is_warning()).collect();
        if !errors.is_empty() || !warnings.is_empty() {
            for d in &errors {
                suggestions.push(format!("[full] error: {d}"));
            }
            for d in &warnings {
                suggestions.push(format!("[full] warning: {d}"));
            }
        }
    }

    let props = aigent::read_properties(dir)?;

    // Check for missing compatibility field.
    if props.compatibility.is_none() {
        suggestions.push(
            "Missing 'compatibility' field — recommended for multi-platform skills.".to_string(),
        );
    }

    // Check for missing trigger phrase in description.
    let desc_lower = props.description.to_lowercase();
    if !desc_lower.contains("use when") && !desc_lower.contains("use this when") {
        suggestions.push(
            "Description lacks 'Use when...' trigger phrase — helps Claude activate the skill."
                .to_string(),
        );
    }

    // Check body length.
    let body = aigent::read_body(dir)?;
    let line_count = body.lines().count();
    if line_count > 500 {
        suggestions.push(format!(
            "Body is {line_count} lines — consider splitting into reference files (recommended < 500)."
        ));
    }

    // Apply upgrades if requested.
    if apply && !suggestions.is_empty() {
        if let Some(path) = aigent::find_skill_md(dir) {
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Ok((raw_map, body)) = aigent::parse_frontmatter(&content) {
                    let front_lines = extract_frontmatter_lines(&content);
                    let mut updated_lines = front_lines.clone();

                    // Append compatibility if missing.
                    if props.compatibility.is_none() && !raw_map.contains_key("compatibility") {
                        updated_lines.push("compatibility: claude-code".to_string());
                    }

                    let updated_yaml = updated_lines.join("\n");
                    let new_content = format!("---\n{updated_yaml}\n---\n{body}");
                    if new_content != content {
                        std::fs::write(&path, &new_content)?;
                        eprintln!("Applied upgrades to {}", path.display());
                    }
                }
            }
        }
    }

    Ok(suggestions)
}
