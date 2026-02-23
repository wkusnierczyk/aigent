use std::path::PathBuf;

use aigent::diagnostics::{Diagnostic, ValidationTarget};

pub(crate) fn run(
    skill_dirs: Vec<PathBuf>,
    format: super::Format,
    target: super::Target,
    structure: bool,
    recursive: bool,
    apply_fixes: bool,
    watch: bool,
) {
    // Watch mode: re-run validation on filesystem changes.
    #[cfg(feature = "watch")]
    if watch {
        super::watch::run_watch_mode(
            &skill_dirs,
            format,
            target,
            structure,
            recursive,
            apply_fixes,
        );
        return;
    }
    #[cfg(not(feature = "watch"))]
    if watch {
        eprintln!(
            "Watch mode requires the 'watch' feature. Rebuild with: cargo build --features watch"
        );
        std::process::exit(1);
    }

    // Resolve directories: expand --recursive, resolve file paths.
    let (dirs, disc_warnings) = super::resolve_dirs(&skill_dirs, recursive);
    for w in &disc_warnings {
        eprintln!("warning: {}: {}", w.path.display(), w.message);
    }
    if dirs.is_empty() {
        if recursive {
            eprintln!("No SKILL.md files found under the specified path(s).");
        } else {
            eprintln!("Usage: aigent validate <skill-dir> [<skill-dir>...]");
        }
        std::process::exit(1);
    }

    let mut all_diags: Vec<(PathBuf, Vec<Diagnostic>)> = Vec::new();
    let target_val: ValidationTarget = target.into();

    for dir in &dirs {
        let mut diags = aigent::validate_with_target(dir, target_val);

        // Apply fixes if requested.
        if apply_fixes {
            match aigent::apply_fixes(dir, &diags) {
                Ok(count) if count > 0 => {
                    eprintln!("Applied {count} fix(es) to {}", dir.display());
                    // Re-validate after fixes.
                    diags = aigent::validate_with_target(dir, target_val);
                }
                Ok(_) => {}
                Err(e) => {
                    eprintln!("warning: could not apply fixes to {}: {e}", dir.display());
                }
            }
        }

        // Append structure checks if requested.
        if structure {
            diags.extend(aigent::validate_structure(dir));
        }

        all_diags.push((dir.clone(), diags));
    }

    // Run cross-skill conflict detection for multi-dir validation.
    let conflict_diags = if all_diags.len() > 1 {
        let skill_dirs_refs: Vec<&std::path::Path> = dirs.iter().map(|p| p.as_path()).collect();
        let (entries, coll_warnings) = aigent::collect_skills_verbose(&skill_dirs_refs);
        for w in &coll_warnings {
            eprintln!("warning: {}: {}", w.path.display(), w.message);
        }
        aigent::detect_conflicts(&entries)
    } else {
        vec![]
    };

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
            // Print cross-skill conflict warnings.
            if !conflict_diags.is_empty() {
                eprintln!("\nCross-skill conflicts:");
                for d in &conflict_diags {
                    eprintln!("  {d}");
                }
            }
            // Print summary for multi-dir, or "ok" for clean single-dir.
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
                let total_diags: usize =
                    all_diags.iter().map(|(_, d)| d.len()).sum::<usize>() + conflict_diags.len();
                if total_diags == 0 {
                    eprintln!("ok");
                }
            }
        }
        super::Format::Json => {
            // Always emit consistent array-of-objects format.
            let mut entries: Vec<serde_json::Value> = all_diags
                .iter()
                .map(|(dir, diags)| {
                    serde_json::json!({
                        "path": dir.display().to_string(),
                        "diagnostics": diags,
                    })
                })
                .collect();
            // Append cross-skill conflict diagnostics.
            if !conflict_diags.is_empty() {
                entries.push(serde_json::json!({
                    "path": "<cross-skill>",
                    "diagnostics": conflict_diags,
                }));
            }
            let json = serde_json::to_string_pretty(&entries).unwrap();
            println!("{json}");
        }
    }

    if has_errors {
        std::process::exit(1);
    }
}
