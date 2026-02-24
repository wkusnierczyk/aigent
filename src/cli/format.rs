use std::path::PathBuf;

pub(crate) fn run(skill_dirs: Vec<PathBuf>, check: bool, recursive: bool) {
    let (dirs, disc_warnings) = super::resolve_dirs(&skill_dirs, recursive);
    for w in &disc_warnings {
        eprintln!("warning: {}: {}", w.path.display(), w.message);
    }
    if dirs.is_empty() {
        if recursive {
            eprintln!("No SKILL.md files found under the specified path(s).");
        } else {
            eprintln!("Usage: aigent format <skill-dir> [<skill-dir>...]");
        }
        std::process::exit(1);
    }

    let mut any_changed = false;
    let mut any_error = false;
    for dir in &dirs {
        match aigent::format_skill(dir) {
            Ok(result) => {
                if result.changed {
                    any_changed = true;
                    if check {
                        eprintln!("Would reformat: {}", dir.display());
                        let diff = aigent::diff_skill(&result, &dir.display().to_string());
                        eprint!("{diff}");
                    } else {
                        let path = aigent::find_skill_md(dir).unwrap();
                        if !aigent::is_regular_file(&path) {
                            eprintln!(
                                "aigent format: target is no longer a regular file: {}",
                                path.display()
                            );
                            std::process::exit(1);
                        }
                        std::fs::write(&path, &result.content).unwrap_or_else(|e| {
                            eprintln!("aigent format: failed to write {}: {e}", path.display());
                            std::process::exit(1);
                        });
                        eprintln!("Formatted {}", dir.display());
                    }
                }
            }
            Err(e) => {
                eprintln!("aigent format: {}: {e}", dir.display());
                any_error = true;
            }
        }
    }

    // Print "ok" for single-dir text mode with no changes and no errors.
    if !any_error && !any_changed && dirs.len() == 1 {
        eprintln!("ok");
    }

    if any_error || (check && any_changed) {
        std::process::exit(1);
    }
}
