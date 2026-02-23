use std::path::PathBuf;

use aigent::diagnostics::{Diagnostic, ValidationTarget};

pub(crate) fn run(
    skill_dirs: Vec<PathBuf>,
    format: super::Format,
    target: super::Target,
    no_validate: bool,
    structure: bool,
    recursive: bool,
    apply_fixes: bool,
) {
    let (dirs, disc_warnings) = super::resolve_dirs(&skill_dirs, recursive);
    for w in &disc_warnings {
        eprintln!("warning: {}: {}", w.path.display(), w.message);
    }
    if dirs.is_empty() {
        if recursive {
            eprintln!("No SKILL.md files found under the specified path(s).");
        } else {
            eprintln!("Usage: aigent check <skill-dir> [<skill-dir>...]");
        }
        std::process::exit(1);
    }

    let mut all_diags: Vec<(PathBuf, Vec<Diagnostic>)> = Vec::new();
    let target_val: ValidationTarget = target.into();

    for dir in &dirs {
        let mut diags = Vec::new();

        // Run spec conformance checks unless --no-validate.
        if !no_validate {
            diags.extend(aigent::validate_with_target(dir, target_val));

            // Apply fixes if requested.
            if apply_fixes {
                match aigent::apply_fixes(dir, &diags) {
                    Ok(count) if count > 0 => {
                        eprintln!("Applied {count} fix(es) to {}", dir.display());
                        diags = aigent::validate_with_target(dir, target_val);
                    }
                    Ok(_) => {}
                    Err(e) => {
                        eprintln!("warning: could not apply fixes to {}: {e}", dir.display());
                    }
                }
            }
        }

        // Always run semantic lint checks (the core of `check`).
        match aigent::read_properties(dir) {
            Ok(props) => {
                let body = aigent::read_body(dir).unwrap_or_default();
                diags.extend(aigent::lint(&props, &body));
            }
            Err(e) => {
                // Report parse failures as diagnostics rather than silently skipping.
                diags.push(Diagnostic::new(
                    aigent::Severity::Error,
                    "E000",
                    format!("cannot read properties: {e}"),
                ));
            }
        }

        // Append structure checks if requested.
        if structure {
            diags.extend(aigent::validate_structure(dir));
        }

        all_diags.push((dir.clone(), diags));
    }

    let has_errors = all_diags
        .iter()
        .any(|(_, d)| d.iter().any(|d| d.is_error()));

    match format {
        super::Format::Text => {
            let multi = all_diags.len() > 1;
            for (dir, diags) in &all_diags {
                if multi && !diags.is_empty() {
                    eprintln!("{}:", dir.display());
                }
                for d in diags {
                    if multi {
                        eprintln!("  {d}");
                    } else {
                        eprintln!("{d}");
                    }
                }
            }
            if multi {
                let total = all_diags.len();
                let errors = all_diags
                    .iter()
                    .filter(|(_, d)| d.iter().any(|d| d.is_error()))
                    .count();
                let warnings = all_diags
                    .iter()
                    .filter(|(_, d)| {
                        d.iter().any(|d| d.is_warning()) && !d.iter().any(|d| d.is_error())
                    })
                    .count();
                let ok = total - errors - warnings;
                eprintln!("\n{total} skills: {ok} ok, {errors} errors, {warnings} warnings only");
            } else {
                let total_diags: usize = all_diags.iter().map(|(_, d)| d.len()).sum();
                if total_diags == 0 {
                    eprintln!("ok");
                }
            }
        }
        super::Format::Json => {
            let entries: Vec<serde_json::Value> = all_diags
                .iter()
                .map(|(dir, diags)| {
                    serde_json::json!({
                        "path": dir.display().to_string(),
                        "diagnostics": diags,
                    })
                })
                .collect();
            let json = serde_json::to_string_pretty(&entries).unwrap();
            println!("{json}");
        }
    }

    if has_errors {
        std::process::exit(1);
    }
}
