use std::path::PathBuf;

pub(crate) fn run(
    skill_dirs: Vec<PathBuf>,
    format: super::Format,
    recursive: bool,
    generate: bool,
) {
    let (dirs, disc_warnings) = super::resolve_dirs(&skill_dirs, recursive);
    for w in &disc_warnings {
        eprintln!("warning: {}: {}", w.path.display(), w.message);
    }
    if dirs.is_empty() {
        if recursive {
            eprintln!("No SKILL.md files found under the specified path(s).");
        } else {
            eprintln!("Usage: aigent test <skill-dir> [<skill-dir>...]");
        }
        std::process::exit(1);
    }

    if generate {
        let mut any_error = false;
        for dir in &dirs {
            match aigent::generate_fixture(dir) {
                Ok(yaml) => {
                    let fixture_path = dir.join("tests.yml");
                    if fixture_path.exists() {
                        eprintln!("Skipping {} — tests.yml already exists", dir.display());
                    } else {
                        std::fs::write(&fixture_path, &yaml).unwrap_or_else(|e| {
                            eprintln!(
                                "aigent test: failed to write {}: {e}",
                                fixture_path.display()
                            );
                            std::process::exit(1);
                        });
                        eprintln!("Generated {}", fixture_path.display());
                    }
                }
                Err(e) => {
                    eprintln!("aigent test: {}: {e}", dir.display());
                    any_error = true;
                }
            }
        }
        if any_error {
            std::process::exit(1);
        }
        return;
    }

    let mut total_passed = 0;
    let mut total_failed = 0;
    let mut any_error = false;

    for dir in &dirs {
        match aigent::run_test_suite(dir) {
            Ok(result) => {
                match format {
                    super::Format::Text => {
                        if dirs.len() > 1 {
                            eprintln!("{}:", dir.display());
                        }
                        eprint!("{}", aigent::format_test_suite(&result));
                    }
                    super::Format::Json => {
                        let json = serde_json::to_string_pretty(&result).unwrap();
                        println!("{json}");
                    }
                }
                total_passed += result.passed;
                total_failed += result.failed;
            }
            Err(e) => {
                eprintln!("aigent test: {}: {e}", dir.display());
                any_error = true;
            }
        }
    }

    if dirs.len() > 1 {
        eprintln!(
            "\nTotal: {total_passed} passed, {total_failed} failed, {} total",
            total_passed + total_failed
        );
    }

    if total_failed > 0 || any_error {
        std::process::exit(1);
    }
}
