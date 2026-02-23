use std::path::PathBuf;

// Upgrade rule IDs (local to upgrade — these are not Diagnostic instances).
const U001: &str = "U001";
const U002: &str = "U002";
const U003: &str = "U003";

/// Whether a suggestion is auto-applied by `--apply` or informational only.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SuggestionKind {
    /// Auto-applied with `--apply`.
    Fix,
    /// Informational only — `--apply` does not act on this.
    Info,
}

/// A single upgrade suggestion with a stable rule ID.
struct Suggestion {
    code: &'static str,
    kind: SuggestionKind,
    message: String,
}

impl std::fmt::Display for Suggestion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let tag = match self.kind {
            SuggestionKind::Fix => "fix",
            SuggestionKind::Info => "info",
        };
        write!(f, "[{tag}] {}: {}", self.code, self.message)
    }
}

pub(crate) fn run(
    skill_dir: PathBuf,
    apply: bool,
    dry_run: bool,
    full: bool,
    format: super::Format,
) {
    // --dry-run is a no-op (default is already dry-run). It exists for script
    // readability. Clap's conflicts_with prevents --dry-run --apply.
    let _ = dry_run;

    let dir = super::resolve_skill_dir(&skill_dir);
    match run_upgrade(&dir, apply, full) {
        Ok((suggestions, full_messages, has_full_errors)) => {
            if suggestions.is_empty() && full_messages.is_empty() {
                eprintln!("No upgrade suggestions — skill follows current best practices.");
            } else {
                match format {
                    super::Format::Text => {
                        for msg in &full_messages {
                            eprintln!("{msg}");
                        }
                        for s in &suggestions {
                            eprintln!("{s}");
                        }
                        let fix_count = suggestions
                            .iter()
                            .filter(|s| s.kind == SuggestionKind::Fix)
                            .count();
                        let info_count = suggestions
                            .iter()
                            .filter(|s| s.kind == SuggestionKind::Info)
                            .count();
                        if !apply && fix_count > 0 {
                            eprint!("\nRun with --apply to apply {fix_count} fix(es).");
                            if info_count > 0 {
                                eprint!(" {info_count} informational suggestion(s) shown above.");
                            }
                            eprintln!();
                        } else if !apply && info_count > 0 {
                            eprintln!(
                                "\n{info_count} informational suggestion(s) — no auto-fixes available."
                            );
                        }
                    }
                    super::Format::Json => {
                        let json_suggestions: Vec<serde_json::Value> = suggestions
                            .iter()
                            .map(|s| {
                                serde_json::json!({
                                    "code": s.code,
                                    "kind": match s.kind {
                                        SuggestionKind::Fix => "fix",
                                        SuggestionKind::Info => "info",
                                    },
                                    "message": s.message,
                                })
                            })
                            .collect();
                        let mut json = serde_json::json!({
                            "suggestions": json_suggestions,
                            "applied": apply,
                        });
                        if !full_messages.is_empty() {
                            json["diagnostics"] = serde_json::json!(full_messages);
                        }
                        println!("{}", serde_json::to_string_pretty(&json).unwrap());
                    }
                }
                let has_unapplied_fixes =
                    !apply && suggestions.iter().any(|s| s.kind == SuggestionKind::Fix);
                if has_unapplied_fixes || has_full_errors {
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
/// Checks for missing best-practice fields and returns structured suggestions.
/// With `apply = true`, attempts to add missing optional fields (fix-kind only).
/// With `full = true`, also runs validate + lint first (and applies fixes if
/// `apply` is also true).
///
/// # Invariant
///
/// Upgrade rules MUST NOT modify the markdown body. Body-modifying
/// transformations belong in `format` (style) or require explicit user
/// confirmation beyond `--apply`.
fn run_upgrade(
    dir: &std::path::Path,
    apply: bool,
    full: bool,
) -> std::result::Result<(Vec<Suggestion>, Vec<String>, bool), aigent::AigentError> {
    let mut suggestions = Vec::new();
    let mut full_messages = Vec::new();
    let mut has_full_errors = false;

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
                    full_messages.push(format!(
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
        if !errors.is_empty() {
            has_full_errors = true;
        }
        if !errors.is_empty() || !warnings.is_empty() {
            for d in &errors {
                full_messages.push(format!("[full] error: {d}"));
            }
            for d in &warnings {
                full_messages.push(format!("[full] warning: {d}"));
            }
        }
    }

    let props = aigent::read_properties(dir)?;

    // U001: Check for missing compatibility field.
    if props.compatibility.is_none() {
        suggestions.push(Suggestion {
            code: U001,
            kind: SuggestionKind::Fix,
            message: "Missing 'compatibility' field — recommended for multi-platform skills."
                .to_string(),
        });
    }

    // U002: Check for missing trigger phrase in description.
    let desc_lower = props.description.to_lowercase();
    let has_trigger = aigent::linter::TRIGGER_PHRASES
        .iter()
        .any(|p| desc_lower.contains(p));
    if !has_trigger {
        suggestions.push(Suggestion {
            code: U002,
            kind: SuggestionKind::Info,
            message:
                "Description lacks 'Use when...' trigger phrase — helps Claude activate the skill."
                    .to_string(),
        });
    }

    // U003: Check body length.
    let body = aigent::read_body(dir)?;
    let line_count = body.lines().count();
    if line_count > 500 {
        suggestions.push(Suggestion {
            code: U003,
            kind: SuggestionKind::Info,
            message: format!(
                "Body is {line_count} lines — consider splitting into reference files (recommended < 500)."
            ),
        });
    }

    // Apply upgrades if requested (fix-kind suggestions only).
    if apply && suggestions.iter().any(|s| s.kind == SuggestionKind::Fix) {
        if let Some(path) = aigent::find_skill_md(dir) {
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Ok((raw_map, body)) = aigent::parse_frontmatter(&content) {
                    let front_lines = extract_frontmatter_lines(&content);
                    let mut updated_lines = front_lines.clone();

                    // U001: Append compatibility if missing.
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

    Ok((suggestions, full_messages, has_full_errors))
}
