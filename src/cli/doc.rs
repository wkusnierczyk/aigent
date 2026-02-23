use std::path::PathBuf;

pub(crate) fn run(skill_dirs: Vec<PathBuf>, output: Option<PathBuf>, recursive: bool) {
    let (dirs, disc_warnings) = super::resolve_dirs(&skill_dirs, recursive);
    for w in &disc_warnings {
        eprintln!("warning: {}: {}", w.path.display(), w.message);
    }
    if dirs.is_empty() {
        if recursive {
            eprintln!("No SKILL.md files found under the specified path(s).");
        } else {
            eprintln!("Usage: aigent doc <skill-dir> [<skill-dir>...]");
        }
        std::process::exit(1);
    }

    let dir_refs: Vec<&std::path::Path> = dirs.iter().map(|p| p.as_path()).collect();
    let (entries, warnings) = aigent::collect_skills_verbose(&dir_refs);
    for w in &warnings {
        eprintln!("warning: {}: {}", w.path.display(), w.message);
    }
    let content = format_doc_catalog(&entries);

    if let Some(output_path) = output {
        // Diff-aware output: only write on change.
        let changed = if output_path.exists() {
            let existing = std::fs::read_to_string(&output_path).unwrap_or_default();
            existing != content
        } else {
            true
        };

        if changed {
            if let Some(parent) = output_path.parent() {
                std::fs::create_dir_all(parent).unwrap_or_else(|e| {
                    eprintln!(
                        "aigent doc: failed to create directory {}: {e}",
                        parent.display()
                    );
                    std::process::exit(1);
                });
            }
            std::fs::write(&output_path, &content).unwrap_or_else(|e| {
                eprintln!("aigent doc: failed to write {}: {e}", output_path.display());
                std::process::exit(1);
            });
            eprintln!("Updated {}", output_path.display());
        } else {
            eprintln!("Unchanged {}", output_path.display());
        }
    } else {
        println!("{content}");
    }
}

/// Format a skill catalog as markdown documentation.
///
/// Generates a markdown document listing all skills sorted alphabetically,
/// with name, description, and location. Missing fields are omitted.
fn format_doc_catalog(entries: &[aigent::SkillEntry]) -> String {
    let mut out = String::from("# Skill Catalog\n");

    let mut sorted: Vec<_> = entries.iter().collect();
    sorted.sort_by(|a, b| a.name.cmp(&b.name));

    for entry in sorted {
        out.push_str(&format!("\n## {}\n", entry.name));
        out.push_str(&format!("> {}\n", entry.description));

        // Read full properties for optional fields.
        // entry.location is a file path to SKILL.md; read_properties expects the parent directory.
        let loc_path = std::path::Path::new(&entry.location);
        let skill_dir = loc_path.parent().unwrap_or(loc_path);
        if let Ok(props) = aigent::read_properties(skill_dir) {
            if let Some(compat) = &props.compatibility {
                out.push_str(&format!("\n**Compatibility**: {compat}\n"));
            }
            if let Some(license) = &props.license {
                out.push_str(&format!("**License**: {license}\n"));
            }
        }

        out.push_str(&format!("**Location**: `{}`\n", entry.location));
        out.push_str("\n---\n");
    }

    out
}
