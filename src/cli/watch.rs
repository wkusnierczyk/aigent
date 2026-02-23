use std::path::PathBuf;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};

use aigent::diagnostics::ValidationTarget;

/// Run watch mode: re-validate on filesystem changes.
pub(crate) fn run_watch_mode(
    skill_dirs: &[PathBuf],
    _format: super::Format,
    target: super::Target,
    structure: bool,
    recursive: bool,
    apply_fixes: bool,
) {
    let (dirs, disc_warnings) = super::resolve_dirs(skill_dirs, recursive);
    for w in &disc_warnings {
        eprintln!("warning: {}: {}", w.path.display(), w.message);
    }
    if dirs.is_empty() {
        eprintln!("No SKILL.md files found.");
        std::process::exit(1);
    }

    let target_val: ValidationTarget = target.into();

    // Run initial validation.
    run_validation_pass(&dirs, target_val, structure, apply_fixes);

    // Set up file watcher.
    let (tx, rx) = mpsc::channel();
    let mut watcher = RecommendedWatcher::new(tx, Config::default()).unwrap_or_else(|e| {
        eprintln!("aigent watch: failed to create watcher: {e}");
        std::process::exit(1);
    });

    // Watch all parent directories of skill dirs.
    let mut watch_paths: Vec<PathBuf> = Vec::new();
    for dir in &dirs {
        let watch_dir = if recursive {
            dir.parent().unwrap_or(dir).to_path_buf()
        } else {
            dir.clone()
        };
        if !watch_paths.contains(&watch_dir) {
            watch_paths.push(watch_dir);
        }
    }
    for path in &watch_paths {
        if let Err(e) = watcher.watch(path, RecursiveMode::Recursive) {
            eprintln!("aigent watch: failed to watch {}: {e}", path.display());
        }
    }

    eprintln!("Watching for changes... (press Ctrl+C to stop)");

    let debounce = Duration::from_millis(500);
    let mut last_run = Instant::now();

    loop {
        match rx.recv() {
            Ok(_event) => {
                // Debounce: skip if we ran too recently.
                if last_run.elapsed() < debounce {
                    // Drain pending events.
                    while rx.try_recv().is_ok() {}
                    continue;
                }

                // Clear terminal.
                eprint!("\x1b[2J\x1b[H");

                // Re-resolve dirs in case new skills appeared.
                let (dirs, disc_warnings) = super::resolve_dirs(skill_dirs, recursive);
                for w in &disc_warnings {
                    eprintln!("warning: {}: {}", w.path.display(), w.message);
                }
                run_validation_pass(&dirs, target_val, structure, apply_fixes);

                last_run = Instant::now();

                // Drain any queued events during validation.
                while rx.try_recv().is_ok() {}
            }
            Err(e) => {
                eprintln!("aigent watch: watcher error: {e}");
                break;
            }
        }
    }
}

/// Run a single validation pass (used by watch mode).
fn run_validation_pass(
    dirs: &[PathBuf],
    target: ValidationTarget,
    structure: bool,
    apply_fixes: bool,
) {
    let mut total_errors = 0;
    let mut total_warnings = 0;

    for dir in dirs {
        let mut diags = aigent::validate_with_target(dir, target);

        if apply_fixes {
            if let Ok(count) = aigent::apply_fixes(dir, &diags) {
                if count > 0 {
                    eprintln!("Applied {count} fix(es) to {}", dir.display());
                    diags = aigent::validate_with_target(dir, target);
                }
            }
        }

        if structure {
            diags.extend(aigent::validate_structure(dir));
        }

        let has_errors = diags.iter().any(|d| d.is_error());
        let has_warnings = diags.iter().any(|d| d.is_warning());

        if has_errors {
            total_errors += 1;
        } else if has_warnings {
            total_warnings += 1;
        }

        if !diags.is_empty() {
            if dirs.len() > 1 {
                eprintln!("{}:", dir.display());
            }
            for d in &diags {
                if dirs.len() > 1 {
                    eprintln!("  {d}");
                } else {
                    eprintln!("{d}");
                }
            }
        }
    }

    let total = dirs.len();
    let ok = total - total_errors - total_warnings;
    eprintln!("\n{total} skills: {ok} ok, {total_errors} errors, {total_warnings} warnings only");
}
