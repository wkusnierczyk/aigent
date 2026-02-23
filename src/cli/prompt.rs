use std::path::PathBuf;

pub(crate) fn run(
    skill_dirs: Vec<PathBuf>,
    format: super::PromptOutputFormat,
    budget: bool,
    output: Option<PathBuf>,
) {
    let dirs: Vec<&std::path::Path> = skill_dirs.iter().map(|p| p.as_path()).collect();
    let prompt_format: aigent::prompt::PromptFormat = format.into();
    let (entries, warnings) = aigent::prompt::collect_skills_verbose(&dirs);
    for w in &warnings {
        eprintln!("warning: {}: {}", w.path.display(), w.message);
    }
    let content = aigent::prompt::format_entries(&entries, prompt_format);

    if let Some(output_path) = output {
        // Diff-aware file output: compare with existing, only write on change.
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
                        "aigent to-prompt: failed to create directory {}: {e}",
                        parent.display()
                    );
                    std::process::exit(1);
                });
            }
            std::fs::write(&output_path, &content).unwrap_or_else(|e| {
                eprintln!(
                    "aigent to-prompt: failed to write {}: {e}",
                    output_path.display()
                );
                std::process::exit(1);
            });
            eprintln!("Updated {}", output_path.display());
            if budget {
                eprint!("{}", aigent::prompt::format_budget(&entries));
            }
            std::process::exit(1);
        } else {
            eprintln!("Unchanged {}", output_path.display());
        }
    } else {
        println!("{content}");
        if budget {
            eprint!("{}", aigent::prompt::format_budget(&entries));
        }
    }
}
